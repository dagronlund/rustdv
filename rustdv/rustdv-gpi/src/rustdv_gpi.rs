//! # rustdv-gpi
//!
//! Safe wrapper over the simulator programming interface (design-doc §3.3).
//! Invariants upheld here so everything above is safe Rust:
//!
//! 1. Handles are opaque and non-null; fallible acquisition is `Result`.
//! 2. Object-handle lifetime = simulation lifetime (freely `Copy`able IDs).
//!    Callback registrations are modeled by RAII ([`CallbackHandle`]):
//!    dropping a live handle removes it, and fired one-shots remove themselves
//!    from inside the trampoline while both supported simulators still accept
//!    the registration handle.
//! 3. Strings are copied at the boundary, every call.
//! 4. No unwinding across FFI: every trampoline wraps the closure in
//!    `catch_unwind`; panics are routed to the panic sink.
//! 5. Callback user-data ownership: an `Rc` whose C-side reference is
//!    reclaimed exactly once (on fire for one-shots, on removal otherwise).
//!
//! Thread affinity (§3.4): all types here hold raw pointers and are
//! therefore `!Send`/`!Sync` — the compiler rejects moving them off the
//! simulator thread.

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

pub use num_bigint::BigUint;
use rustdv_gpi_sys::{self as sys, t_vpi_vecval};

// Test executables need vpi_* symbol definitions (the simulator provides
// them for the real cdylib) — see rustdv-vpi-stubs.
#[cfg(test)]
use rustdv_vpi_stubs as _;

pub mod value;
pub use value::{Logic, LogicArray};
pub mod util;
pub use util::ToVpiWords;

// ===========================================================================
// Errors
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    /// Name did not resolve (cocotb raises AttributeError here; rustdv
    /// returns this — design-doc §0.6).
    NotFound {
        name: String,
        scope: String,
    },
    /// Handle exists but is not the requested kind.
    WrongKind {
        name: String,
        expected: &'static str,
        actual: String,
    },
    NoTopModule,
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::NotFound { name, scope } => {
                write!(f, "no object named '{name}' in scope '{scope}'")
            }
            HandleError::WrongKind {
                name,
                expected,
                actual,
            } => {
                write!(f, "'{name}' is a {actual}, expected {expected}")
            }
            HandleError::NoTopModule => write!(f, "no top-level module found"),
        }
    }
}
impl std::error::Error for HandleError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    /// Value contains x/z bits and was asked for as an integer.
    FourState(String),
    /// The simulator did not supply the requested value representation.
    Unavailable,
    Width {
        want: u32,
        have: usize,
    },
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueError::FourState(s) => write!(f, "value '{s}' has x/z bits"),
            ValueError::Unavailable => write!(f, "simulator did not return a vector value"),
            ValueError::Width { want, have } => {
                write!(f, "width mismatch: want {want}, have {have}")
            }
        }
    }
}
impl std::error::Error for ValueError {}

// ===========================================================================
// Handles
// ===========================================================================

/// Raw non-null simulator object handle. Valid for the whole simulation
/// (invariant 2), hence `Copy`. `!Send` because it wraps a raw pointer.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ObjHandle(sys::vpiHandle);

impl ObjHandle {
    fn new(h: sys::vpiHandle) -> Option<Self> {
        if h.is_null() {
            None
        } else {
            Some(ObjHandle(h))
        }
    }
    fn get(self, prop: i32) -> i32 {
        unsafe { sys::vpi_get(prop, self.0) }
    }
    fn get_str(self, prop: i32) -> String {
        // Invariant 3: copy immediately.
        unsafe {
            let p = sys::vpi_get_str(prop, self.0);
            if p.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }
}

/// Any simulator object, classified by type (design-doc §3.3 downcasting).
#[derive(Copy, Clone)]
pub enum AnyHandle {
    Hierarchy(HierarchyHandle),
    Logic(LogicHandle),
    Other(ObjHandle),
}

impl AnyHandle {
    pub fn classify(h: ObjHandle) -> AnyHandle {
        match h.get(sys::vpiType) {
            sys::vpiModule => AnyHandle::Hierarchy(HierarchyHandle { h }),
            sys::vpiNet
            | sys::vpiReg
            | sys::vpiIntegerVar
            | sys::vpiPort
            | sys::vpiMemory
            | sys::vpiLongIntVar
            | sys::vpiShortIntVar
            | sys::vpiIntVar
            | sys::vpiByteVar
            | sys::vpiEnumVar
            | sys::vpiBitVar => {
                // Signal width is immutable for the lifetime of a VPI
                // object. Cache it at discovery so every value read/write
                // does not pay for another vpi_get(vpiSize) crossing.
                let width = h.get(sys::vpiSize).max(0) as u32;
                AnyHandle::Logic(LogicHandle { h, width })
            }
            _ => AnyHandle::Other(h),
        }
    }

