---
title: The conventions at a glance
description: The whole convention set on one page, as a reference to come back to while reading the module chapters.
---

| Convention | The rule |
|---|---|
| `int` return | fd or `0` on success; `-1` + `errno` on failure. |
| `ssize_t` return | byte length on success; `-1` + `errno` on failure. Strings exclude the `NUL`. |
| Two-call protocol | `cap == 0` / `NULL` probes for the size; too-small non-zero buffer → `ERANGE`, nothing written. |
| errno | standard values only; check the return first, then `errno`. |
| Access denial | `-1` + `EACCES`, with the granted mask still written to the out-param. |
| Builders | heap-backed, sticky-error, `void` adders; check `_error()` at the end; `_free()` every one; `_bytes()` borrows, and the security builders add a `_finish()` that copies. |
| Views | caller-allocated (stack), opaque, borrow the parsed buffer; keep that buffer alive and unmodified. |
| File descriptors | raw `int`, `O_CLOEXEC` by default, closed with `close()`. |
| Constants | use the `<pkm/*.h>` `KACS_*` names directly; only libpeios's own additions are `PEIOS_*`. |

With these in hand, the module documentation reads as just "what does this function do?" — the *how* of memory and errors is answered here, once, for all of them. Next: [your first program](~peios/sdk-basics/your-first-program), which puts the protocol and the error model to work in something you can compile.
