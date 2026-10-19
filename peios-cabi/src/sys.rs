//! Kernel-call mechanism — thin wrappers over libc `syscall()` / `ioctl()`.
//!
//! Every entry into the kernel from a Peios C-ABI library funnels through here. The
//! kernel returns `-errno` directly (raw syscall ABI); libc's `syscall()` and
//! `ioctl()` translate that into `errno` + a `-1` return — exactly the `int`/`-1`+
//! errno contract the libraries expose. So these wrappers stay trivial and the
//! kernel's own errno passes straight through to the caller (no error remapping,
//! unlike the pure-userspace parse/build paths in the consuming crates).

use core::ffi::{c_int, c_long, c_ulong, c_void};

/// Issue an ioctl on a fd. `arg` may be null for argument-less `_IO` codes. Returns
/// 0 on success, `-1` with `errno` set on failure (libc translates the kernel's
/// `-errno`). Any new fd is delivered through a field in `*arg`, not the return.
///
/// # Safety
/// `fd` and `arg` must satisfy the selected request's contract, including every
/// pointed-to buffer's layout, access, lifetime, and synchronization requirements.
#[inline]
pub unsafe fn ioctl(fd: c_int, request: c_ulong, arg: *mut c_void) -> c_int {
    libc::ioctl(fd, request, arg)
}

/// Zero-argument syscall (e.g. `reg_begin_transaction`).
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall0(nr: u32) -> c_long {
    libc::syscall(nr as c_long)
}

/// One-argument syscall.
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall1(nr: u32, a: c_long) -> c_long {
    libc::syscall(nr as c_long, a)
}

/// Two-argument syscall.
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall2(nr: u32, a: c_long, b: c_long) -> c_long {
    libc::syscall(nr as c_long, a, b)
}

/// Three-argument syscall.
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall3(nr: u32, a: c_long, b: c_long, c: c_long) -> c_long {
    libc::syscall(nr as c_long, a, b, c)
}

/// Four-argument syscall (e.g. `kmes_emit`: type, type_len, payload, payload_len).
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall4(nr: u32, a: c_long, b: c_long, c: c_long, d: c_long) -> c_long {
    libc::syscall(nr as c_long, a, b, c, d)
}

/// Five-argument syscall (the native file open: dirfd, path, how, howsize, status).
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall5(nr: u32, a: c_long, b: c_long, c: c_long, d: c_long, e: c_long) -> c_long {
    libc::syscall(nr as c_long, a, b, c, d, e)
}

/// Six-argument syscall (SD get/set: dirfd, path, secinfo, buf, len, flags).
///
/// # Safety
/// The syscall number and arguments must match the target kernel ABI. Any pointer
/// arguments must remain valid for the operation, and the caller must uphold its
/// resource ownership and synchronization requirements.
#[inline]
pub unsafe fn syscall6(
    nr: u32,
    a: c_long,
    b: c_long,
    c: c_long,
    d: c_long,
    e: c_long,
    f: c_long,
) -> c_long {
    libc::syscall(nr as c_long, a, b, c, d, e, f)
}

/// Map a syscall return to the `int` C-ABI result (an fd, 0, or a value; or `-1`).
/// libc has already set `errno` when the raw kernel return was negative.
#[inline]
pub fn ret_int(r: c_long) -> c_int {
    if r < 0 {
        -1
    } else {
        r as c_int
    }
}
