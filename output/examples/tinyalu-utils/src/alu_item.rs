//! The struct transactions (Chapter 35, Figure 1): plain data, three
//! derives, no base class.

use crate::Ops;

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}

/// The golden model over plain data — testable with no simulator.
pub fn predict(cmd: &AluCommand) -> AluResult {
    let a = cmd.a as u16;
    let b = cmd.b as u16;
    let result = match cmd.op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    };
    AluResult { result }
}
