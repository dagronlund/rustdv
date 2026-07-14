// Chapter 21, Figure 4: Link-time registration in miniature
// Run: cargo run --bin ch21_fig04_link_time_registration_in_miniature
// (Linux/ELF: the technique rustdv's test registry uses, boiled to its
// bones. The __start_/__stop_ section symbols are an ELF feature, so on
// macOS/Windows this binary just prints a pointer to a Linux box.)

#![allow(dead_code)]

/// What a registration carries: a name and a function to run.
struct Registration {
    name: &'static str,
    run: fn(),
}

// Each "test" plants one static in the linker section `demo_tests`.
// The linker concatenates every such static, from every object file,
// into one contiguous array — no runtime registration step anywhere.

fn hello() {
    println!("Hello, world.");
}
#[cfg(target_os = "linux")]
#[used]
#[link_section = "demo_tests"]
static REG_HELLO: Registration = Registration { name: "hello", run: hello };

fn goodbye() {
    println!("Goodbye, world.");
}
#[cfg(target_os = "linux")]
#[used]
#[link_section = "demo_tests"]
static REG_GOODBYE: Registration = Registration { name: "goodbye", run: goodbye };

// The linker defines __start_<section> and __stop_<section> for us.
#[cfg(target_os = "linux")]
extern "C" {
    static __start_demo_tests: u8;
    static __stop_demo_tests: u8;
}

#[cfg(target_os = "linux")]
fn collect() -> &'static [Registration] {
    unsafe {
        let start = std::ptr::addr_of!(__start_demo_tests) as *const Registration;
        let stop = std::ptr::addr_of!(__stop_demo_tests) as *const Registration;
        std::slice::from_raw_parts(start, stop.offset_from(start) as usize)
    }
}

#[cfg(target_os = "linux")]
fn main() {
    let tests = collect();
    println!("found {} registered tests:", tests.len());
    for t in tests {
        print!("  {} -> ", t.name);
        (t.run)();
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("This figure demonstrates ELF linker sections - run it on Linux.");
    println!("(rustdv's real registry handles the per-platform spellings.)");
}
