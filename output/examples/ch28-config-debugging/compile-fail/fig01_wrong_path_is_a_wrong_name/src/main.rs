// Chapter 28, Figure 1: The "wrong path" mistake is now a wrong name
// This program FAILS TO COMPILE on purpose. See EXPECTED.txt.

struct AluEnvConfig {
    enable_coverage: bool,
}

fn main() {
    // In pyuvm: ConfigDB().set(self, "env.covergae", "ENABLE", True)
    // matched nothing, silently, and the component used its default.
    let config = AluEnvConfig { enable_covergae: true };
    let _ = config;
}