    pub fn as_logic(self) -> Result<LogicHandle, HandleError> {
        match self {
            AnyHandle::Logic(l) => Ok(l),
            AnyHandle::Hierarchy(h) => Err(HandleError::WrongKind {
                name: h.full_name(),
                expected: "signal",
                actual: "module".into(),
            }),
            AnyHandle::Other(o) => Err(HandleError::WrongKind {
                name: o.get_str(sys::vpiFullName),
                expected: "signal",
                actual: format!("vpiType {}", o.get(sys::vpiType)),
            }),
        }
    }

    pub fn as_hierarchy(self) -> Result<HierarchyHandle, HandleError> {
        match self {
            AnyHandle::Hierarchy(h) => Ok(h),
            AnyHandle::Logic(l) => Err(HandleError::WrongKind {
                name: l.full_name(),
                expected: "module",
                actual: "signal".into(),
            }),
            AnyHandle::Other(o) => Err(HandleError::WrongKind {
                name: o.get_str(sys::vpiFullName),
                expected: "module",
                actual: format!("vpiType {}", o.get(sys::vpiType)),
            }),
        }
    }
}

/// A module/scope handle. Port of cocotb's HierarchyObject
/// (cocotb: handle.py), with `Result`-returning lookup (mapping row 19).
#[derive(Copy, Clone)]
pub struct HierarchyHandle {
    h: ObjHandle,
}

impl HierarchyHandle {
    /// A handle to nothing, for unit tests that need a `RustdvCtx` but never
    /// touch the DUT. Any VPI call through it goes to `rustdv-vpi-stubs`,
    /// which panics — so a test that *does* touch the DUT fails loudly
    /// instead of reading garbage.
    pub fn null_for_test() -> HierarchyHandle {
        HierarchyHandle {
            h: ObjHandle(std::ptr::null_mut()),
        }
    }

    /// Dynamic child lookup: `dut.child("clk")?` (design-doc OQ-6 lean).
    pub fn child(&self, name: &str) -> Result<AnyHandle, HandleError> {
        let cname = CString::new(name).expect("NUL in signal name");
        let h = unsafe { sys::vpi_handle_by_name(cname.as_ptr(), self.h.0) };
        match ObjHandle::new(h) {
            Some(h) => Ok(AnyHandle::classify(h)),
            None => Err(HandleError::NotFound {
                name: name.into(),
                scope: self.full_name(),
            }),
        }
    }

    /// Shorthand: child that must be a signal.
    pub fn signal(&self, name: &str) -> Result<LogicHandle, HandleError> {
        self.child(name)?.as_logic()
    }

    pub fn name(&self) -> String {
        self.h.get_str(sys::vpiName)
    }
    pub fn full_name(&self) -> String {
        self.h.get_str(sys::vpiFullName)
    }

    /// Iterate child objects (modules, nets, regs) — serves the
    /// `visit_children` debug-print role at the DUT level.
    pub fn children(&self) -> Vec<AnyHandle> {
        let mut out = Vec::new();
        for t in [sys::vpiModule, sys::vpiNet, sys::vpiReg] {
            unsafe {
                let it = sys::vpi_iterate(t, self.h.0);
                if it.is_null() {
                    continue;
                }
                loop {
                    let c = sys::vpi_scan(it);
                    if c.is_null() {
                        break; // scan returning NULL frees the iterator
                    }
                    if let Some(h) = ObjHandle::new(c) {
                        out.push(AnyHandle::classify(h));
                    }
                }
            }
        }
        out
    }
}

/// A value-bearing signal handle (net/reg/var). Port of cocotb's
/// LogicObject surface: explicit `get()`/`set()` (mapping row 20).
/// Buffered-write semantics live a layer up in rustdv-sim; the methods
/// here apply values immediately.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct LogicHandle {
    h: ObjHandle,
    width: u32,
}

impl LogicHandle {
    pub fn name(&self) -> String {
        self.h.get_str(sys::vpiName)
    }
    pub fn full_name(&self) -> String {
        self.h.get_str(sys::vpiFullName)
    }
    pub fn size(&self) -> u32 {
        self.width
    }

    /// Current value as a slice of VPI vector words. `Err` if the simulator
    /// did not supply a vector representation (design-doc §0.6: conversion
    /// failures are Results, not exceptions). Handles `unsafe` and null pointer
    /// checks internally.
    fn get_vpi_words(&self) -> Result<&[sys::t_vpi_vecval], ValueError> {
        let mut val = sys::t_vpi_value {
            format: sys::vpiVectorVal,
            value: sys::u_vpi_value_union {
                vector: std::ptr::null_mut(),
            },
        };
        unsafe {
            sys::vpi_get_value(self.h.0, &mut val);
            let words = val.value.vector;
            if words.is_null() {
                return Err(ValueError::Unavailable);
            }
            let word_count = self.size().div_ceil(32) as usize;
            Ok(std::slice::from_raw_parts(words, word_count))
        }
    }

