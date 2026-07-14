# Chapter 28: Configuration Debugging: What the Compiler Now Does for You

The Python book needed a whole chapter to debug the ConfigDB, and it earned its keep: the database's failure modes were quiet, remote, and ranked among pyuvm's most common support questions. This chapter walks the same crime scenes with the rustdv config-struct design from Chapter 27 and files a report on each: where the bug went, what it looks like now, and — the honest part — what debugging *remains* when the plumbing can no longer fail.

> **In Python we...** learned the ConfigDB's classic mistakes one painful demonstration at a time: a typo'd path that matched nothing and left the component running on defaults; a `get()` called before the corresponding `set()`; a wildcard shadowing a specific path (or the reverse, depending on lengths); and a value stored with the wrong type, discovered as an explosion at the point of use, deep in the run.

## The wrong path

The classic: you configure `"env.covergae"` and nothing complains — glob matching has no opinion about paths that match nothing. The component quietly uses its default, and you discover the typo three weeks later when coverage comes back empty. There is no path string in rustdv; the "path" is a chain of field names, and a wrong name is:

```rust
// Figure 1: The "wrong path" mistake is now a wrong name

struct AluEnvConfig {
    enable_coverage: bool,
}

fn main() {
    // In pyuvm: ConfigDB().set(self, "env.covergae", "ENABLE", True)
    // matched nothing, silently, and the component used its default.
    let config = AluEnvConfig { enable_covergae: true };
    let _ = config;
}
```

```text
--
error[E0560]: struct `AluEnvConfig` has no field named `enable_covergae`
  --> src/main.rs:11:33
   |
11 |     let config = AluEnvConfig { enable_covergae: true };
   |                                 ^^^^^^^^^^^^^^^ unknown field
   |
help: a field with a similar name exists
   |
11 +     let config = AluEnvConfig { enable_coverage: true };
```

Read the bottom of that message again: the compiler *found the field you meant* and typed out the corrected line. The bug class that cost afternoons now costs the time it takes to accept a suggestion.

## The wrong type

Second classic: store `"true"` — the string — where the component expects a boolean. The ConfigDB, whose values were `Any`, stored it cheerfully; the explosion came at the point of use, mid-simulation, with a stack trace pointing at the innocent component rather than the guilty `set()`.

```rust
// Figure 2: The "wrong type" mistake never reaches runtime

fn main() {
    // In pyuvm: set(..., "ENABLE", "true") stored a *string*; the component
    // exploded (or worse, didn't) at the point of use, mid-simulation.
    let config = AluEnvConfig { enable_coverage: "true" };
    let _ = config;
}
```

```text
--
error[E0308]: mismatched types
  --> src/main.rs:11:50
   |
11 |     let config = AluEnvConfig { enable_coverage: "true" };
   |                                                  ^^^^^^ expected `bool`, found `&str`
```

Note where the error points: at the *setter*, not the user — the guilty line, not the innocent one. And note the "or worse, didn't" in the comment: in Python, a non-empty string is truthy, so `"false"` would have *enabled* coverage and passed every runtime check. That bug — configuration accepted, wrong, and invisible — is the one this design was bought to kill.

## The wrong phase, and the shadowed precedence

Two crime scenes need no figure, because the premises were demolished. **Set-after-get** — calling `ConfigDB().get()` in a build phase that ran before the test's `set()` — was an *ordering* bug between two runtime events. In rustdv there are no two events: the config struct is built, whole, before the env constructor runs, and a component cannot ask for a value before it exists because the value's existence is a precondition of the component's. The borrow checker will not even let you construct the env from a config you haven't finished building.

**Shadowed precedence** — the wildcard-versus-specific-path puzzles, the parent-beats-child rule, the whole "longest path wins, except in the build phase, where depth wins" apparatus — dissolved with the database. Chapter 27's figure 10 showed the residue: two values for one field is `E0062`, a contradiction, not a contest. There is exactly one value, constructed in test code you can read top to bottom, and *reading the test* is the entire resolution algorithm.

For completeness, the third member of the demolished set:

