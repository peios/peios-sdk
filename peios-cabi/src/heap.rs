//! Global allocator implementation: routes Rust's `alloc` through the C library heap.
//!
//! A Peios C-ABI cdylib is `no_std`, but its dependencies (`kacs-core`, `lcs-core`)
//! and its own builders allocate via the `alloc` crate, so the shipped artifact must
//! supply a `#[global_allocator]`. [`LibcAllocator`] backs it with libc
//! `malloc`/`free`/`realloc` so that whether the library is loaded as a `.so` or
//! linked statically into a C host, every allocation shares that process's C heap —
//! there is no second allocator and no surprise about which `free` owns a pointer.
//!
//! The implementation mirrors the standard library's Unix `System` allocator:
//! the common case (alignment within libc's `max_align_t`) goes straight to
//! `malloc`/`realloc`, and the over-aligned case falls back to `posix_memalign`.
//! Byte buffers and ordinary builder structs use the fast path. The aligned
//! path preserves the allocator contract for any more strictly aligned type.
//!
//! This crate provides the allocator *type* only. The `#[global_allocator]` static
//! that installs it lives in each consuming cdylib, gated out of `cfg(test)` builds
//! (where std supplies its own global allocator).

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use libc::c_void;

/// Alignment guaranteed by libc `malloc` on the targets we ship (x86_64 LP64):
/// allocations are suitably aligned for any `max_align_t`, i.e. 16 bytes.
const MIN_ALIGN: usize = 16;

/// A `GlobalAlloc` backed by libc `malloc`/`free`/`realloc`. Install it from a
/// consuming cdylib with `#[global_allocator] static G: LibcAllocator = LibcAllocator;`.
pub struct LibcAllocator;

// SAFETY: every successful allocation satisfies its requested layout and is
// owned by libc. Over-aligned reallocations copy only the live overlap and leave
// the original allocation untouched when allocating its replacement fails.
unsafe impl GlobalAlloc for LibcAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc requires a nonzero layout; libc provides MIN_ALIGN
        // or aligned_alloc raises the requested alignment to its accepted floor.
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            libc::malloc(layout.size()) as *mut u8
        } else {
            aligned_alloc(&layout)
        }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: calloc initializes its allocation; the fallback writes exactly
        // layout.size() bytes only after allocation succeeds.
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            libc::calloc(layout.size(), 1) as *mut u8
        } else {
            let ptr = self.alloc(layout);
            if !ptr.is_null() {
                ptr::write_bytes(ptr, 0, layout.size());
            }
            ptr
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // SAFETY: GlobalAlloc callers return a live allocation owned by this allocator.
        libc::free(ptr as *mut c_void);
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller provides the original allocation and layout, and a
        // nonzero valid replacement size. Both branches preserve ownership on failure.
        if layout.align() <= MIN_ALIGN && layout.align() <= new_size {
            libc::realloc(ptr as *mut c_void, new_size) as *mut u8
        } else {
            realloc_aligned(self, ptr, layout, new_size)
        }
    }
}

/// `posix_memalign` wrapper for the over-aligned case.
#[inline]
unsafe fn aligned_alloc(layout: &Layout) -> *mut u8 {
    let mut out: *mut c_void = ptr::null_mut();
    // `posix_memalign` requires the alignment to be a power-of-two multiple of
    // `sizeof(void*)`; `Layout` already guarantees power-of-two, so we only need
    // to raise the floor to the pointer size.
    let align = layout.align().max(core::mem::size_of::<usize>());
    if libc::posix_memalign(&mut out, align, layout.size()) != 0 {
        ptr::null_mut()
    } else {
        out as *mut u8
    }
}

/// Over-aligned `realloc`: allocate fresh, copy the overlap, free the old block.
#[inline]
unsafe fn realloc_aligned(
    alloc: &LibcAllocator,
    ptr: *mut u8,
    old_layout: Layout,
    new_size: usize,
) -> *mut u8 {
    let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };
    let new_ptr = alloc.alloc(new_layout);
    if !new_ptr.is_null() {
        let copied = core::cmp::min(old_layout.size(), new_size);
        // SAFETY: the fresh allocation cannot overlap the old one; copied is
        // bounded by both live allocations. Free only after the copy succeeds.
        ptr::copy_nonoverlapping(ptr, new_ptr, copied);
        alloc.dealloc(ptr, old_layout);
    }
    new_ptr
}