    /// Masks one VPI vector word to the signal width. `Err` if any used bit is
    /// x/z (design-doc §0.6: conversion failures are Results, not exceptions).
    ///
    /// # Arguments
    ///
    /// * `word` - Individual VPI vector word from [`Self::get_vpi_words`].
    /// * `width` - Width of the entire signal in bits.
    /// * `last` - Whether `word` is the signal's final VPI vector word.
    fn get_vpi_word(&self, word: &t_vpi_vecval, width: u32, last: bool) -> Result<u32, ValueError> {
        let used_bits = if last && !width.is_multiple_of(32) {
            width % 32
        } else {
            32
        };
        let mask = if used_bits == 32 {
            u32::MAX
        } else {
            (1u32 << used_bits) - 1
        };
        if word.bval & mask != 0 {
            return Err(ValueError::FourState(self.get_logic()?.to_binstr()));
        }
        Ok(word.aval & mask)
    }

    /// Check that the signal width is at most `want` bits. Returns `Err` if
    /// the signal is wider than `want` (design-doc §0.6: conversion failures
    /// are Results, not exceptions).
    fn check_size(&self, want: u32) -> Result<(), ValueError> {
        if self.size() > want {
            return Err(ValueError::Width {
                want,
                have: self.size() as usize,
            });
        }
        Ok(())
    }

