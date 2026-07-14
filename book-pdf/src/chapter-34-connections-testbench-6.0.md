# Chapter 34: Connections in Testbench 6.0

Chapter 33 built six components that don't know each other. This chapter introduces them — and it is the shortest architecture chapter in the book, because rustdv has no `connect_phase` to fill: wiring happens where construction happens, and the whole network fits in one constructor.

> **In Python we...** connected everything in `connect_phase()` methods: the tester's put port to one side of a `uvm_tlm_fifo`, the driver's get port to the other; the monitors' analysis ports to the scoreboard's FIFO exports and the coverage export — a diagram's worth of `connect()` calls, each a runtime operation that could fail with `UVMTLMConnectionError`.

## The environment

```rust
// Figure 1: The environment: every connection is an argument

#[derive(rustdv::Component)]
pub struct AluEnv<T: Tester + 'static> {
    #[component(child)]
    tester: TesterComp<T>,
    #[component(child)]
    driver: Driver,
    #[component(child)]
    cmd_mon: Monitor<CmdTuple>,
    #[component(child)]
    result_mon: Monitor<u64>,
    #[component(child)]
    scoreboard: Scoreboard,
    #[component(child)]
    coverage: Coverage,
}

impl<T: Tester + 'static> AluEnv<T> {
    pub fn new(bfm: Rc<TinyAluBfm>, tester: T) -> AluEnv<T> {
        // build: create the plumbing...
        let (cmd_tx, cmd_rx) = channel::<Cmd>(1);
        let cmd_ap: AnalysisPort<CmdTuple> = AnalysisPort::new();
        let result_ap: AnalysisPort<u64> = AnalysisPort::new();

        // connect: ...and hand each component its endpoints.
        AluEnv {
            tester: TesterComp::new(cmd_tx, tester),
            driver: Driver::new(bfm.clone(), cmd_rx),
            scoreboard: Scoreboard::new(cmd_ap.connect_fifo(), result_ap.connect_fifo()),
            coverage: Coverage::new(&cmd_ap),
            cmd_mon: Monitor::new("cmd_monitor", bfm.clone(), get_cmd, cmd_ap),
            result_mon: Monitor::new("result_monitor", bfm, get_result, result_ap),
        }
    }
}
```

Read `new()` as the wiring diagram, because it is one. The stimulus path: `cmd_tx` to the tester, `cmd_rx` to the driver — one channel, two ends, and the tester-to-driver connection *is* the fact that both ends came from one `channel()` call. The observation paths: each analysis port is created, its FIFO ends handed to the scoreboard, its subscriber hook to coverage, and the port itself to the monitor that will broadcast on it. Six components, five connections, zero `connect()` calls — and zero possible `UVMTLMConnectionError`s, because every mistake this figure could contain (wrong direction, wrong transaction type, missing endpoint) is one of the compile errors Chapters 27 and 31 already demonstrated.

Order matters in one instructive way: the scoreboard and coverage lines run *before* the monitor lines, because `connect_fifo()` and `connect(...)` borrow the ports, and the `Monitor::new` calls then consume them. The borrow checker enforces build-then-connect-then-handoff sequencing that pyuvm needed separate phases to guarantee.

Comparing hierarchies with pyuvm's 6.0 turns up one absentee: the `uvm_tlm_fifo` component that sat *between* tester and driver. Our channel does that FIFO's job without appearing in the hierarchy — buffering is plumbing here, not a citizen. (When a buffer *should* be visible — introspectable depth, flushing — `TlmFifo<T>` from Chapter 31 takes a `#[component(child)]` slot. This testbench doesn't need one.)

## The tests

```rust
// Figure 2: The test body — unchanged since 4.0

async fn run_test<T: Tester + 'static>(ctx: &TestCtx, tester: T) -> Result<(), TestError> {
    Clock::new(&ctx.dut().signal("clk")?, SimDuration::ns(10)).start();
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);

    let mut env = AluEnv::new(bfm, tester);

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}
```

The skeleton from Chapter 25, verbatim but shorter — reset and `start_tasks` moved into the driver (Chapter 33), so the test touches the BFM only to create it. `random_test` and `max_test` are the familiar one-liners.

```text
# Figure 3: The 6.0 testbench running
--
     45.00ns INFO     cmd_monitor: (193, 103, 1)
     45.00ns INFO     result_monitor: 296
     65.00ns INFO     cmd_monitor: (94, 11, 2)
     65.00ns INFO     result_monitor: 10
     85.00ns INFO     cmd_monitor: (185, 128, 3)
     85.00ns INFO     result_monitor: 57
    105.00ns INFO     cmd_monitor: (165, 117, 4)
    135.00ns INFO     result_monitor: 19305
    235.00ns INFO     PASSED: c1 Add 67 = 0128
    235.00ns INFO     PASSED: 5e And 0b = 000a
    235.00ns INFO     PASSED: b9 Xor 80 = 0039
    235.00ns INFO     PASSED: a5 Mul 75 = 4b69
    235.00ns INFO     Covered all operations
    235.00ns INFO     random_test PASSED
```

A new voice in the transcript: the monitors narrate live — `cmd_monitor: (165, 117, 4)` as each transaction crosses the bus — while the scoreboard's verdicts arrive in a batch at check time. This is the Interlude's log format taking shape, and it is the operational payoff of the fan-out: the same broadcast feeds the log, the scoreboard, and coverage, none of them aware of the others.

## The agent-shaped hole

One structural note before moving on, because the Python book's readers will be looking for it. SystemVerilog UVM (and pyuvm, in larger examples) groups driver-monitor-sequencer into an **agent** — the reusable bundle for one bus interface, with an `is_active` switch so a agent embedded in a bigger system can keep its monitors and drop its driver. The TinyALU testbench is small enough that the env plays the agent's role directly, but the pattern is worth one paragraph of Rust, because it lands on machinery you already have:

```rust
// Figure 4: The agent pattern: active/passive as an enum and two Options

pub struct AluAgentConfig {
    pub is_active: Active,   // Active | Passive — an enum, not a string
    pub bfm: Rc<TinyAluBfm>,
}

pub struct AluAgent {
    driver: Option<Driver>,      // a passive agent simply has no driver
    tester: Option<TesterComp<RandomTester>>,
    cmd_mon: Monitor<CmdTuple>,  // monitors exist in every configuration
    result_mon: Monitor<u64>,
}
```

In pyuvm, an agent read `is_active` from the ConfigDB and *warned at runtime* if the value was garbage; a passive agent still had driver-shaped code paths to keep inert. Here `Active` is a two-variant enum (garbage unrepresentable), and a passive agent's constructor simply doesn't build the driver: `driver: None`. The derive's traversal skips `None` children, the lifecycle never starts what doesn't exist, and the illegal state — a passive agent driving the bus — cannot be constructed. The capstone testbench in Chapter 40 assembles a full agent along exactly these lines.

## Summary

Testbench 6.0 assembled. The environment's constructor is the block diagram: a `channel()` whose ends connect tester to driver, two analysis ports fanning monitor observations out to scoreboard FIFOs and the coverage subscriber, every connection an argument, every wiring mistake a compile error, and the borrow checker enforcing the build/connect ordering that used to need phases. The tests kept their one-decision shape, and the transcript gained the live monitor narration that the fan-out makes free. The agent pattern — active/passive as enum plus `Option` children — closes the structural story.

What remains is the data itself. The testbench still slings `(u64, u64, u64)` tuples, with `op` living at index 2 by convention — the very complaint Chapter 22 recorded. Before sequences can give stimulus its final form, the transactions need theirs: `uvm_object`, and the three derives that replace it.
