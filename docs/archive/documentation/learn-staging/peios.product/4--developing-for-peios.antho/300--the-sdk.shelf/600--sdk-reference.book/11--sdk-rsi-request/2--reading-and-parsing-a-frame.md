---
title: Reading and parsing a frame
description: The rsi_request struct, what each field means, and how to read one frame off the source fd and parse its header.
---

```c
struct rsi_request {
    uint64_t    request_id;   /* echo this in the response */
    uint64_t    txn_id;       /* transaction id (0 outside a transaction) */
    const void *payload;      /* borrowed; valid until the frame is reused */
    uint32_t    payload_len;
    uint16_t    op_code;      /* RSI_LOOKUP, RSI_SET_VALUE, … — dispatch on this */
};

ssize_t rsi_read_request(int fd, void *buf, size_t cap);
int     rsi_parse_request(const void *frame, size_t len, struct rsi_request *out);
```

- **`rsi_read_request`** reads one framed request from the source fd into `buf` — a thin `read(2)` wrapper that **blocks** until a request is queued, then returns the frame length (pass it to `rsi_parse_request`). It returns **`0` at EOF** (the source is closing — leave the loop) or `-1` with `errno`, notably **`EMSGSIZE`** if `cap` is smaller than the pending frame (size `buf` generously, or grow and retry).
- **`rsi_parse_request`** splits a frame into its header and payload view, filling `out` with the `request_id` (which you must echo in the response), the `txn_id` (`0` when the request is not inside a transaction), the `op_code` to dispatch on, and a **borrowed** `payload` pointer. Returns `0`, or `-1` with `errno` (`EINVAL` on NULL args, `EBADMSG` on a malformed frame).