    /// Current value as bool; `Err` if the signal is wider than one bit or its
    /// value is x/z (design-doc §0.6: conversion failures are Results, not
    /// exceptions).
    pub fn get_bool(&self) -> Result<bool, ValueError> {
        self.check_size(1)?;
        Ok(self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)? != 0)
    }

    /// Current value as u8; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u8(&self) -> Result<u8, ValueError> {
        self.check_size(u8::BITS)?;
        Ok(self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)? as u8)
    }

    /// Current value as u16; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u16(&self) -> Result<u16, ValueError> {
        self.check_size(u16::BITS)?;
        Ok(self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)? as u16)
    }

    /// Current value as u32; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u32(&self) -> Result<u32, ValueError> {
        self.check_size(u32::BITS)?;
        self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)
    }

    /// Current value as u64; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u64(&self) -> Result<u64, ValueError> {
        self.check_size(u64::BITS)?;
        let words = self.get_vpi_words()?;
        let mut result = 0;
        for (index, word) in words.iter().enumerate() {
            let word = self.get_vpi_word(word, self.size(), index + 1 == words.len())?;
            result |= (word as u64) << (index * 32);
        }
        Ok(result)
    }

    /// Current value as u128; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u128(&self) -> Result<u128, ValueError> {
        self.check_size(u128::BITS)?;
        let words = self.get_vpi_words()?;
        let mut result = 0;
        for (index, word) in words.iter().enumerate() {
            let word = self.get_vpi_word(word, self.size(), index + 1 == words.len())?;
            result |= (word as u128) << (index * 32);
        }
        Ok(result)
    }

    /// Current value as an unsigned arbitrary-precision integer; `Err` if
    /// any bit is x/z.
    pub fn get_bigint(&self) -> Result<BigUint, ValueError> {
        let words = self.get_vpi_words()?;
        let mut digits = Vec::with_capacity(words.len());
        for (index, word) in words.iter().enumerate() {
            digits.push(self.get_vpi_word(word, self.size(), index + 1 == words.len())?);
        }
        Ok(BigUint::from_slice(&digits))
    }

    /// Current value as a LogicArray (4-state). `Err` if the simulator does
    /// not supply a VPI vector representation.
    pub fn get_logic(&self) -> Result<LogicArray, ValueError> {
        let words = self.get_vpi_words()?;
        let words: Vec<(u32, u32)> = words.iter().map(|word| (word.aval, word.bval)).collect();
        Ok(LogicArray::from_vpi_words(&words, self.size() as usize))
    }

    /// Mask unused bits in the final VPI vector word to the signal width.
    fn mask_vector_words(&self, words: &mut [sys::t_vpi_vecval]) {
        if !self.size().is_multiple_of(32) {
            let mask = (1u32 << (self.size() % 32)) - 1;
            let last = words.last_mut().expect("partial width has a vector word");
            last.aval &= mask;
            last.bval &= mask;
        }
    }

    /// Resize vector words to the signal width and pass them to
    /// `vpi_put_value`. Handles unsafe usage internally, i.e. words can be any
    /// length and the vpi call is still safe.
    fn set_vector_words(&self, words: &mut [sys::t_vpi_vecval], flags: i32) {
        let word_count = self.size().div_ceil(32) as usize;
        if words.len() == word_count {
            // Word length matches, just mask the final word to the signal width
            self.mask_vector_words(words);
            unsafe {
                let mut vpi_value = sys::t_vpi_value {
                    format: sys::vpiVectorVal,
                    value: sys::u_vpi_value_union {
                        vector: words.as_mut_ptr(),
                    },
                };
                sys::vpi_put_value(self.h.0, &mut vpi_value, std::ptr::null_mut(), flags);
            }
        } else if words.len() > word_count {
            // Word length is greater, truncate to the signal width and mask the
            // final word
            let words = &mut words[..word_count];
            self.mask_vector_words(words);
            unsafe {
                let mut vpi_value = sys::t_vpi_value {
                    format: sys::vpiVectorVal,
                    value: sys::u_vpi_value_union {
                        vector: words.as_mut_ptr(),
                    },
                };
                sys::vpi_put_value(self.h.0, &mut vpi_value, std::ptr::null_mut(), flags);
            }
        } else {
            // Word length is less, pad with zeros to the signal width
            let mut words = words.to_vec();
            words.resize(word_count, sys::t_vpi_vecval::default());
            unsafe {
                let mut vpi_value = sys::t_vpi_value {
                    format: sys::vpiVectorVal,
                    value: sys::u_vpi_value_union {
                        vector: words.as_mut_ptr(),
                    },
                };
                sys::vpi_put_value(self.h.0, &mut vpi_value, std::ptr::null_mut(), flags);
            }
        }
    }

    /// Write a bool with the supplied VPI flag, zero-extended to the signal
    /// width.
    pub fn set_bool(&self, v: bool, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a bool.
    pub fn set_bool_now(&self, v: bool) {
        self.set_bool(v, sys::vpiNoDelay);
    }

    /// Write a u8 with the supplied VPI flag, zero-extended / truncated to the
    /// signal width.
    pub fn set_u8(&self, v: u8, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u8.
    pub fn set_u8_now(&self, v: u8) {
        self.set_u8(v, sys::vpiNoDelay);
    }

    /// Write a u16 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u16(&self, v: u16, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u16.
    pub fn set_u16_now(&self, v: u16) {
        self.set_u16(v, sys::vpiNoDelay);
    }

    /// Write a u32 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u32(&self, v: u32, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u32.
    pub fn set_u32_now(&self, v: u32) {
        self.set_u32(v, sys::vpiNoDelay);
    }

    /// Write a u64 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u64(&self, v: u64, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u64.
    pub fn set_u64_now(&self, v: u64) {
        self.set_u64(v, sys::vpiNoDelay);
    }

    /// Write a u128 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u128(&self, v: u128, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u128.
    pub fn set_u128_now(&self, v: u128) {
        self.set_u128(v, sys::vpiNoDelay);
    }

    /// Write an arbitrary-precision integer with the supplied VPI flag,
    /// zero-extended / truncated to the signal width.
    pub fn set_bigint(&self, v: &BigUint, flags: i32) {
        let mut words: Vec<sys::t_vpi_vecval> = v
            .iter_u32_digits()
            .map(|aval| sys::t_vpi_vecval { aval, bval: 0 })
            .collect();
        self.set_vector_words(&mut words, flags);
    }

    /// Immediate (NoDelay) write of an arbitrary-precision integer.
    pub fn set_bigint_now(&self, v: &BigUint) {
        self.set_bigint(v, sys::vpiNoDelay);
    }

    /// Write a 4-state value with the supplied VPI flag.
    pub fn set_logic(&self, v: &LogicArray, flags: i32) {
        let mut words: Vec<sys::t_vpi_vecval> = v
            .to_vpi_words()
            .into_iter()
            .map(|(aval, bval)| sys::t_vpi_vecval { aval, bval })
            .collect();
        self.set_vector_words(&mut words, flags);
    }

    /// Immediate (NoDelay) write of a 4-state value.
    pub fn set_logic_now(&self, v: &LogicArray) {
        self.set_logic(v, sys::vpiNoDelay);
    }
}

/// All top-level modules in the design.
pub fn top_modules() -> Vec<HierarchyHandle> {
    let mut out = Vec::new();
    unsafe {
        let it = sys::vpi_iterate(sys::vpiModule, std::ptr::null_mut());
        if it.is_null() {
            return out;
        }
        loop {
            let m = sys::vpi_scan(it);
            if m.is_null() {
                break;
            }
            if let Some(h) = ObjHandle::new(m) {
                out.push(HierarchyHandle { h });
            }
        }
    }
    out
}

/// The first top-level module (the DUT in single-top designs).
pub fn top_module() -> Result<HierarchyHandle, HandleError> {
    top_modules()
        .into_iter()
        .next()
        .ok_or(HandleError::NoTopModule)
}

// ===========================================================================
// Time
// ===========================================================================

/// Current simulation time in simulator precision steps.
pub fn sim_time_steps() -> u64 {
    let mut t = sys::t_vpi_time {
        type_: sys::vpiSimTime,
        high: 0,
        low: 0,
        real: 0.0,
    };
    unsafe { sys::vpi_get_time(std::ptr::null_mut(), &mut t) };
    ((t.high as u64) << 32) | (t.low as u64)
}

/// Simulator time precision as a power of ten (e.g. -9 = 1 ns).
pub fn time_precision() -> i32 {
    thread_local! {
        static PREC: Cell<Option<i32>> = const { Cell::new(None) };
    }
    PREC.with(|p| match p.get() {
        Some(v) => v,
        None => {
            let v = unsafe { sys::vpi_get(sys::vpiTimePrecision, std::ptr::null_mut()) };
            p.set(Some(v));
            v
        }
    })
}

/// End the simulation (vpi_control(vpiFinish)).
pub fn finish() {
    unsafe {
        sys::vpi_control(sys::vpiFinish, 0i32);
    }
}

// ===========================================================================
// Panic sink (invariant 4)
// ===========================================================================

type PanicSink = RefCell<Option<Box<dyn Fn(String)>>>;

thread_local! {
    static PANIC_SINK: PanicSink = const { RefCell::new(None) };
}

/// Install the handler invoked when a callback closure panics (the runner
/// routes this to "fail the current test", mirroring cocotb catching
/// BaseException per task).
pub fn set_panic_sink(f: Box<dyn Fn(String)>) {
    PANIC_SINK.with(|s| *s.borrow_mut() = Some(f));
}

fn report_panic(payload: Box<dyn std::any::Any + Send>) {
    let msg = if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic (non-string payload)".to_string()
    };
    PANIC_SINK.with(|s| {
        if let Some(f) = s.borrow().as_ref() {
            f(msg.clone());
        } else {
            eprintln!("rustdv: panic in simulator callback: {msg}");
        }
    });
}

