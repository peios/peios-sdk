---
title: Status-only responses
description: The workhorse helper — what it is for, and the cases where it is the whole of a response.
---

```c
int rsi_respond_status(int fd, const struct rsi_request *req, uint32_t status);
```

The workhorse. Use it for:

- **status-only ops on success** — pass `status = RSI_OK`; and
- **any op reporting a non-OK status** — a `LOOKUP` that found nothing, a `SET_VALUE` that failed a compare-and-swap, a permission error: reply with the appropriate `RSI_*` status here, whatever the op.

It fails with `EINVAL` on a bad `req`, an unknown `status`, or `RSI_OK` given for a payload-bearing op (those must use their own helper on success), plus `EIO` / the `write` error.

The rule of thumb: **on failure, always `rsi_respond_status`; on success, `rsi_respond_status` unless the op is one of the five below.**
