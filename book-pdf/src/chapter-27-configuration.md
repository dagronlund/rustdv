# Chapter 27: Configuration: The ConfigDB Problem, Solved by Types

Chapter 25's environment took its BFM as a constructor argument and thought nothing of it. This chapter is about why that unremarkable line is the answer to one of the UVM's most remarkable subsystems. The problem is real and permanent: *tests must parameterize components buried deep in a hierarchy the test didn't write* — messages, modes, shared resources. The UVM's answer was the config database — `uvm_config_db#(T)` in SystemVerilog, `ConfigDB()` in pyuvm — a runtime store of path-keyed values. rustdv's answer is a config struct. This chapter replays the config database's classic scenarios and watches each one land.

> **In the UVM...** we stored values with `uvm_config_db#(string)::set(this, "env.loga", "MSG", ...)` or `ConfigDB().set(self, "env.loga", "MSG", "LOG A msg")` — context object plus path string plus key — and components retrieved them with `get()`. Wildcards (`"env.t*"`) configured groups; a null context made globals; "longest path wins" resolved overlaps; and a parent and child setting the same path was settled by a precedence rule you had to memorize.

## A component that needs configuration

The lab animal is a `MsgLogger`: a component whose one behavior — the message it logs — comes from outside.

```rust
// Figure 1: A component that needs configuration

pub struct MsgLogger {
    logger: Logger,
    msg: String, // the configured value: a field, not a database lookup
}

impl MsgLogger {
    pub fn new(path: &str, msg: String) -> MsgLogger {
        MsgLogger { logger: Logger::new(path), msg }
    }
}

impl Component for MsgLogger {
    fn start(&mut self, ctx: &mut RunCtx) {
        let obj = ctx.raise_objection("logging a message");
        let (logger, msg) = (self.logger.clone(), self.msg.clone());
        spawn_named(
            async move {
                logger.info(&msg);
                drop(obj);
            },
            "msg_logger.run",
        );
    }
}
```

The pyuvm `MsgLogger` called `ConfigDB().get(self, "", "MSG")` in its `build_phase` and hoped: hoped someone had `set()` a value, hoped the path matched, hoped the value was a string. The rustdv `MsgLogger` *has a `msg` field*. There is no lookup, so there is nothing to hope about — a `MsgLogger` without a message does not construct, and the compiler makes that argument at every call site.

## The config struct

Where does the value come from? From a struct the test builds, whose shape mirrors the hierarchy it configures:

```rust
// Figure 2: The config struct mirrors the hierarchy

pub struct MsgEnvConfig {
    pub loga_msg: String,
    pub logb_msg: String,
}

#[derive(rustdv::Component)]
pub struct MsgEnv {
    #[component(child)]
    loga: MsgLogger,
    #[component(child)]
    logb: MsgLogger,
}

impl MsgEnv {
    pub fn new(config: MsgEnvConfig) -> MsgEnv {
        MsgEnv {
            loga: MsgLogger::new("env.loga", config.loga_msg),
            logb: MsgLogger::new("env.logb", config.logb_msg),
        }
    }
}
```

```rust
// Figure 3: The test builds the config and hands it over

#[rustdv::test]
async fn msg_test(_ctx: TestCtx) -> Result<(), TestError> {
    let config = MsgEnvConfig {
        loga_msg: "LOG A msg".to_string(),
        logb_msg: "LOG B msg".to_string(),
    };
    let mut env = MsgEnv::new(config);

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}
```

```text
# Figure 4: The loga and logb components have different things to say
--
      0.00ns INFO     [env.loga]: LOG A msg
      0.00ns INFO     [env.logb]: LOG B msg
```

Same output as ever, and compare what stood behind it. The config database: two `set()` calls with paths assembled from a context object and a string, matched at build time against `get()` calls by a path-glob algorithm, any link of which could silently fail. rustdv: a struct with two fields, passed to a constructor that distributes them. The "path" is the nesting — `MsgEnvConfig` configures `MsgEnv`, whose constructor routes each field to its child. Wrong type in a field: compile error. Missing field: compile error (`E0063`, naming the field). Typo'd field name: compile error. pyuvm's `UVMConfigItemNotFound` and the silent zero of a failed SV `get()` have no rustdv equivalent, because *not found* is not a state a struct field can be in.

## Wildcards become visible sharing

pyuvm's wildcard — `ConfigDB().set(self, "env.t*", "MSG", "TALK TALK")` — configured `talka` and `talkb` at a stroke, by pattern-matching paths at runtime. The rustdv translation is almost embarrassingly direct: one field, used twice.

```rust
// Figure 5: "Wildcards" become one field used twice

pub struct MultiMsgConfig {
    pub loga_msg: String,
    pub logb_msg: String,
    pub talk_msg: String, // talka AND talkb: sharing is visible in new()
}
```

```rust
impl MultiMsgEnv {
    pub fn new(config: MultiMsgConfig) -> MultiMsgEnv {
        MultiMsgEnv {
            talka: MsgLogger::new("env.talka", config.talk_msg.clone()),
            talkb: MsgLogger::new("env.talkb", config.talk_msg),
            loga: MsgLogger::new("env.loga", config.loga_msg),
            logb: MsgLogger::new("env.logb", config.logb_msg),
        }
    }
}
```

