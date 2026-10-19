---
title: Validator
description: Checking a buffer in one pass, with acceptance that matches the kernel's own emit-time check.
---

```c
int peios_mp_validate(const void *buf, size_t len, uint32_t max_depth);
```

`peios_mp_validate` confirms `buf`/`len` is **exactly one well-formed MessagePack value**: UTF-8 strings, nesting bounded by `max_depth`, no trailing bytes, non-empty. Returns `0` if valid, `-1` with `errno == EINVAL` otherwise.

Crucially, its acceptance **matches the kernel's emit-time check**, so a `0` return means the [event emit calls](~peios/sdk-events-api/emitting-events) will accept the payload — *at this depth bound*. Pass `KMES_CONFIG_MAX_NESTING_DEPTH_DEFAULT` (32) for the default emit limit; the top-level value is depth 1. Validate before emitting when a payload comes from an untrusted or dynamic source, so you turn a would-be `EINVAL` from the kernel into a check you control.