// ===========================================================================
// Callbacks (invariant 5)
// ===========================================================================

enum CbKind {
    OneShot,
    Recurring,
}

struct CbShared {
    kind: CbKind,
    /// Registration handle returned by vpi_register_cb. One-shots remove it
    /// from inside the trampoline, while it is valid on both supported
    /// simulators: Icarus reaps the active callback after it returns and
    /// Verilator releases its separately-owned handle object immediately.
    vpi_h: Cell<sys::vpiHandle>,
    /// True once the C-side Rc reference has been reclaimed (fired one-shot
    /// or removed callback). Guards against double-free.
    released: Cell<bool>,
    once: RefCell<Option<Box<dyn FnOnce()>>>,
    repeat: RefCell<Option<Box<dyn FnMut()>>>,
}

/// RAII callback registration. Dropping an unfired/live handle removes the
/// simulator callback — this is what makes drop-based task cancellation
/// (design-doc §4.6) clean up trigger registrations for free.
pub struct CallbackHandle {
    shared: Rc<CbShared>,
    raw: *const CbShared,
    detached: bool,
}

impl CallbackHandle {
    /// Detach Rust ownership without leaking it. A detached one-shot keeps its
    /// C-side reference until it fires and then self-cleans; a detached
    /// recurring callback lives for the rest of the simulation.
    pub fn forget(mut self) {
        self.detached = true;
    }
}

impl Drop for CallbackHandle {
    fn drop(&mut self) {
        if self.detached {
            return;
        }
        if !self.shared.released.get() {
            self.shared.released.set(true);
            unsafe {
                sys::vpi_remove_cb(self.shared.vpi_h.get());
                // Reclaim the C-side reference.
                drop(Rc::from_raw(self.raw));
            }
        }
    }
}

extern "C" fn trampoline(cb: *mut sys::t_cb_data) -> i32 {
    unsafe {
        let ud = (*cb).user_data as *const CbShared;
        if ud.is_null() {
            return 0;
        }
        // Hold our own reference for the duration of the call so that
        // closures dropping the CallbackHandle can't free us mid-flight.
        Rc::increment_strong_count(ud);
        let shared: Rc<CbShared> = Rc::from_raw(ud);
        match shared.kind {
            CbKind::OneShot => {
                if !shared.released.get() {
                    shared.released.set(true);
                    let f = shared.once.borrow_mut().take();
                    // The returned callback handle has different post-fire
                    // ownership across simulators. Remove it while the active
                    // callback is still valid on both: Icarus marks it for
                    // self-reaping, while Verilator deletes the retained
                    // VerilatedVpioReasonCb handle.
                    sys::vpi_remove_cb(shared.vpi_h.get());
                    // Reclaim the C-side reference before running user code.
                    drop(Rc::from_raw(ud));
                    if let Some(f) = f
                        && let Err(p) = catch_unwind(AssertUnwindSafe(f))
                    {
                        report_panic(p);
                    }
                }
            }
            CbKind::Recurring => {
                let mut guard = shared.repeat.borrow_mut();
                if let Some(f) = guard.as_mut()
                    && let Err(p) = catch_unwind(AssertUnwindSafe(f))
                {
                    report_panic(p);
                }
            }
        }
        drop(shared);
    }
    0
}

