# Chapter 28: Debugging the ConfigDb — figure map

Run with:

```
sim-common/run_sim.sh ch28_config_debugging playground
```

No DUT. Chapter 27 showed configuration working; this one shows it failing,
which is the more useful skill.

| Figure | Title | Where |
|---|---|---|
| 1 | The logger that propagates the error | `MsgLogger` |
| 2 | A message for only one of two loggers | `MsgTest` |
| 3 | Misspelling a key | `MsgTestAlmostFixed` |
| 4 | A logger that copes | `NiceMsgLogger` |
| 5 | Printing the ConfigDb | `NiceMsgTest` |
| 6 | Debugging the misspelled key by printing | `NiceMsgTestAlmostFixed` |
| 7 | Wildcards behaving, for contrast | `MultiMsgTest` |
| 8 | Debugging a parent/child conflict | `ConflictTest` |
| 9 | Tracing every ConfigDb operation | `GlobalTest` |

Port of the Python book's chapter 32.

## The three debug tools

**A `Result` that names the cause.** `ConfigError::NotFound` and
`TypeMismatch` are separate variants, so Figure 4 can recover from a missing
value while still failing on a wrong type — defaulting past a type mismatch
would bury a real bug. SystemVerilog collapses both into `return 0`.

**A dump that shows the competition.** `ConfigDb::print()` lists every entry
with its precedences, because a resolved value tells you who won but not who
else was trying:

```
PATH                        : KEY       : DATA
ConflictTest.env.loga       : MSG       : {1000: "PARENT RULES!", 999: "CHILD RULES!"}
```

The test wrote at depth 0 (precedence 1000) and the env at depth 1 (999), so
the parent wins — and the numbers say so rather than asking you to trust the
rule. Figure 6 uses the same dump to spot `MSG` beside `MESG`.

**A tracer.** `ConfigDb::set_tracing(true)` logs every operation with the
context, the offset, and the path they resolved to — which is the thing you
actually got wrong when a lookup misses:

```
CFGDB/SET context=GlobalTest offset="env.loga" -> GlobalTest.env.loga MSG="LOG A msg"
CFGDB/SET context=<none> offset="*" -> * MSG="GLOBAL"
CFGDB/GET context=GlobalTest.env.gtalk offset="" -> GlobalTest.env.gtalk MSG="GLOBAL"
```

## Expected failures

Figures 2 and 3 are declared
`#[rustdv::test(expect_error = "config_not_found")]` — they pass only if they
fail *that way*. A test that failed for some other reason is still reported
as a failure, which a plain "expected to fail" flag could not tell you.

Verified: changing one expectation to the wrong kind produces

```
MsgTest FAILED: expected error 'config_type_mismatch', got config_not_found: ...
```

## What this chapter replaced

It was previously three programs that failed to compile on purpose, arguing
that configuration bugs are caught by the compiler — Figure 1 demonstrated a
"wrong path" by misspelling a *struct field*. A wrong path in a config
database is `"env.loga"` versus `"env.logA"`: a string, resolved at run time,
that no compiler will ever check. Late binding costs you compile-time
checking and buys you these tools instead. That trade is the honest version.
