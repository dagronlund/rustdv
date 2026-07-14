// Chapter 28, Figure 2: The "wrong type" mistake never reaches runtime
// This program FAILS TO COMPILE on purpose. See EXPECTED.txt.

struct AluEnvConfig {
    enable_coverage: bool,
}

fn main() {
    // In pyuvm: set(..., "ENABLE", "true") stored a *string*; the component
    // exploded (or worse, didn't) at the point of use, mid-simulation.
    let config = AluEnvConfig { enable_coverage: "true" };
    let _ = config;
}
