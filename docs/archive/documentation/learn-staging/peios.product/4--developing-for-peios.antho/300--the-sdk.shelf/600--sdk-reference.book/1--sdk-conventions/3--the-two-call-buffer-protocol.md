---
title: The two-call buffer protocol
description: The getxattr-style protocol every variable-length function follows — measure with a null buffer, then call again with one big enough.
---

Every function that returns variable-length bytes — anything with an `ssize_t` return and an `(out, cap)` pair — follows the same **getxattr-style** protocol. It is the single most important convention in the library, so it is worth internalising.

The rule:

- Call with **`cap == 0`** (or a **`NULL` buffer**) to **probe**: the function writes nothing and returns the number of bytes the result needs.
- Call with a buffer of **at least that size** to **retrieve**: the function fills the buffer and returns the number of bytes it wrote.
- Call with a **non-zero but too-small** buffer and it **fails with `ERANGE` and writes nothing** — never a truncated or partial result.

That last point is the safety property that makes the protocol trustworthy: a too-small buffer is a clean, detectable error, not a silent truncation. You never have to wonder whether you got the whole thing.

The canonical two-call sequence:

```c
/* 1. Probe for the size. */
ssize_t need = peios_sid_format(sid, sid_len, NULL, 0);
if (need < 0) { /* errno set */ }

/* 2. Allocate. For a string, add 1 for the NUL. */
char *buf = malloc(need + 1);

/* 3. Retrieve. */
ssize_t n = peios_sid_format(sid, sid_len, buf, need + 1);
if (n < 0) { /* errno set */ }
/* buf now holds the formatted SID; n is its length (excluding the NUL). */
```

When you already know a comfortable upper bound, you can skip the probe and call once with a big-enough buffer. Some results have a fixed maximum the library gives you a constant for — for example a SID is never larger than `PEIOS_SID_MAX_BYTES`, so a stack buffer of that size always fits and never needs a probe. Those shortcuts are called out where they apply; the two-call protocol is always available as the general fallback.