fn register(
    kind: CbKind,
    once: Option<Box<dyn FnOnce()>>,
    repeat: Option<Box<dyn FnMut()>>,
    reason: i32,
    obj: sys::vpiHandle,
    time: Option<sys::t_vpi_time>,
) -> CallbackHandle {
    let shared = Rc::new(CbShared {
        kind,
        vpi_h: Cell::new(std::ptr::null_mut()),
        released: Cell::new(false),
        once: RefCell::new(once),
        repeat: RefCell::new(repeat),
    });
    // C-side reference:
    let raw = Rc::into_raw(shared.clone());

    let mut t = time.unwrap_or(sys::t_vpi_time {
        type_: sys::vpiSuppressTime,
        high: 0,
        low: 0,
        real: 0.0,
    });
    // value: NULL — closures read signal values themselves; Icarus rejects
    // vpiSuppressVal on value-change callbacks ("format 10 not supported").
    let mut cb = sys::t_cb_data {
        reason,
        cb_rtn: Some(trampoline),
        obj,
        time: &mut t,
        value: std::ptr::null_mut(),
        index: 0,
        user_data: raw as *mut _,
    };
    let vpi_h = unsafe { sys::vpi_register_cb(&mut cb) };
    assert!(!vpi_h.is_null(), "vpi_register_cb failed (reason {reason})");
    shared.vpi_h.set(vpi_h);
    CallbackHandle {
        shared,
        raw,
        detached: false,
    }
}

fn simtime(steps: u64) -> sys::t_vpi_time {
    sys::t_vpi_time {
        type_: sys::vpiSimTime,
        high: (steps >> 32) as u32,
        low: (steps & 0xFFFF_FFFF) as u32,
        real: 0.0,
    }
}

/// One-shot callback after `steps` precision units (cbAfterDelay).
pub fn register_timer(steps: u64, f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbAfterDelay,
        std::ptr::null_mut(),
        Some(simtime(steps)),
    )
}

/// Recurring callback on any value change of `sig` (cbValueChange). The
/// closure reads the signal itself; edge filtering happens in rustdv-sim.
pub fn register_value_change(sig: LogicHandle, f: Box<dyn FnMut()>) -> CallbackHandle {
    register(
        CbKind::Recurring,
        None,
        Some(f),
        sys::cbValueChange,
        sig.h.0,
        Some(simtime(0)),
    )
}

/// One-shot callback at the next ReadWrite synch point.
pub fn register_read_write(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbReadWriteSynch,
        std::ptr::null_mut(),
        Some(simtime(0)),
    )
}

/// One-shot callback at the next ReadOnly synch point.
pub fn register_read_only(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbReadOnlySynch,
        std::ptr::null_mut(),
        Some(simtime(0)),
    )
}

/// One-shot callback at the next simulation time step.
pub fn register_next_sim_time(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbNextSimTime,
        std::ptr::null_mut(),
        None,
    )
}

/// One-shot callback at the end of the current simulation time step.
pub fn register_at_end_of_sim_time(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbAtEndOfSimTime,
        std::ptr::null_mut(),
        Some(simtime(sim_time_steps())),
    )
}

/// One-shot callback at start of simulation (the bootstrap hook).
pub fn register_start_of_simulation(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbStartOfSimulation,
        std::ptr::null_mut(),
        None,
    )
}

/// One-shot callback at end of simulation.
pub fn register_end_of_simulation(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbEndOfSimulation,
        std::ptr::null_mut(),
        None,
    )
}

#[cfg(test)]
mod vpi_word_tests {
    use super::*;

    fn assert_single_word<T: ToVpiWords<1>>(value: T, expected: u32) {
        let words = value.to_vpi_words();
        assert_eq!(words[0].aval, expected);
        assert_eq!(words[0].bval, 0);
    }

    #[test]
    fn scalar_values_convert_to_one_vpi_word() {
        assert_single_word(false, 0);
        assert_single_word(true, 1);
        assert_single_word(0xa5u8, 0xa5);
        assert_single_word(0xa5b6u16, 0xa5b6);
        assert_single_word(0xa5b6_c7d8u32, 0xa5b6_c7d8);
    }

    #[test]
    fn wide_values_convert_in_least_significant_word_first_order() {
        let words = 0x0123_4567_89ab_cdefu64.to_vpi_words();
        assert_eq!(words.map(|word| word.aval), [0x89ab_cdef, 0x0123_4567]);
        assert!(words.iter().all(|word| word.bval == 0));

        let words = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210u128.to_vpi_words();
        assert_eq!(
            words.map(|word| word.aval),
            [0x7654_3210, 0xfedc_ba98, 0x89ab_cdef, 0x0123_4567]
        );
        assert!(words.iter().all(|word| word.bval == 0));
    }
}

#[cfg(test)]
mod callback_tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn fired_one_shot_releases_its_vpi_handle() {
        rustdv_vpi_stubs::reset_callbacks();
        let fired = Rc::new(Cell::new(false));
        let fired_in_callback = fired.clone();
        let handle = register_timer(1, Box::new(move || fired_in_callback.set(true)));

        assert_eq!(rustdv_vpi_stubs::live_callback_handles(), 1);
        rustdv_vpi_stubs::fire_next_callback();
        assert!(fired.get());
        drop(handle);

