// Chapter 28, Figure 5: The debugging that remains — print the config tree
// Run: cargo run --bin ch28_fig05_printing_the_config_tree

#[derive(Debug)]
#[allow(dead_code)]
enum Active {
    Active,
    Passive,
}

#[derive(Debug)]
#[allow(dead_code)]
struct AluAgentConfig {
    is_active: Active,
    n_ops: u32,
}

#[derive(Debug)]
#[allow(dead_code)]
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
