// Chapter 35, Figure 2: Three derives, three dunder families
// Run: cargo run --bin ch35_fig02_three_derives_three_dunder_families

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

fn main() {
    let cmd = AluCommand { a: 0xA5, b: 0x75, op: Ops::Mul };

    // Clone is do_copy: a field-wise deep copy
    let copy = cmd.clone();

    // PartialEq is do_compare: field-wise equality
    println!("copy == cmd: {}", copy == cmd);

    // Debug is convert2string: a readable rendering, for free
    println!("{cmd:?}");

    // ...and mutation of the copy proves it was a copy
    let mut tweaked = cmd.clone();
    tweaked.a = 0;
    println!("tweaked == cmd: {}", tweaked == cmd);
}