        assert_eq!(rustdv_vpi_stubs::live_callback_handles(), 0);
        rustdv_vpi_stubs::reset_callbacks();
    }

    #[test]
    fn detached_one_shot_releases_shared_state_after_firing() {
        rustdv_vpi_stubs::reset_callbacks();
        let handle = register_timer(1, Box::new(|| {}));
        let shared = Rc::downgrade(&handle.shared);

        handle.forget();
        assert!(shared.upgrade().is_some());
        rustdv_vpi_stubs::fire_next_callback();

        assert_eq!(rustdv_vpi_stubs::live_callback_handles(), 0);
        assert!(shared.upgrade().is_none());
        rustdv_vpi_stubs::reset_callbacks();
    }

    #[test]
    fn callback_stub_refuses_to_reset_a_live_handle() {
        rustdv_vpi_stubs::reset_callbacks();
        let handle = register_timer(1, Box::new(|| {}));

        let reset = catch_unwind(AssertUnwindSafe(rustdv_vpi_stubs::reset_callbacks));
        assert!(reset.is_err());

        drop(handle);
        rustdv_vpi_stubs::reset_callbacks();
    }
}

#[cfg(test)]
mod handle_tests {
    use super::*;

    fn make_signal(width: i32, words: &[sys::t_vpi_vecval], binstr: &str) -> LogicHandle {
        rustdv_vpi_stubs::configure_signal(width, words, binstr);
        let raw = 1usize as sys::vpiHandle;
        let AnyHandle::Logic(signal) = AnyHandle::classify(ObjHandle(raw)) else {
            panic!("stub object was not classified as a signal");
        };
        signal
    }

    #[test]
    fn signal_width_is_cached_when_the_handle_is_classified() {
        rustdv_vpi_stubs::reset_property_gets();
        let signal = make_signal(37, &[sys::t_vpi_vecval::default(); 2], &"0".repeat(37));

        assert_eq!(signal.size(), 37);
        assert_eq!(signal.size(), 37);
        assert_eq!(rustdv_vpi_stubs::property_get_count(sys::vpiType), 1);
        assert_eq!(rustdv_vpi_stubs::property_get_count(sys::vpiSize), 1);
        rustdv_vpi_stubs::reset_property_gets();
    }

    #[test]
    fn vector_read_uses_vpi_word_order_and_masks_unused_bits() {
        let signal = make_signal(
            37,
            &[
                sys::t_vpi_vecval {
                    aval: 0x89ab_cdef,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: 0xffff_fff5,
                    bval: 0xffff_ffe0,
                },
            ],
            "101011001101010111100110111101111",
        );

        assert_eq!(signal.get_u64(), Ok(0x0000_0015_89ab_cdef));
    }

    #[test]
    fn vector_read_reports_xz_in_used_bits() {
        let signal = make_signal(
            8,
            &[sys::t_vpi_vecval {
                aval: 0x5e,
                bval: 0x04,
            }],
            "01011x10",
        );

        assert_eq!(
            signal.get_u64(),
            Err(ValueError::FourState("01011x10".to_owned()))
        );
    }

    #[test]
    fn logic_array_read_uses_vpi_vector_words() {
        let signal = make_signal(
            4,
            &[sys::t_vpi_vecval {
                aval: 0b0110,
                bval: 0b0011,
            }],
            "0000",
        );

        assert_eq!(signal.get_logic().unwrap().to_binstr(), "01xz");
    }

    #[test]
    fn vector_read_returns_numeric_zero_from_a_vector_word() {
        let signal = make_signal(8, &[sys::t_vpi_vecval { aval: 0, bval: 0 }], "00000000");

        assert_eq!(signal.get_u64(), Ok(0));
    }

    #[test]
    fn bool_read_converts_single_bit_values() {
        let low = make_signal(1, &[sys::t_vpi_vecval { aval: 0, bval: 0 }], "0");
        assert_eq!(low.get_bool(), Ok(false));

        let high = make_signal(1, &[sys::t_vpi_vecval { aval: 1, bval: 0 }], "1");
        assert_eq!(high.get_bool(), Ok(true));
    }

    #[test]
    fn bool_read_rejects_wide_and_four_state_values() {
        let wide = make_signal(2, &[sys::t_vpi_vecval { aval: 1, bval: 0 }], "01");
        assert_eq!(wide.get_bool(), Err(ValueError::Width { want: 1, have: 2 }));

        let unknown = make_signal(1, &[sys::t_vpi_vecval { aval: 1, bval: 1 }], "x");
        assert_eq!(
            unknown.get_bool(),
            Err(ValueError::FourState("x".to_owned()))
        );
    }

    #[test]
    fn vector_read_reports_an_unavailable_vpi_result() {
        let signal = make_signal(8, &[sys::t_vpi_vecval { aval: 0, bval: 0 }], "00000000");
        rustdv_vpi_stubs::make_vector_value_unavailable();

        assert_eq!(signal.get_u64(), Err(ValueError::Unavailable));
        assert_eq!(signal.get_bigint(), Err(ValueError::Unavailable));
        assert_eq!(signal.get_logic(), Err(ValueError::Unavailable));
    }

