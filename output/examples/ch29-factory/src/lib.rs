//! Chapter 29: The factory problem, solved by closures and generics.
//!
//!     sim-common/run_sim.sh ch29_factory playground

use rustdv::prelude::*;
use rustdv::sim::log::Logger;

rustdv::vpi_bootstrap!();

// Chapter 29, Figure 1: A tiny example component

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

impl ComponentNode for TinyComponent {
    fn node_name(&self) -> &'static str {
        "TinyComponent"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 29, Figure 2: Instantiating the component by calling new() directly
#[derive(rustdv::Component)]
pub struct TinyEnv {
    #[component(child)]
    tc: TinyComponent,
}

impl Component for TinyEnv {}

#[rustdv::test]
async fn tiny_test(_ctx: TestCtx) -> Result<(), TestError> {
    let mut env = TinyEnv { tc: TinyComponent::new() };
    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 29, Figure 4: A designed variation point: the maker closure

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

impl Component for FlexEnv {}

impl ComponentNode for FlexEnv {
    fn node_name(&self) -> &'static str {
        "FlexEnv"
    }
    fn visit_children(&mut self, f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {
        f("tc", self.tc.as_mut());
    }
}

// Chapter 29, Figure 5: The default maker builds the original component
#[rustdv::test]
async fn tiny_factory_test(_ctx: TestCtx) -> Result<(), TestError> {
    let mut env = FlexEnv::new(FlexEnvConfig::default());
    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 29, Figure 6: The component a test will swap in

pub struct MediumComponent {
    logger: Logger,
}

impl MediumComponent {
    pub fn new() -> MediumComponent {
        MediumComponent { logger: Logger::new("uvm_test_top.tc") }
    }
}

impl Component for MediumComponent {
    fn start(&mut self, ctx: &mut RunCtx) {
        let obj = ctx.raise_objection("medium");
        let logger = self.logger.clone();
        spawn_named(
            async move {
                logger.info("I'm medium size.");
                drop(obj);
            },
            "tc.run",
        );
    }
}

impl ComponentNode for MediumComponent {
    fn node_name(&self) -> &'static str {
        "MediumComponent"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 29, Figure 7: The override is an assignment, visible in the test
#[rustdv::test]
async fn medium_test(_ctx: TestCtx) -> Result<(), TestError> {
    let config = FlexEnvConfig {
        make_tc: Box::new(|| Box::new(MediumComponent::new())),
    };
    let mut env = FlexEnv::new(config);

    let mut run_ctx = RunCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}
