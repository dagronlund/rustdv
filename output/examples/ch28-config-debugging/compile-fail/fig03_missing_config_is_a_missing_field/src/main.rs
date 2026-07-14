// Chapter 28, Figure 3: Forgetting to configure is a missing field
// This program FAILS TO COMPILE on purpose. See EXPECTED.txt.

struct AluEnvConfig {
    enable_coverage: bool,
    n_ops: u32,
}

fn main() {
    // In pyuvm: the get() raised UVMConfigItemNotFound at build time —
    // if you were lucky. Here the test simply doesn't build:
    let config = AluEnvConfig { enable_coverage: true };
    let _ = config;
}
