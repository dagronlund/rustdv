# Chapter 29: The Factory Problem, Solved by Closures and Generics

The UVM factory answers a question every test-writer eventually asks: *how do I change what the testbench does without editing the testbench?* Testbench 4.0 needed two environments because the tester was hardcoded into each; the factory's promise is one environment whose parts a test can swap from outside. The promise is kept in rustdv — this chapter and the next are the keeping — but the machinery goes the way of the ConfigDB's: the global registry, the override tables, and `create()` dissolve, and what delivers the capability is a language feature you have held since Chapter 12: **constructors are values, and closures carry them.**

> **In Python we...** instantiated components through `TinyComponent.create("tc", self)` instead of calling the class, which routed construction through the UVM factory; then `set_type_override_by_type(TinyComponent, MediumComponent)` made every subsequent `create()` of a Tiny produce a Medium. A metaclass had registered every component class by name at import time, and the factory resolved overrides — including chains of them — at each creation.

## The component, created directly

The Python book's lab animal, ported:

```rust
// Figure 1: A tiny example component

pub struct TinyComponent {
    logger: Logger,
}

impl TinyComponent {
    pub fn new() -> TinyComponent {
        TinyComponent { logger: Logger::new("uvm_test_top.tc") }
    }
}

impl Component for TinyComponent {
    fn start(&mut self, ctx: &mut RunCtx) {
        let obj = ctx.raise_objection("tiny");
        let logger = self.logger.clone();
        spawn_named(
            async move {
                logger.info("I'm so tiny!");
                drop(obj);
            },
            "tc.run",
        );
    }
}
```

```rust
// Figure 2: Instantiating the component by calling new() directly

#[derive(rustdv::Component)]
pub struct TinyEnv {
    #[component(child)]
    tc: TinyComponent,
}
```

```text
# Figure 3: The expected log message
--
      0.00ns INFO     [uvm_test_top.tc]: I'm so tiny!
```

Direct construction, and note precisely what the Python book noted: the component's *type is hardcoded* — in our case doubly so, in the field's type and in the constructor call. No test can change what `TinyEnv` builds without editing `TinyEnv`. In pyuvm the remedy began by swapping `TinyComponent("tc", self)` for `TinyComponent.create("tc", self)` — same result, but construction now routed through a global registry that overrides could redirect. rustdv has no `create()`, because it has something Python and SystemVerilog lack: constructors you can *pass around*.

## The variation point

Here is the whole trick. If `TinyEnv` should be overridable, its config carries the constructor:

```rust
// Figure 4: A designed variation point: the maker closure

/// Anything that can stand where a TinyComponent stood.
pub trait TinyLike: ComponentNode {}
impl<T: ComponentNode> TinyLike for T {}

pub struct FlexEnvConfig {
    /// The variation point, explicit in the type. A test overrides the
    /// component by assigning a different closure.
    pub make_tc: Box<dyn FnOnce() -> Box<dyn TinyLike>>,
}

impl Default for FlexEnvConfig {
    fn default() -> FlexEnvConfig {
        FlexEnvConfig { make_tc: Box::new(|| Box::new(TinyComponent::new())) }
    }
}

pub struct FlexEnv {
    tc: Box<dyn TinyLike>,
}

impl FlexEnv {
    pub fn new(config: FlexEnvConfig) -> FlexEnv {
        FlexEnv { tc: (config.make_tc)() }
    }
}
```

