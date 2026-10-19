---
title: Conventions
description: The few rules that hold across the whole codec, and what they mean for buffers you pass in.
---

A few rules hold across the codec:

- **Integers are written in their smallest MessagePack form** automatically — you write an `int64`/`uint64` and the encoder picks the compact encoding.
- **`str` values must be valid UTF-8.** Use `bin` for arbitrary bytes. The reader enforces this on `str` reads too.
- **A valid payload is exactly one top-level value**, and an **empty buffer is not valid**. (A map or array at the top counts as that one value.)
- **The writer is sticky-error**, exactly like the [`<peios/security.h>` builders](~peios/sdk-conventions/memory-ownership): the write calls cannot fail individually; the first error latches and surfaces at `peios_mp_writer_bytes` / `peios_mp_writer_error`.