```rust
// Figure 3: Forgetting to configure is a missing field

fn main() {
    // In pyuvm: the get() raised UVMConfigItemNotFound at build time —
    // if you were lucky. Here the test simply doesn't build:
    let config = AluEnvConfig { enable_coverage: true };
    let _ = config;
}
```

```text
--
error[E0063]: missing field `n_ops` in initializer of `AluEnvConfig`
```

`UVMConfigItemNotFound` was the *lucky* outcome in pyuvm — it meant the miss happened where a lookup could notice. The unlucky outcome was a default silently used. Both are now `E0063`, at compile time, naming the field.

## The scorecard

```text
# Figure 4: The ConfigDB failure modes, dispositioned

pyuvm mistake                     discovered            rustdv disposition
-------------                     ----------            ------------------
typo'd path                       silently, much later  E0560, with the fix suggested
wrong value type                  at point of use       E0308, at the setter
get() before set()                build-time raise      unrepresentable (one-pass build)
wildcard/precedence shadowing     seed-dependent        unrepresentable (one value per field)
parent/child conflict             rule you memorized    E0062, a contradiction
forgot to set at all              raise — or a default  E0063, naming the field
```

## The debugging that remains

Now the honest half of the chapter, because "the compiler catches it" is true only of configuration *plumbing*. What remains is configuration *values*: the config that builds fine, flows fine, and is wrong — twenty ops where you meant two hundred, a passive agent where the test needed an active one. The compiler has no opinion about your intentions.

Two habits cover most of it. First, print the tree. pyuvm could dump the ConfigDB's contents on demand; the rustdv equivalent is one derive away, and better formatted:

```rust
// Figure 5: The debugging that remains — print the config tree

#[derive(Debug)]
struct AluAgentConfig {
    is_active: Active,
    n_ops: u32,
}

#[derive(Debug)]
struct AluEnvConfig {
    agent: AluAgentConfig,
    enable_coverage: bool,
}

fn main() {
    let config = AluEnvConfig {
        agent: AluAgentConfig { is_active: Active::Active, n_ops: 20 },
        enable_coverage: true,
    };
    // pyuvm: ConfigDB() printed its store on demand. rustdv: one derive.
    println!("{config:#?}");
}
```

```text
--
AluEnvConfig {
    agent: AluAgentConfig {
        is_active: Active,
        n_ops: 20,
    },
    enable_coverage: true,
}
```

Put `#[derive(Debug)]` on every config struct as a matter of policy, and log the tree at the top of every test (`log::info(&format!("{config:#?}"))`); when a run misbehaves, the first question — *what configuration actually ran?* — is answered in the transcript, next to the seed that reproduces it. Second, when a config value encodes a constraint (n_ops must be positive; a passive agent must not be handed a sequence), check it in the config's constructor and return `Result` — Chapter 9's machinery, pushing even value bugs as early as they can go, which is construction time rather than four hundred nanoseconds in.

And when a wrong *value* does slip through to the DUT? Then it is not a configuration bug anymore; it is a stimulus bug wearing configuration's clothes, and the tools are the ones the rest of this book teaches: the scoreboard that catches the consequence, the seed that reproduces it, and the logging levels (Chapter 26) that open the suspect component's mouth.

## Summary

The ConfigDB's debugging chapter ported as a scorecard. Wrong path, wrong type, missing value: compile errors with the guilty line and often the fix in the message. Set-before-get and precedence shadowing: unrepresentable, because one-pass construction leaves no gap for ordering bugs and one-field-one-value leaves nothing to shadow. What remains is the debugging that no type system removes — wrong values, honestly configured — and its tools are `#[derive(Debug)]` on every config, the tree logged at test start, constructor-time validation returning `Result`, and, past that, the ordinary machinery of a checking testbench.

The ConfigDB was one of pyuvm's two big runtime databases. The other — the factory, which turned class names into objects and let tests swap components by override — is next, and its rustdv fate is the same in outline and more interesting in detail: the job stays, the registry goes, and the replacement has been hiding in Chapter 12 all along.