Take it a piece at a time, because every piece is a Chapter 10–13 alumnus doing methodology work. `TinyLike` is the *contract of the slot*: what must be true of anything standing in this position — here, just "be a component" (real slots say more; testbench 6.0's driver slot demands the driver interface). `make_tc` is a boxed `FnOnce` closure returning a boxed trait object: a constructor, stored in a struct field, called exactly once — `(config.make_tc)()` — where pyuvm called `create()`. The `Default` impl is the factory's "no override registered" case: by default, the maker builds the original. And the child field became `Box<dyn TinyLike>` — dynamic dispatch, deliberately, because an overridable slot *is* the place where you don't know the concrete type; this is the trait-objects-versus-generics line from Chapter 10, drawn exactly where the design doc drew it.

```text
# Figure 5: The default maker builds the original component
--
      0.00ns INFO     [uvm_test_top.tc]: I'm so tiny!
```

## The override

```rust
// Figure 6: The component a test will swap in

pub struct MediumComponent {
    logger: Logger,
}
// ...identical shape; its start() logs "I'm medium size."
```

```rust
// Figure 7: The override is an assignment, visible in the test

#[rustdv::test]
async fn medium_test(_ctx: TestCtx) -> Result<(), TestError> {
    let config = FlexEnvConfig {
        make_tc: Box::new(|| Box::new(MediumComponent::new())),
    };
    let mut env = FlexEnv::new(config);
    // ...lifecycle as always
```

```text
# Figure 8: The environment builds the substitute
--
      0.00ns INFO     [uvm_test_top.tc]: I'm medium size.
```

That is `set_type_override_by_type(TinyComponent, MediumComponent)`: three visible lines in the test, no strings, no registry, checked end to end. If `MediumComponent` doesn't satisfy the slot's contract, the closure doesn't compile — where pyuvm discovered an unsuitable override by runtime failure inside `create()`. If two tests want different substitutes, each builds its own config; there is no ambient global registry for one test's override to leak through into the next test's run, a bug class the SystemVerilog UVM knows well.

And the resolution story deserves its sentence of appreciation. pyuvm's `find_override` was a recursive resolver walking override *chains* — Tiny→Medium, Medium→Large, with loop detection, because overrides of overrides accumulate in a global table. The rustdv equivalent is: the field holds one closure. Assignment is visible and final; there is no chain to chase, no loop to detect, and "what will this env build?" is answered by reading the config at the construction site.

## The honest ledger

The Python book's factory chapter closed with `uvm_factory().debug_level` printing the registry's contents. There is no registry to print, which is the cue to write down what this design deliberately does *not* do — the same ledger the rustdv design documents keep:

```text
# Figure 9: The factory, dispositioned

pyuvm capability                        rustdv disposition
----------------                        ------------------
create() + type override                maker closure in the config (this chapter)
per-test behavior swap                  a different sequence, or a different maker (Ch. 30)
create_component_by_name("...")         not ported — strings only where strings help
instance-path overrides ("*.agent2.*")  honestly lost: no ambient registry to pattern-match
override chains + loop detection        nothing to chase: one closure per slot
factory debug printing                  read the config; #[derive(Debug)] prints it
string registry                         survives in exactly one place: test discovery (Ch. 21)
```

Two rows need a word. *Create-by-name* — conjuring a component from a string — was mechanism in service of the override table; with the table gone, a string-to-constructor map is something you can build in an afternoon if a flow genuinely needs it (a `HashMap<&str, Maker>` is not a framework). *Instance-path overrides* are the real loss, and the book will not pretend otherwise: in SV-UVM you can override every driver under `*.agent2` in an env whose source you cannot edit. With no global registry there is nothing to pattern-match against. The exchange is that an env's possible behaviors are exactly what its config type declares — no action at a distance — and the mitigation is a design convention this book teaches from here on: **envs intended for reuse expose maker fields in their configs.** An env without designed variation points can only be forked; rustdv is honestly weaker than SV-UVM here, and honestly clearer about what a given testbench can do.

The dominant use of the factory in the Python book, though, was none of these exotica. It was: *the max-ops test overrides the random tester.* And for that, the next chapter shows, you often need even less machinery than this chapter built — because the thing tests most want to vary is the sequence, and sequences are just values you start.

## Summary

The factory's methodology — tests changing testbench behavior without editing the env — ported whole; its mechanism compressed into the language. A variation point is a config field holding a maker closure (`Box<dyn FnOnce() -> Box<dyn SlotContract>>`), defaulted to the original component, overridden by assignment in the test that wants a substitute; the slot's requirements are a trait bound, checked at compile time, and resolution is reading the field. The registry survives only where strings genuinely help — test selection by name — and the ledger records the deliberate losses, instance-path overrides chief among them, traded for envs whose capabilities are declared in their types.

Testbench 5.0 now does what testbench 4.0's twin environments existed to avoid: one environment, two tests, with the difference between them carried entirely in what the tests pass in.
