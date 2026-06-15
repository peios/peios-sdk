//! Shared C-ABI plumbing: the getxattr-style copy-out contract and a small
//! stack formatter.
//!
//! Every length-returning entry point (`ssize_t`) in a Peios C-ABI library funnels
//! its output through here, so the two-call / probe / `ERANGE` semantics promised in
//! the headers are implemented in exactly one place.

use core::alloc::Layout;
use core::ffi::{c_char, c_int};

use alloc::vec::Vec;

use crate::error::set_errno;

/// Copy `src` into a caller buffer, getxattr-style, and return its length.
///
/// - `cap == 0` (a NULL buffer is allowed here): size probe — return
///   `src.len()`, write nothing.
/// - `cap != 0` but `out` is NULL: caller error — `EINVAL`, return `-1`.
/// - `cap < src.len()`: the buffer is present but too small — `ERANGE`, write
///   nothing (never a truncated result), return `-1`.
/// - otherwise: copy all of `src` and return its length.
///
/// # Safety
/// `out` must be valid for `cap` bytes when `cap != 0`.
pub unsafe fn emit_bytes(src: &[u8], out: *mut u8, cap: usize) -> isize {
    let need = src.len();
    if cap == 0 {
        return need as isize; // probe
    }
    if out.is_null() {
        set_errno(libc::EINVAL);
        return -1;
    }
    if cap < need {
        set_errno(libc::ERANGE);
        return -1;
    }
    core::ptr::copy_nonoverlapping(src.as_ptr(), out, need);
    need as isize
}

/// Copy a string into a caller buffer with a trailing NUL, and return its length
/// *excluding* the NUL.
///
/// Mirrors [`emit_bytes`] but accounts for the terminator: a real write needs
/// `cap >= s.len() + 1`, while a `cap == 0` probe returns `s.len()` (so the
/// caller allocates `s.len() + 1`).
///
/// # Safety
/// `out` must be valid for `cap` bytes when `cap != 0`.
pub unsafe fn emit_str(s: &[u8], out: *mut c_char, cap: usize) -> isize {
    let need = s.len(); // excluding the NUL
    if cap == 0 {
        return need as isize; // probe
    }
    if out.is_null() {
        set_errno(libc::EINVAL);
        return -1;
    }
    if cap < need + 1 {
        set_errno(libc::ERANGE);
        return -1;
    }
    let dst = out as *mut u8;
    core::ptr::copy_nonoverlapping(s.as_ptr(), dst, need);
    *dst.add(need) = 0; // NUL terminator
    need as isize
}

/// A `core::fmt::Write` sink backed by a fixed stack buffer.
///
/// Lets us render a `Display` value (e.g. a SID's SDDL string) without touching
/// the allocator. The buffer is generously sized for the longest possible SID
/// string (`S-1-<48-bit auth>` plus 15 ten-digit sub-authorities ~= 184 bytes);
/// an overrun latches and surfaces as a `core::fmt::Error` to the caller.
pub struct StackWriter {
    buf: [u8; Self::CAP],
    len: usize,
}

impl StackWriter {
    const CAP: usize = 256;

    pub fn new() -> Self {
        Self {
            buf: [0; Self::CAP],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl core::fmt::Write for StackWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let b = s.as_bytes();
        let end = self.len.checked_add(b.len()).ok_or(core::fmt::Error)?;
        if end > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(b);
        self.len = end;
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// Heap-backed builder plumbing
// ----------------------------------------------------------------------------

/// Fallibly heap-allocate `value` and leak it as a raw pointer, returning NULL on
/// allocation failure (the `*_builder_new()` contract). Pairs with [`raw_free`].
///
/// # Safety
/// The returned pointer, if non-null, owns `value` and must be released exactly
/// once via [`raw_free`].
pub unsafe fn raw_new<T>(value: T) -> *mut T {
    let layout = Layout::new::<T>();
    if layout.size() == 0 {
        return core::ptr::NonNull::<T>::dangling().as_ptr();
    }
    let raw = alloc::alloc::alloc(layout) as *mut T;
    if !raw.is_null() {
        raw.write(value);
    }
    raw
}

/// Drop and free a pointer produced by [`raw_new`]. NULL is a no-op (so
/// `*_builder_free(NULL)` is safe, matching `free(3)`).
///
/// # Safety
/// `raw` must be NULL or a pointer previously returned by [`raw_new::<T>`] that
/// has not already been freed.
pub unsafe fn raw_free<T>(raw: *mut T) {
    if raw.is_null() {
        return;
    }
    core::ptr::drop_in_place(raw);
    let layout = Layout::new::<T>();
    if layout.size() != 0 {
        alloc::alloc::dealloc(raw as *mut u8, layout);
    }
}

/// Append `data` to `vec`, reserving fallibly first. `Err(())` signals an
/// out-of-memory condition the caller maps to `ENOMEM` — we never abort on OOM.
pub fn try_extend(vec: &mut Vec<u8>, data: &[u8]) -> Result<(), ()> {
    vec.try_reserve(data.len()).map_err(|_| ())?;
    vec.extend_from_slice(data);
    Ok(())
}

/// Narrow a caller `usize` length to the `u32` the wire formats use, rejecting the
/// (practically impossible, but unchecked otherwise) > 4 GiB case with `EINVAL`
/// rather than a silent truncation. Shared by every entry point that hands a length
/// across the syscall boundary as a `u32`.
pub fn u32_len(n: usize) -> Result<u32, c_int> {
    u32::try_from(n).map_err(|_| libc::EINVAL)
}

/// Fallibly copy `bytes` into a fresh owned `Vec`. `Err(())` on OOM.
pub fn to_vec(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut v = Vec::new();
    try_extend(&mut v, bytes)?;
    Ok(v)
}

/// View a NUL-terminated C string as a byte slice (excluding the NUL), scanning
/// at most `max` bytes. `None` if unterminated within `max`.
///
/// # Safety
/// `ptr` must be non-null and readable up to the NUL (or at least `max` bytes).
pub unsafe fn cstr_bytes<'a>(ptr: *const c_char, max: usize) -> Option<&'a [u8]> {
    let mut len = 0;
    while len < max {
        if *ptr.add(len) == 0 {
            return Some(core::slice::from_raw_parts(ptr as *const u8, len));
        }
        len += 1;
    }
    None
}
