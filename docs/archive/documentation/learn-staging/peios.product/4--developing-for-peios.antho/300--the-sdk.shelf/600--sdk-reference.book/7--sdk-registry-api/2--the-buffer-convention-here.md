---
title: The buffer convention here
description: Where the registry API departs from the library-wide two-call protocol, and what to do instead.
---

Most of libpeios returns variable-length data with an `ssize_t` and the [two-call protocol](~peios/sdk-conventions/the-two-call-buffer-protocol). The registry's *reads* use the same idea but express it through **descriptor structs** rather than a return value, because a single read often fills more than one buffer (a value's data *and* its layer name, say). The pattern:

- Each read takes a descriptor struct with `*_cap` fields (in) and `*_len` fields (out), plus buffer pointers.
- On success it returns `0` and writes the actual length into each `*_len`.
- If a buffer is too small it returns `-1` with `errno == ERANGE` and writes the **required** length into the matching `*_len` — so a **zero-capacity buffer probes the size**.
- A `NULL` buffer is valid only with zero capacity; `NULL` with a nonzero capacity is `EINVAL`.
- For a read with two buffers, `ERANGE` is returned if *either* is too small, and *both* required lengths are reported, so one probe sizes everything.

Everything else follows the usual Linux convention: `0` / `-1` + `errno`.
