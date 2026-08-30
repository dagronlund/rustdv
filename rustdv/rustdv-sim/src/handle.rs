//! Typed DUT handles with scheduled-write semantics and edge methods —
//! the user-facing layer over rustdv-gpi (design-doc mapping rows 14,
//! 19–22). `set()` is buffered and applied at the next ReadWrite phase;
//! `set_now()` is the immediate variant.

use rustdv_gpi as gpi;
pub use rustdv_gpi::{BigUint, HandleError, Logic, LogicArray, ValueError};

use crate::phase;
use crate::triggers::{Edge, EdgeKind};

/// A module/scope handle: `dut.child("name")?` (OQ-6 dynamic-first lean).
#[derive(Copy, Clone)]
pub struct HierarchyHandle {
    raw: gpi::HierarchyHandle,
}
impl HierarchyHandle {
    /// A handle to nothing, for unit tests that need a `RustdvCtx` but never
    /// touch the DUT. Any VPI call through it reaches `rustdv-vpi-stubs`,
    /// which panics — so a test that *does* touch the DUT fails loudly rather
    /// than reading garbage, and its author learns it belongs in a `sim-*`
    /// case instead.
    pub fn null_for_test() -> HierarchyHandle {
        HierarchyHandle {
            raw: gpi::HierarchyHandle::null_for_test(),
        }
    }
}

impl HierarchyHandle {
    pub fn child(&self, name: &str) -> Result<AnyHandle, HandleError> {
        Ok(AnyHandle::wrap(self.raw.child(name)?))
    }

    /// Child that must be a signal.
    pub fn signal(&self, name: &str) -> Result<LogicHandle, HandleError> {
        Ok(LogicHandle {
            raw: self.raw.child(name)?.as_logic()?,
        })
    }

    pub fn name(&self) -> String {
        self.raw.name()
    }
    pub fn full_name(&self) -> String {
        self.raw.full_name()
    }

    pub fn children(&self) -> Vec<AnyHandle> {
        self.raw
            .children()
            .into_iter()
            .map(AnyHandle::wrap)
            .collect()
    }
}

#[derive(Copy, Clone)]
pub enum AnyHandle {
    Hierarchy(HierarchyHandle),
    Logic(LogicHandle),
    Other,
}

impl AnyHandle {
    fn wrap(h: gpi::AnyHandle) -> AnyHandle {
        match h {
            gpi::AnyHandle::Hierarchy(h) => AnyHandle::Hierarchy(HierarchyHandle { raw: h }),
            gpi::AnyHandle::Logic(l) => AnyHandle::Logic(LogicHandle { raw: l }),
            gpi::AnyHandle::Other(_) => AnyHandle::Other,
        }
    }

    pub fn as_logic(self) -> Option<LogicHandle> {
        match self {
            AnyHandle::Logic(l) => Some(l),
            _ => None,
        }
    }
    pub fn as_hierarchy(self) -> Option<HierarchyHandle> {
        match self {
            AnyHandle::Hierarchy(h) => Some(h),
            _ => None,
        }
    }
}

/// A value-bearing signal. Explicit `get()`/`set()` (mapping row 20 —
/// cocotb 2.x itself moved off the `.value` property).
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct LogicHandle {
    raw: gpi::LogicHandle,
}

impl std::fmt::Debug for LogicHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LogicHandle(\"{}\")", self.full_name())
    }
}

impl std::fmt::Debug for HierarchyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HierarchyHandle(\"{}\")", self.full_name())
    }
}

impl LogicHandle {
    pub fn name(&self) -> String {
        self.raw.name()
    }
    pub fn full_name(&self) -> String {
        self.raw.full_name()
    }
    pub fn size(&self) -> u32 {
        self.raw.size()
    }

    // ----------------- Read: current value (mapping row 19) -----------------

    /// Current boolean value, or `Err` if VPI does not supply it.
    pub fn get_bool(&self) -> Result<bool, ValueError> {
        self.raw.get_bool()
    }

    /// Current unsigned 8-bit value, or `Err` if VPI does not supply it.
    pub fn get_u8(&self) -> Result<u8, ValueError> {
        self.raw.get_u8()
    }
    /// Current unsigned 16-bit value, or `Err` if VPI does not supply it.
    pub fn get_u16(&self) -> Result<u16, ValueError> {
        self.raw.get_u16()
    }

    /// Current unsigned 32-bit value, or `Err` if VPI does not supply it.
    pub fn get_u32(&self) -> Result<u32, ValueError> {
        self.raw.get_u32()
    }

    /// Current unsigned 64-bit value, or `Err` if VPI does not supply it.
    pub fn get_u64(&self) -> Result<u64, ValueError> {
        self.raw.get_u64()
    }

    /// Current unsigned 128-bit value, or `Err` if VPI does not supply it.
    pub fn get_u128(&self) -> Result<u128, ValueError> {
        self.raw.get_u128()
    }

    /// Current arbitrary-width unsigned value, or `Err` if VPI does not supply it.
    pub fn get_bigint(&self) -> Result<BigUint, ValueError> {
        self.raw.get_bigint()
    }

    /// Current four-state value, or `Err` if VPI does not supply it.
    pub fn get_logic(&self) -> Result<LogicArray, ValueError> {
        self.raw.get_logic()
    }

    /// Current four-state value formatted as an MSB-first binary string.
    pub fn get_binstr(&self) -> Result<String, ValueError> {
        Ok(self.raw.get_logic()?.to_binstr())
    }

    /// Convenience for 1-bit signals: true iff the value is 1b1.
    pub fn is_high(&self) -> bool {
        self.raw.get_bool() == Ok(true)
    }