    #[test]
    fn bigint_read_supports_values_wider_than_u64() {
        let signal = make_signal(
            70,
            &[
                sys::t_vpi_vecval {
                    aval: 0x89ab_cdef,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: 0x0123_4567,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: 0xffff_ffea,
                    bval: 0xffff_ffc0,
                },
            ],
            "1010100000000100100011010001010110011110001001101010111100110111101111",
        );

        assert_eq!(
            signal.get_bigint().map(|value| value.to_str_radix(16)),
            Ok("2a0123456789abcdef".to_owned())
        );
    }

    #[test]
    fn bigint_read_reports_xz_in_used_bits() {
        let signal = make_signal(
            65,
            &[
                sys::t_vpi_vecval::default(),
                sys::t_vpi_vecval::default(),
                sys::t_vpi_vecval { aval: 1, bval: 1 },
            ],
            &format!("x{}", "0".repeat(64)),
        );

        assert_eq!(
            signal.get_bigint(),
            Err(ValueError::FourState(format!("x{}", "0".repeat(64))))
        );
    }

    #[test]
    fn vector_write_truncates_and_zero_extends_to_signal_width() {
        let narrow = make_signal(8, &[sys::t_vpi_vecval::default()], "00000000");
        narrow.set_u64_now(0xffff_ffff_ffff_ffa5);
        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].aval, 0xa5);
        assert_eq!(words[0].bval, 0);

        let signal = make_signal(37, &[sys::t_vpi_vecval::default(); 2], &"0".repeat(37));
        signal.set_u64_now(0xffff_fff5_89ab_cdef);
        let words = rustdv_vpi_stubs::last_put_vector();

        assert_eq!(words.len(), 2);
        assert_eq!(words[0].aval, 0x89ab_cdef);
        assert_eq!(words[1].aval, 0x15);
        assert_eq!(words[0].bval, 0);
        assert_eq!(words[1].bval, 0);

        let wide = make_signal(70, &[sys::t_vpi_vecval::default(); 3], &"0".repeat(70));
        wide.set_u64_now(0x0123_4567_89ab_cdef);
        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].aval, 0x89ab_cdef);
        assert_eq!(words[1].aval, 0x0123_4567);
        assert_eq!(words[2].aval, 0);
    }

    #[test]
    fn typed_integer_setters_write_vpi_words() {
        let signal = make_signal(1, &[sys::t_vpi_vecval::default()], "0");
        signal.set_bool_now(true);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 1);

        let signal = make_signal(8, &[sys::t_vpi_vecval::default()], "00000000");
        signal.set_u8(0xa5, sys::vpiNoDelay);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 0xa5);

        let signal = make_signal(16, &[sys::t_vpi_vecval::default()], &"0".repeat(16));
        signal.set_u16_now(0xa5b6);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 0xa5b6);

        let signal = make_signal(32, &[sys::t_vpi_vecval::default()], &"0".repeat(32));
        signal.set_u32_now(0xa5b6_c7d8);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 0xa5b6_c7d8);

        let signal = make_signal(128, &[sys::t_vpi_vecval::default(); 4], &"0".repeat(128));
        signal.set_u128_now(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(
            words.iter().map(|word| word.aval).collect::<Vec<_>>(),
            [0x7654_3210, 0xfedc_ba98, 0x89ab_cdef, 0x0123_4567]
        );
        assert!(words.iter().all(|word| word.bval == 0));
    }

    #[test]
    fn bigint_setter_writes_and_zero_extends_vpi_words() {
        let signal = make_signal(192, &[sys::t_vpi_vecval::default(); 6], &"0".repeat(192));
        let value = BigUint::from_slice(&[
            0x7654_3210,
            0xfedc_ba98,
            0x89ab_cdef,
            0x0123_4567,
            0xa5a5_5a5a,
        ]);

        signal.set_bigint_now(&value);

        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(
            words.iter().map(|word| word.aval).collect::<Vec<_>>(),
            [
                0x7654_3210,
                0xfedc_ba98,
                0x89ab_cdef,
                0x0123_4567,
                0xa5a5_5a5a,
                0,
            ]
        );
        assert!(words.iter().all(|word| word.bval == 0));
    }

    #[test]
    fn vector_word_mask_clears_unused_aval_and_bval_bits() {
        let signal = make_signal(37, &[sys::t_vpi_vecval::default(); 2], &"0".repeat(37));
        let mut words = [
            sys::t_vpi_vecval {
                aval: u32::MAX,
                bval: u32::MAX,
            },
            sys::t_vpi_vecval {
                aval: u32::MAX,
                bval: u32::MAX,
            },
        ];

        signal.mask_vector_words(&mut words);

        assert_eq!(words[0].aval, u32::MAX);
        assert_eq!(words[0].bval, u32::MAX);
        assert_eq!(words[1].aval, 0x1f);
        assert_eq!(words[1].bval, 0x1f);
    }

    #[test]
    fn logic_array_write_uses_vpi_vector_words() {
        let signal = make_signal(
            4,
            &[sys::t_vpi_vecval::default()],
            "binary-string path must not be used",
        );
        signal.set_logic_now(&LogicArray::from_binstr("01xz"));

        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].aval, 0b0110);
        assert_eq!(words[0].bval, 0b0011);
    }
}
