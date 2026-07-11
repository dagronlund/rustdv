//! Stub definitions of the VPI entry points for **test executables**.
//!
//! Unit-test binaries (design-doc §7.3 convention 5: pure-Rust tests, no
//! simulator) link the whole crate graph, including `rustdv-gpi-sys`'s
//! `extern "C"` declarations. Executables, unlike cdylibs, cannot carry
//! undefined symbols — so tests pull these panicking definitions in as a
//! dev-dependency:
//!
//! ```ignore
//! #[cfg(test)]
//! use rustdv_vpi_stubs as _;
//! ```
//!
//! The `.vpi` module never links this crate; there the real symbols come
//! from the simulator process (vvp) at load time.
//!
//! Signatures only need matching *names* for the linker; none of these may
//! ever be called (variadics are deliberately simplified).

#![allow(non_snake_case, clippy::missing_safety_doc)]

use std::os::raw::c_void;

macro_rules! stub {
    ($($name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty;)*) => {
        $(
            #[no_mangle]
            pub extern "C" fn $name($(_: $ty),*) -> $ret {
                panic!(concat!(
                    stringify!($name),
                    " called outside a simulator (rustdv-vpi-stubs is for unit tests only)"
                ));
            }
        )*
    };
}

stub! {
    vpi_handle_by_name(a: *const i8, b: *mut c_void) -> *mut c_void;
    vpi_handle_by_index(a: *mut c_void, b: i32) -> *mut c_void;
    vpi_iterate(a: i32, b: *mut c_void) -> *mut c_void;
    vpi_scan(a: *mut c_void) -> *mut c_void;
    vpi_get(a: i32, b: *mut c_void) -> i32;
    vpi_get_str(a: i32, b: *mut c_void) -> *mut i8;
    vpi_get_value(a: *mut c_void, b: *mut c_void) -> ();
    vpi_put_value(a: *mut c_void, b: *mut c_void, c: *mut c_void, d: i32) -> *mut c_void;
    vpi_get_time(a: *mut c_void, b: *mut c_void) -> ();
    vpi_register_cb(a: *mut c_void) -> *mut c_void;
    vpi_remove_cb(a: *mut c_void) -> i32;
    vpi_free_object(a: *mut c_void) -> i32;
    vpi_control(a: i32) -> i32;
    vpi_printf(a: *const i8) -> i32;
}