    /// Convenience for 1-bit signals: true iff the value is 1b0.
    pub fn is_low(&self) -> bool {
        self.raw.get_bool() == Ok(false)
    }

    // ------ Write: scheduled (applied at next ReadWrite phase, row 22) ------

    /// Set the boolean value to be applied at the next ReadWrite phase. The
    /// write is buffered and the last write wins, preserving order of distinct
    /// signals.
    pub fn set_bool(&self, v: bool) {
        phase::schedule(self.raw, phase::WriteVal::Bool(v));
    }

    /// Set the unsigned 8-bit value to be applied at the next ReadWrite phase.
    /// The write is buffered and the last write wins, preserving order of
    /// distinct signals.
    pub fn set_u8(&self, v: u8) {
        phase::schedule(self.raw, phase::WriteVal::U8(v));
    }

    /// Set the unsigned 16-bit value to be applied at the next ReadWrite phase.
    /// The write is buffered and the last write wins, preserving order of
    /// distinct signals.
    pub fn set_u16(&self, v: u16) {
        phase::schedule(self.raw, phase::WriteVal::U16(v));
    }

    /// Set the unsigned 32-bit value to be applied at the next ReadWrite phase.
    /// The write is buffered and the last write wins, preserving order of
    /// distinct signals.
    pub fn set_u32(&self, v: u32) {
        phase::schedule(self.raw, phase::WriteVal::U32(v));
    }

    /// Set the unsigned 64-bit value to be applied at the next ReadWrite phase.
    /// The write is buffered and the last write wins, preserving order of
    /// distinct signals.
    pub fn set_u64(&self, v: u64) {
        phase::schedule(self.raw, phase::WriteVal::U64(v));
    }

    /// Set the unsigned 128-bit value to be applied at the next ReadWrite
    /// phase. The write is buffered and the last write wins, preserving order
    /// of distinct signals.
    pub fn set_u128(&self, v: u128) {
        phase::schedule(self.raw, phase::WriteVal::U128(v));
    }

    /// Set the arbitrary-width unsigned value to be applied at the next
    /// ReadWrite phase. The write is buffered and the last write wins,
    /// preserving order of distinct signals.
    pub fn set_bigint(&self, v: &BigUint) {
        phase::schedule(self.raw, phase::WriteVal::BigInt(v.clone()));
    }

    /// Set the four-state value to be applied at the next ReadWrite phase. The
    /// write is buffered and the last write wins, preserving order of distinct
    /// signals.
    pub fn set_logic(&self, v: &LogicArray) {
        phase::schedule(self.raw, phase::WriteVal::Logic(v.clone()));
    }

    // -------------- write: immediate (setimmediatevalue analog) --------------
    //
    // Immediate skips the write *scheduler*, not the phase *rule* (D108). The
    // rule is the simulator's: writing during ReadOnly is illegal however the
    // write gets there. Left unchecked, these two went straight to
    // `vpi_put_value` and Icarus swallowed them with a printed diagnostic —
    // "attempted to put a value to variable 'x' during a read-only synch
    // callback" — after which the run continued on values that were never
    // applied. That is the worst kind of wrong: it looks like a warning and it
    // silently changes results.

    /// Set the boolean value immediately, skipping the write scheduler. The
    /// write is applied at once, but the ReadOnly rule is still enforced.
    pub fn set_bool_now(&self, v: bool) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_bool_now(v);
    }

    /// Set the unsigned 8-bit value immediately, skipping the write scheduler.
    /// The write is applied at once, but the ReadOnly rule is still enforced.
    pub fn set_u8_now(&self, v: u8) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_u8_now(v);
    }

    /// Set the unsigned 16-bit value immediately, skipping the write scheduler.
    /// The write is applied at once, but the ReadOnly rule is still enforced.
    pub fn set_u16_now(&self, v: u16) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_u16_now(v);
    }

    /// Set the unsigned 32-bit value immediately, skipping the write scheduler.
    /// The write is applied at once, but the ReadOnly rule is still enforced.
    pub fn set_u32_now(&self, v: u32) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_u32_now(v);
    }

    /// Set the unsigned 64-bit value immediately, skipping the write scheduler.
    /// The write is applied at once, but the ReadOnly rule is still enforced.
    pub fn set_u64_now(&self, v: u64) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_u64_now(v);
    }

    /// Set the unsigned 128-bit value immediately, skipping the write
    /// scheduler. The write is applied at once, but the ReadOnly rule is still
    /// enforced.
    pub fn set_u128_now(&self, v: u128) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_u128_now(v);
    }

    /// Set the arbitrary-width unsigned value immediately, skipping the write
    /// scheduler. The write is applied at once, but the ReadOnly rule is still
    /// enforced.
    pub fn set_bigint_now(&self, v: &BigUint) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_bigint_now(v);
    }

    /// Set the four-state value immediately, skipping the write scheduler. The
    /// write is applied at once, but the ReadOnly rule is still enforced.
    pub fn set_logic_now(&self, v: &LogicArray) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_logic_now(v);
    }

    // ---- edges (methods on typed handles, mapping row 14) ----
    pub fn rising_edge(&self) -> Edge {
        Edge::new(self.raw, EdgeKind::Rising)
    }
    pub fn falling_edge(&self) -> Edge {
        Edge::new(self.raw, EdgeKind::Falling)
    }
    pub fn value_change(&self) -> Edge {
        Edge::new(self.raw, EdgeKind::AnyChange)
    }
}

/// The first top-level module (the DUT in single-top designs).
pub fn top_module() -> Result<HierarchyHandle, HandleError> {
    Ok(HierarchyHandle {
        raw: gpi::top_module()?,
    })
}
