---
title: Memory ownership
description: libpeios never hands you an allocation to free — the builder pattern for constructing buffers, and the view pattern for reading them.
---

libpeios never hands you an allocation to `free()`. Instead it uses two ownership patterns — **builders** for constructing byte buffers and **views** for reading them — and both keep the memory question simple: you own your buffers, the library borrows or copies, and the two never get confused.

### Builders — constructing buffers

Anything you *assemble* (an ACL, a security descriptor, a token specification) is built with a **builder**: an opaque, heap-backed object you create, feed, take the bytes from, and free.

Builders have three properties worth knowing up front:

1. **They are sticky-error.** The incremental `add`/`set` calls return `void` — they never fail inline. If one hits a problem (a bad input, an allocation failure), the builder **latches** the error and every later call is a no-op. You do not have to check each step. Instead you check *once*, at the end: either call the builder's `_error()` accessor (it returns the latched errno, or `0` if all is well), or notice that taking the bytes fails. This lets you write a long, clean sequence of `add` calls without a conditional after every line.

2. **You free every builder you create.** Each `_new()` is paired with a `_free()`. Builders also have a `_reset()` that drops the accumulated content *and* clears the sticky error, so you can reuse one builder across several objects instead of churning allocations.

3. **Taking the bytes: borrow (and sometimes copy).** Every builder has a **`_bytes()`** that hands back a pointer *into the builder* — zero-copy, no allocation. That pointer is valid only until the next mutating call, `_reset()`, or `_free()` on that builder. Use it when you are going to consume the bytes immediately (for instance, pass them straight into a kernel call). The call comes in two shapes, and not every builder offers a copying counterpart:
   - The **security builders** (`peios_acl_builder_bytes`, `peios_sd_builder_bytes`) *return the pointer* — `NULL` if the sticky error is set — and write the length through an optional `len_out` pointer. Each is paired with a **`_finish()`** that copies the buffer into a caller-supplied buffer using the [two-call protocol](~peios/sdk-conventions/the-two-call-buffer-protocol) above, for when the bytes must outlive the builder.
   - **`peios_token_builder_bytes` and `peios_mp_writer_bytes`** are shaped the other way round: they *return the length* as an `ssize_t` (`-1` with `errno` on a latched error) and write the borrowed pointer through an out-parameter (which may be `NULL` to get just the length). Neither has a `_finish()` — copy the borrowed bytes yourself if they need to outlive the builder.

A typical builder lifecycle:

```c
peios_acl_builder *b = peios_acl_builder_new();   /* NULL on OOM */
peios_acl_builder_allow(b, sid, sid_len, mask, 0); /* void — no check */
peios_acl_builder_deny(b, other, other_len, mask, 0);

size_t len;
const void *acl = peios_acl_builder_bytes(b, &len); /* NULL if errored */
if (!acl) { int err = peios_acl_builder_error(b); /* handle */ }
/* … use `acl` before the next mutation … */

peios_acl_builder_free(b);
```

### Views — reading buffers

Anything you *parse* (a security descriptor, an ACL, a SID array from a token) is read through a **view**: a small, caller-allocated struct that you point at a buffer you already hold.

Views have their own two rules:

1. **You allocate the view; it is stack-friendly.** A view type such as `peios_sd_view` is an opaque fixed-size struct — you declare one as a local variable and pass its address to the parse call. No heap, no free. The struct's fields are opaque: never read them directly; use the accessor functions.

2. **A view borrows the buffer it parses — zero-copy.** The parse call does not copy the data; the view points *into* your buffer, and every accessor that yields a SID, a nested ACL, or a blob hands back a pointer into that same buffer. So the buffer must **stay alive and unmodified** for as long as the view — and anything you derived from it — is in use. Free or mutate the underlying buffer and every pointer the view gave you dangles.

```c
peios_sd_view sd;                        /* on the stack */
if (peios_sd_parse(buf, buf_len, &sd) != 0) { /* EINVAL */ }

const void *owner; size_t owner_len;
if (peios_sd_view_owner(&sd, &owner, &owner_len) == 0) {
    /* `owner` points INTO `buf` — valid only while `buf` lives. */
}
```

Views compose: parsing a security descriptor gives you a `peios_sd_view`, from which you obtain a `peios_acl_view` for its DACL, from which you obtain each `peios_ace_view`. Every one of them borrows the *same* original buffer, so keeping that one buffer alive keeps the whole tree valid.

The symmetry is the thing to remember: **builders own heap and must be freed; views own nothing and borrow your buffer.** Constructing is builders, reading is views, and neither ever asks you to free something the library allocated.
