//! Chapter 27: Configuration — the ConfigDB's job, done by types.
//!
//!     sim-common/run_sim.sh ch27_configuration playground

use rustdv::prelude::*;
use rustdv::sim::log::Logger;

rustdv::vpi_bootstrap!();

// Chapter 27, Figure 1: A component that needs configuration
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
    fn start(&mut self, ctx: &mut RustdvCtx) {
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

impl ComponentNode for MsgLogger {
    fn node_name(&self) -> &'static str {
        "MsgLogger"
    }
    fn visit_children(&mut self, _f: &mut dyn FnMut(&str, &mut dyn ComponentNode)) {}
}

// Chapter 27, Figure 2: The config struct mirrors the hierarchy
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

impl Component for MsgEnv {}

// Chapter 27, Figure 3: The test builds the config and hands it over
#[rustdv::test]
async fn msg_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let config = MsgEnvConfig {
        loga_msg: "LOG A msg".to_string(),
        logb_msg: "LOG B msg".to_string(),
    };
    let mut env = MsgEnv::new(config);

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 27, Figure 5: "Wildcards" become one field used twice
pub struct MultiMsgConfig {
    pub loga_msg: String,
    pub logb_msg: String,
    pub talk_msg: String, // talka AND talkb: sharing is visible in new()
}

#[derive(rustdv::Component)]
pub struct MultiMsgEnv {
    #[component(child)]
    talka: MsgLogger,
    #[component(child)]
    talkb: MsgLogger,
    #[component(child)]
    loga: MsgLogger,
    #[component(child)]
    logb: MsgLogger,
}

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

impl Component for MultiMsgEnv {}

#[rustdv::test]
async fn multi_msg_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let config = MultiMsgConfig {
        loga_msg: "LOG A msg".to_string(),
        logb_msg: "LOG B msg".to_string(),
        talk_msg: "TALK TALK".to_string(),
    };
    let mut env = MultiMsgEnv::new(config);

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}

// Chapter 27, Figure 7: "Global data" becomes a Default implementation
pub struct GlobalConfig {
    pub loga_msg: String,
    pub logb_msg: String,
    pub talk_msg: String,
    pub gtalk_msg: String,
}

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

#[derive(rustdv::Component)]
pub struct GlobalEnv {
    #[component(child)]
    gtalk: MsgLogger,
    #[component(child)]
    talka: MsgLogger,
    #[component(child)]
    loga: MsgLogger,
    #[component(child)]
    logb: MsgLogger,
}

impl GlobalEnv {
    pub fn new(config: GlobalConfig) -> GlobalEnv {
        GlobalEnv {
            gtalk: MsgLogger::new("env.gtalk", config.gtalk_msg),
            talka: MsgLogger::new("env.talka", config.talk_msg),
            loga: MsgLogger::new("env.loga", config.loga_msg),
            logb: MsgLogger::new("env.logb", config.logb_msg),
        }
    }
}

impl Component for GlobalEnv {}

// Chapter 27, Figure 8: Overriding some fields, defaulting the rest
#[rustdv::test]
async fn global_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    let config = GlobalConfig {
        loga_msg: "LOG A msg".to_string(),
        logb_msg: "LOG B msg".to_string(),
        talk_msg: "TALK TALK".to_string(),
        ..GlobalConfig::default() // gtalk_msg falls back to "GLOBAL"
    };
    let mut env = GlobalEnv::new(config);

    let mut run_ctx = RustdvCtx::new();
    start_all(&mut env, &mut run_ctx);
    run_ctx.all_objections_dropped().await;
    run_extract_check_report(&mut env).map_err(TestError::from)
}
