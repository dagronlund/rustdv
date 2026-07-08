// Rust for RTL Verification — Chapter 10, Figure 4
// "The shape of rustvm's Component trait (preview — signatures only)"


pub trait Component {
    fn start(&mut self, ctx: &mut RunCtx);            // required: every component runs
    fn check(&mut self, errors: &mut CheckSink) {}    // default: do nothing
    fn report(&self) {}                               // default: do nothing
}
