// Chapter 35, Figure 4: Comparison policy lives in the checker
// Run: cargo run --bin ch35_fig04_comparison_policy_lives_in_the_checker

#[derive(Clone, Debug, PartialEq)]
pub struct BusResult {
    pub data: u16,
    pub timestamp_ns: u64, // interesting to the log, irrelevant to correctness
}

/// A scoreboard that takes its notion of "matches" as a value.
fn check(expected: &BusResult, actual: &BusResult, matches: impl Fn(&BusResult, &BusResult) -> bool) {
    if matches(expected, actual) {
        println!("PASSED: {actual:?}");
    } else {
        println!("FAILED: {actual:?} - expected {expected:?}");
    }
}

fn main() {
    let expected = BusResult { data: 0x4B69, timestamp_ns: 100 };
    let actual = BusResult { data: 0x4B69, timestamp_ns: 130 };

    // Structural equality says no — the timestamps differ:
    check(&expected, &actual, |e, a| e == a);

    // The policy this DUT needs: compare the data, ignore the clock.
    // In pyuvm this was a do_compare override on the *transaction*;
    // here it is a closure handed to the *checker*.
    check(&expected, &actual, |e, a| e.data == a.data);
}
