//! Test failure value.
//!
//! `Err` fails the test (design-doc §0.6): `Result` for *checks*, panics for
//! *testbench bugs* (§7.3 failure taxonomy).
//!
//! This lived in `rustdv-runner` until step 4. `Component::run` returns it
//! (D46/D47), and `rustdv-uvm` sits below the runner, so the type had to
//! move down. `rustdv-runner` re-exports it, and `::rustdv::TestError`
//! resolves exactly as before.

use std::fmt;

use rustdv_sim::executor::TaskError;
use rustdv_sim::{HandleError, ValueError};

use crate::sequence::SeqError;

#[derive(Debug, Clone)]
pub struct TestError(pub String);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TestError {}

impl From<&str> for TestError {
    fn from(s: &str) -> Self {
        TestError(s.into())
    }
}
impl From<String> for TestError {
    fn from(s: String) -> Self {
        TestError(s)
    }
}
impl From<HandleError> for TestError {
    fn from(e: HandleError) -> Self {
        TestError(e.to_string())
    }
}
impl From<ValueError> for TestError {
    fn from(e: ValueError) -> Self {
        TestError(e.to_string())
    }
}
impl From<SeqError> for TestError {
    fn from(e: SeqError) -> Self {
        TestError(e.to_string())
    }
}
impl From<TaskError> for TestError {
    fn from(e: TaskError) -> Self {
        TestError(e.to_string())
    }
}
