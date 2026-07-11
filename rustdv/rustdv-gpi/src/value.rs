//! 4-state value types: port of cocotb's `Logic`/`LogicArray`
//! (cocotb: types/, design-doc mapping row 23). Conversion failures are
//! `Result`s, not exceptions.

use std::fmt;

use crate::ValueError;

/// One 4-state bit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Logic {
    Zero,
    One,
    X,
    Z,
}

impl Logic {
    pub fn from_char(c: char) -> Logic {
        match c {
            '0' => Logic::Zero,
            '1' => Logic::One,
            'z' | 'Z' => Logic::Z,
            _ => Logic::X,
        }
    }
    pub fn to_char(self) -> char {
        match self {
            Logic::Zero => '0',
            Logic::One => '1',
            Logic::X => 'x',
            Logic::Z => 'z',
        }
    }
    pub fn is_resolvable(self) -> bool {
        matches!(self, Logic::Zero | Logic::One)
    }
}

impl From<bool> for Logic {
    fn from(b: bool) -> Logic {
        if b { Logic::One } else { Logic::Zero }
    }
}

impl fmt::Display for Logic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// A fixed-width vector of 4-state bits, MSB first (binstr order).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicArray {
    bits: Vec<Logic>,
}

impl LogicArray {
    pub fn from_binstr(s: &str) -> LogicArray {
        LogicArray { bits: s.chars().map(Logic::from_char).collect() }
    }

    pub fn from_u64(v: u64, width: usize) -> LogicArray {
        let mut bits = Vec::with_capacity(width);
        for i in (0..width).rev() {
            bits.push(Logic::from((v >> i) & 1 == 1));
        }
        LogicArray { bits }
    }

    pub fn to_binstr(&self) -> String {
        self.bits.iter().map(|b| b.to_char()).collect()
    }

    pub fn len(&self) -> usize {
        self.bits.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    pub fn is_resolvable(&self) -> bool {
        self.bits.iter().all(|b| b.is_resolvable())
    }

    /// MSB-first bit access.
    pub fn bit(&self, i: usize) -> Option<Logic> {
        self.bits.get(i).copied()
    }

    pub fn to_u64(&self) -> Result<u64, ValueError> {
        let mut v: u64 = 0;
        for b in &self.bits {
            match b {
                Logic::Zero => v <<= 1,
                Logic::One => v = (v << 1) | 1,
                _ => return Err(ValueError::FourState(self.to_binstr())),
            }
        }
        Ok(v)
    }
}

impl TryFrom<&LogicArray> for u64 {
    type Error = ValueError;
    fn try_from(v: &LogicArray) -> Result<u64, ValueError> {
        v.to_u64()
    }
}

impl fmt::Display for LogicArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_binstr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_u64() {
        let v = LogicArray::from_u64(0xA5, 8);
        assert_eq!(v.to_binstr(), "10100101");
        assert_eq!(v.to_u64().unwrap(), 0xA5);
    }

    #[test]
    fn four_state_rejected() {
        let v = LogicArray::from_binstr("1x0z");
        assert!(v.to_u64().is_err());
        assert!(!v.is_resolvable());
    }
}
