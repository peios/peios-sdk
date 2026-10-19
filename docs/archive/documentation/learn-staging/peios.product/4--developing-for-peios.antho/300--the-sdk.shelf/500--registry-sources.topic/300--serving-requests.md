---
title: Serving requests
type: how-to
description: The source serve loop end to end — read a framed request, parse its header, dispatch on the op-code, and decode the payload with its typed parser.
related:
  - peios/sdk-rsi-request/rsi-request-h-decoding-requests
  - peios/registry-sources/building-responses
---

With a [source fd in hand](~peios/registry-sources/registering-a-source), a source spends its life in one loop: read a request, decode it, do the work, reply. This guide builds that loop. The exhaustive per-op detail is in [`rsi/request.h`](~peios/sdk-rsi-request/rsi-request-h-decoding-requests) and [`rsi/response.h`](~peios/sdk-rsi-response/rsi-response-h-building-responses).

## The loop skeleton

```c
unsigned char buf[64 * 1024];   /* size generously — a short buffer is EMSGSIZE */

for (;;) {
    ssize_t n = rsi_read_request(src_fd, buf, sizeof buf);
    if (n == 0) break;                       /* EOF — the source is closing */
    if (n < 0) { perror("read_request"); break; }

    struct rsi_request req;
    if (rsi_parse_request(buf, n, &req) != 0) continue;   /* EBADMSG — skip */

    switch (req.op_code) {
        case RSI_LOOKUP:    handle_lookup(src_fd, &req);    break;
        case RSI_SET_VALUE: handle_set_value(src_fd, &req); break;
        /* … a case per op you support … */
        default:
            /* Unknown or unsupported op: reply with a non-OK status
             * (RSI_TXN_NOT_SUPPORTED for transaction ops you don't implement). */
            rsi_respond_status(src_fd, &req, RSI_INVALID);
            break;
    }
}
```

Three details in that skeleton matter:

- **`rsi_read_request` blocks** until a request is queued, and returns **`0` at EOF** — that's your exit. Size `buf` generously; a frame larger than `buf` returns `-1`/`EMSGSIZE`.
- **`rsi_parse_request` gives you `req.op_code`** to dispatch on, plus `req.request_id` (which every reply must echo — the helpers do this for you) and `req.txn_id` (nonzero when the request is inside a transaction).
- **Always reply.** Every request expects exactly one response. If you can't handle an op, reply with a non-OK status rather than dropping it.

## Decoding a request

Inside a handler, decode the payload with the matching `rsi_request_*` parser. For a `LOOKUP`:

```c
void handle_lookup(int fd, const struct rsi_request *req)
{
    struct rsi_lookup q;
    if (rsi_request_lookup(req, &q) != 0) {          /* EBADMSG */
        rsi_respond_status(fd, req, RSI_INVALID);
        return;
    }

    /* q.parent_guid (by value), q.child_name / q.child_name_len (borrowed).
       Resolve the child in your storage across its layers … */

    /* … then reply with the resolved path entries + metadata: */
    rsi_respond_lookup(fd, req, entries, entry_count, metadata, metadata_count);
}
```

And for a `SET_VALUE`, a status-only op:

```c
void handle_set_value(int fd, const struct rsi_request *req)
{
    struct rsi_set_value v;
    if (rsi_request_set_value(req, &v) != 0) {
        rsi_respond_status(fd, req, RSI_INVALID);
        return;
    }

    /* Honour the CAS guard before storing. */
    if (v.expected_sequence != 0 && current_seq(&v) != v.expected_sequence) {
        rsi_respond_status(fd, req, RSI_CAS_FAILED);
        return;
    }
    store_value(&v);                                  /* your storage */
    rsi_respond_status(fd, req, RSI_OK);
}
```

## The borrow rule

Every decoded name and data field — `q.child_name`, `v.value_name`, `v.data`, and the rest — is a `(ptr, len)` pair that **points into `buf`**. Those pointers are valid only until the next `rsi_read_request` reuses the buffer. So:

- **Consume them within the handler** (store the bytes, resolve the name) before the loop comes around again, or
- **Copy out** anything you need to keep. Never stash a raw borrowed pointer across iterations.

GUIDs and scalars in the decoded struct are copied by value, so those are always safe to keep — it's only the borrowed `(ptr, len)` fields that have the lifetime.

## Transactions

When the kernel sends `BEGIN_TRANSACTION`, buffer subsequent writes tagged with that `transaction_id` instead of applying them; `req.txn_id` on each later request tells you which transaction it belongs to (`0` = none). On `COMMIT_TRANSACTION` apply the buffered set atomically; on `ABORT_TRANSACTION` discard it. All three are status-only replies. The kernel coordinates transaction boundaries — your job is to buffer, then commit or discard on command.

## Next

- **[Building responses](~peios/registry-sources/building-responses)** — choosing and filling the right reply.
- **[`rsi/request.h` reference](~peios/sdk-rsi-request/rsi-request-h-decoding-requests)** — every op's decoder and struct.