```text
# Figure 6: The "talk" components get the same message
--
      0.00ns INFO     [env.talka]: TALK TALK
      0.00ns INFO     [env.talkb]: TALK TALK
      0.00ns INFO     [env.loga]: LOG A msg
      0.00ns INFO     [env.logb]: LOG B msg
```

The `clone()`/move pair even documents the fan-out: `talka` gets a copy, `talkb` gets the original, and if you add `talkc` without cloning, the compiler stops you at the use-after-move. Where the glob pattern acted at a distance — who *else* matches `env.t*`? grep and pray — the loop or repeated field acts exactly where you can see it. (For configuring a `Vec` of twenty drivers, the same idea is a `for` loop or `vec![config.msg.clone(); 20]`; visibility scales.)

## Global data becomes a Default

pyuvm's `set(None, "*", "MSG", "GLOBAL")` planted a value every component could see, serving as a fallback when no specific path matched, with "longest path wins" arbitrating. Rust has a standard trait for "the value you get when nobody speaks up":

```rust
// Figure 7: "Global data" becomes a Default implementation

impl Default for GlobalConfig {
    fn default() -> GlobalConfig {
        GlobalConfig {
            loga_msg: "GLOBAL".to_string(),
            logb_msg: "GLOBAL".to_string(),
            talk_msg: "GLOBAL".to_string(),
            gtalk_msg: "GLOBAL".to_string(),
        }
    }
}
```

```rust
// Figure 8: Overriding some fields, defaulting the rest

    let config = GlobalConfig {
        loga_msg: "LOG A msg".to_string(),
        logb_msg: "LOG B msg".to_string(),
        talk_msg: "TALK TALK".to_string(),
        ..GlobalConfig::default() // gtalk_msg falls back to "GLOBAL"
    };
```

```text
# Figure 9: The default is matched only where nothing overrides it
--
      0.00ns INFO     [env.gtalk]: GLOBAL
      0.00ns INFO     [env.talka]: TALK TALK
      0.00ns INFO     [env.loga]: LOG A msg
      0.00ns INFO     [env.logb]: LOG B msg
```

The `..Default::default()` syntax — *struct update*, Rust calls it — is "longest path wins" with the resolution done by the reader's eyes: explicit fields win, everything else defaults. There is no algorithm because there is no ambiguity; each field is set in exactly one visible place.

Which brings us to the config database's most instructive scenario. A parent sets `env.loga`'s message; the env itself sets `loga`'s message; both paths resolve to `uvm_test_top.env.loga`, and the UVM applies its rule: *the parent wins* — a precedence you memorize, and Chapter 28's debugging chapter exists substantially because people don't. Try to write that conflict in rustdv:

```rust
// Figure 10: The parent/child conflict has nowhere to live

    let config = MsgEnvConfig {
        loga_msg: "PARENT RULES!".to_string(),
        loga_msg: "CHILD RULES!".to_string(),
    };
```

```text
--
error[E0062]: field `loga_msg` specified more than once
  --> src/main.rs:13:9
   |
12 |         loga_msg: "PARENT RULES!".to_string(),
   |         ------------------------------------- first use of `loga_msg`
13 |         loga_msg: "CHILD RULES!".to_string(),
   |         ^^^^^^^^ used more than once
```

The conflict is not resolved by a precedence rule; it is rejected as a contradiction. A field has one value, set at one construction site, and two claimants collide in the compiler rather than in a debugging session at seed 8,441.

## Sharing real resources

Strings made the mechanics visible; the case that matters is sharing something live — a handle to the TinyAluBfm, the job SystemVerilog's config database spends most of its life doing for virtual interfaces. You have been reading the rustdv answer since Chapter 25: an `Rc<TinyAluBfm>` field in the config, cloned to each component that needs the one BFM. The form testbench 6.0 adopts wholesale:

```rust
// Figure 11: The shape of a real config tree (testbench 6.0's, previewed)

pub struct AluEnvConfig {
    pub bfm: Rc<TinyAluBfm>,     // shared resource: an Rc field
    pub is_active: Active,       // an enum — not a string-keyed int
    pub enable_coverage: bool,
}
```

Nested hierarchies nest their configs (`AluEnvConfig` holding an `AluAgentConfig`, mirroring the ownership tree), and a test configures a component three levels down by building a struct three levels deep — every level named, every field typed, the whole tree readable top to bottom in the test that built it.

What of `wait_modified`, the block-until-someone-changes-my-config? It has no direct port; the pattern it serves — a component reacting to a mid-run parameter change — is a `sim::Event` (or a channel) carried *in* the config struct, which says what it means: this value is a signal, not a setting. Nobody ever used `wait_modified` in anger, and neither will we.

## Summary

The ConfigDB's problem — tests parameterizing deeply buried components — survives untouched; its mechanism dissolved into the language. Values travel in config structs whose nesting mirrors the ownership tree: fields instead of path-plus-key lookups, so missing, mistyped, and wrongly-typed configuration are compile errors with field names in them. Wildcards became a field used in several visible places; global defaults became `Default` plus struct-update syntax; parent/child conflicts became `E0062`, a contradiction the compiler refuses to arbitrate. Shared resources ride as `Rc` fields, and the enum-not-int, bool-not-string typing of figure 11 is the config style every remaining testbench version uses.

pyuvm needed a whole second chapter to teach *debugging* the ConfigDB. Chapter 28 walks the same crime scenes — wrong path, wrong phase, shadowed precedence, wrong type — and files the report on where each body went.
