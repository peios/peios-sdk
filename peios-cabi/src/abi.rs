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
/// `out` must be NULL or valid for `cap` writable bytes when `cap != 0`.
/// A non-null destination must not overlap the source, and must be exclusively
/// writable for the duration of the call.
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
/// `out` must be NULL or valid for `cap` writable bytes when `cap != 0`.
/// A non-null destination must not overlap the source, and must be exclusively
/// writable for the duration of the call.
pub unsafe fn emit_str(s: &[u8], out: *mut c_char, cap: usize) -> isize {
    let need = s.len(); // excluding the NUL
    if cap == 0 {
        return need as isize; // probe
    }
    if out.is_null() {
        set_errno(libc::EINVAL);
        return -1;
    }
    // A real write needs room for the NUL too. Guard the `need + 1` against the
    // (degenerate, unreachable for a real slice) overflow so it can never wrap
    // the comparison into a spurious success.
    let Some(needed) = need.checked_add(1) else {
        set_errno(libc::ERANGE);
        return -1;
    };
    if cap < needed {
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
/// an overrun returns `core::fmt::Error` without appending that fragment.
pub struct StackWriter {
    buf: [u8; Self::CAP],
    len: usize,
}

impl Default for StackWriter {
    fn default() -> Self {
        Self::new()
    }
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
// Allocation failure is the only error; C ABI consumers translate it to ENOMEM.
#[allow(clippy::result_unit_err)]
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
// Preserve the shared allocation-only error contract used by the ABI wrappers.
#[allow(clippy::result_unit_err)]
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
/// That memory must remain live and unmodified for the returned slice's lifetime.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{get_errno, set_errno};

    // ---- emit_bytes: the getxattr probe / EINVAL / ERANGE / copy contract ----

    #[test]
    fn emit_bytes_probe_reports_len_and_writes_nothing() {
        // cap == 0 is a probe: returns src.len(), tolerates a NULL out.
        let src = [1u8, 2, 3, 4];
        assert_eq!(unsafe { emit_bytes(&src, core::ptr::null_mut(), 0) }, 4);
    }

    #[test]
    fn emit_bytes_null_out_with_capacity_is_einval() {
        set_errno(0);
        let src = [1u8, 2, 3];
        assert_eq!(unsafe { emit_bytes(&src, core::ptr::null_mut(), 8) }, -1);
        assert_eq!(get_errno(), libc::EINVAL);
    }

    #[test]
    fn emit_bytes_too_small_is_erange_and_never_truncates() {
        set_errno(0);
        let src = [9u8, 9, 9, 9];
        let mut buf = [0u8; 3];
        assert_eq!(unsafe { emit_bytes(&src, buf.as_mut_ptr(), buf.len()) }, -1);
        assert_eq!(get_errno(), libc::ERANGE);
        assert_eq!(buf, [0, 0, 0]); // nothing written
    }

    #[test]
    fn emit_bytes_exact_and_oversized_copy_all() {
        let src = [1u8, 2, 3, 4];
        let mut exact = [0u8; 4];
        assert_eq!(unsafe { emit_bytes(&src, exact.as_mut_ptr(), 4) }, 4);
        assert_eq!(exact, src);
        let mut over = [0u8; 8];
        assert_eq!(unsafe { emit_bytes(&src, over.as_mut_ptr(), 8) }, 4);
        assert_eq!(&over[..4], &src);
    }

    #[test]
    fn emit_bytes_empty_source() {
        let src: [u8; 0] = [];
        assert_eq!(unsafe { emit_bytes(&src, core::ptr::null_mut(), 0) }, 0);
        let mut buf = [7u8; 2];
        assert_eq!(unsafe { emit_bytes(&src, buf.as_mut_ptr(), 2) }, 0);
        assert_eq!(buf, [7, 7]);
    }

    // ---- emit_str: same contract, plus the NUL terminator accounting ----

    #[test]
    fn emit_str_probe_excludes_nul() {
        assert_eq!(unsafe { emit_str(b"hello", core::ptr::null_mut(), 0) }, 5);
    }

    #[test]
    fn emit_str_needs_room_for_the_terminator() {
        set_errno(0);
        let mut buf = [0u8; 3]; // exactly len, no room for the NUL
        assert_eq!(
            unsafe { emit_str(b"abc", buf.as_mut_ptr() as *mut c_char, buf.len()) },
            -1
        );
        assert_eq!(get_errno(), libc::ERANGE);
    }

    #[test]
    fn emit_str_writes_and_nul_terminates_without_overrun() {
        let mut buf = [0xAAu8; 5];
        assert_eq!(
            unsafe { emit_str(b"abc", buf.as_mut_ptr() as *mut c_char, buf.len()) },
            3
        );
        assert_eq!(&buf[..3], b"abc");
        assert_eq!(buf[3], 0); // NUL at `need`
        assert_eq!(buf[4], 0xAA); // byte past the NUL is untouched
    }

    #[test]
    fn emit_str_null_out_with_capacity_is_einval() {
        set_errno(0);
        assert_eq!(unsafe { emit_str(b"x", core::ptr::null_mut(), 4) }, -1);
        assert_eq!(get_errno(), libc::EINVAL);
    }

    #[test]
    fn emit_str_empty_needs_one_byte_for_the_nul() {
        assert_eq!(unsafe { emit_str(b"", core::ptr::null_mut(), 0) }, 0);
        let mut buf = [0xAAu8; 1];
        assert_eq!(
            unsafe { emit_str(b"", buf.as_mut_ptr() as *mut c_char, 1) },
            0
        );
        assert_eq!(buf[0], 0);
    }

    // ---- u32_len: the boundary narrowing ----

    #[test]
    fn u32_len_passes_in_range_and_rejects_over() {
        assert_eq!(u32_len(0), Ok(0));
        assert_eq!(u32_len(u32::MAX as usize), Ok(u32::MAX));
        #[cfg(target_pointer_width = "64")]
        assert_eq!(u32_len((u32::MAX as usize) + 1), Err(libc::EINVAL));
    }

    // ---- cstr_bytes: bounded scan ----

    #[test]
    fn cstr_bytes_stops_at_nul() {
        let buf = b"hi\0rest";
        let got = unsafe { cstr_bytes(buf.as_ptr() as *const c_char, buf.len()) };
        assert_eq!(got, Some(&b"hi"[..]));
    }

    #[test]
    fn cstr_bytes_empty_string() {
        let buf = b"\0";
        assert_eq!(
            unsafe { cstr_bytes(buf.as_ptr() as *const c_char, 1) },
            Some(&b""[..])
        );
    }

    #[test]
    fn cstr_bytes_unterminated_within_max_is_none() {
        let buf = b"abcd"; // no NUL in the first 4 bytes
        assert_eq!(
            unsafe { cstr_bytes(buf.as_ptr() as *const c_char, 4) },
            None
        );
    }

    #[test]
    fn cstr_bytes_finds_nul_at_last_scanned_index() {
        // Indices 0..max are scanned; a NUL at index max-1 is found.
        let buf = b"abc\0";
        assert_eq!(
            unsafe { cstr_bytes(buf.as_ptr() as *const c_char, 4) },
            Some(&b"abc"[..])
        );
    }
}
