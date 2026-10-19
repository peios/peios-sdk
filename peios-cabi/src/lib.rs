//! peios-cabi — shared C-ABI substrate for the Peios userspace libraries.
//!
//! The plumbing every Peios `extern "C"` cdylib (libpeios and librsi) needs to
//! behave as a sound C library:
//!
//! - [`LibcAllocator`] — a malloc-backed `GlobalAlloc` (see [`heap`]);
//! - [`error`] — the POSIX errno slot;
//! - [`sys`] — the raw `syscall()` / `ioctl()` wrappers and return reshaping;
//! - [`abi`] — the getxattr-style copy-out contract, opaque-handle `new`/`free`, and
//!   the heap-builder helpers.
//!
//! It is `no_std` + `alloc` and depends only on `libc`. The two attributes that may
//! exist only once per final artifact — `#[global_allocator]` and `#[panic_handler]`
//! — are deliberately **not** set here: each consuming cdylib declares them (gated
//! out of `cfg(test)` builds), wiring the allocator to [`LibcAllocator`]. Domain
//! error→errno translation also stays in the consumers; this crate carries only the
//! errno slot itself.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod abi;
pub mod error;
pub mod heap;
pub mod sys;

pub use heap::LibcAllocator;
