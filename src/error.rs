//! The POSIX `errno` slot.
//!
//! The C ABI reports failure the way the C library does: a sentinel return (`-1`, a
//! null pointer, or an `ERANGE`-style code) with the reason left in `errno`. This is
//! the per-thread errno setter/getter shared by every C-ABI entry point. The
//! mapping from a library's internal error type to an `errno` value is
//! domain-specific and lives in each consuming crate, not here.

/// Set the calling thread's `errno`.
///
/// `__errno_location` is the per-thread errno slot exposed by both glibc and musl —
/// our two link targets — so this is correct under multithreading.
#[inline]
pub fn set_errno(code: libc::c_int) {
    // SAFETY: `__errno_location` always returns a valid, thread-local `int*`.
    unsafe {
        *libc::__errno_location() = code;
    }
}

/// Read the calling thread's current `errno`.
///
/// Used to tell a soft failure libc has already encoded (e.g. an `EACCES` denial or
/// an `ERANGE` short buffer) apart from a hard error, after libc has translated the
/// kernel's `-errno` return.
#[inline]
pub fn get_errno() -> libc::c_int {
    // SAFETY: `__errno_location` always returns a valid, thread-local `int*`.
    unsafe { *libc::__errno_location() }
}
