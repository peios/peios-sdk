---
title: Building responses
type: how-to
description: Reply to RSI requests the right way — status-only for most ops and failures, payload-bearing helpers for the five ops that return data, and the validation contract to respect.
related:
  - peios/sdk-rsi-response/rsi-response-h-building-responses
  - peios/registry-sources/serving-requests
---

Every request gets exactly one response. Choosing the right one is simple once you know the rule; filling it in is a matter of handing librsi flat arrays and letting it encode the frame. The full contract is in [`rsi/response.h`](~peios/sdk-rsi-response/rsi-response-h-building-responses); this guide is the working version.

## The rule

> **On failure, always [`rsi_respond_status`](~peios/sdk-rsi-response/status-only-responses). On success, `rsi_respond_status` too — unless the op is one of the five that carry a payload.**

Most operations (`SET_VALUE`, `CREATE_KEY`, the transaction ops, `FLUSH`, …) are status-only in both cases. And *any* op reports a non-OK outcome with `rsi_respond_status`, whatever it is:

```c
/* Success for a status-only op: */
rsi_respond_status(fd, req, RSI_OK);

/* Any op reporting a problem: */
rsi_respond_status(fd, req, RSI_NOT_FOUND);   /* e.g. a missing key */
rsi_respond_status(fd, req, RSI_CAS_FAILED);  /* a failed CAS */
```

You never build the frame or echo the request id yourself — the helper reads what it needs from `req` and writes the framed reply to `fd`.

## The five payload-bearing ops

Exactly five ops return data on success, each with its own helper:

| Op | Helper | You supply |
|---|---|---|
| `LOOKUP` | `rsi_respond_lookup` | path entries + the referenced keys' metadata |
| `ENUM_CHILDREN` | `rsi_respond_enum_children` | children (name + path entries) + metadata |
| `READ_KEY` | `rsi_respond_read_key` | one key's non-layered metadata |
| `QUERY_VALUES` | `rsi_respond_query_values` | value entries + blanket tombstones |
| `DELETE_LAYER` | `rsi_respond_delete_layer` | the orphaned keys' GUIDs |

You pass the result as flat arrays; librsi validates and heap-encodes the wire frame. A `QUERY_VALUES` reply, for example:

```c
struct rsi_value_entry values[] = {
    { .value_name = "Timeout", .value_name_len = 7,
      .layer_name = "base",    .layer_name_len = 4,
      .value_type = REG_DWORD, .data = &timeout, .data_len = 4,
      .sequence = 42 },
    /* … one per effective value you hold … */
};

rsi_respond_query_values(fd, req, values, 1, /*blankets=*/NULL, 0);
```

You report *what you store*, per layer; the kernel resolves precedence across the layers you return. (Note the `(NULL, 0)` for the empty blanket array — a NULL pointer is allowed only when its count is zero.)

## The validation contract

The helpers check their inputs before encoding and return `-1`/`EINVAL` if you break the contract — better a caught programming error than a malformed frame on the wire. The rules that apply across all of them:

- **`(ptr, len)` pairs:** a pointer may be `NULL` only when its length/count is zero.
- **Booleans** (`volatile_key`, `symlink`, target types) are strictly `0` or `1`.
- **Hidden path targets** (`RSI_PATH_TARGET_HIDDEN`) carry an **all-zero** `target_guid`.
- **`LOOKUP` / `ENUM_CHILDREN` metadata** must exactly cover the GUID targets in your path entries — every referenced key present, none missing, no duplicates, nothing unreferenced.
- **`DELETE_LAYER` orphan GUIDs** are nonzero and unique.

Beyond `EINVAL`, a helper can also fail with `ENOMEM` or `EOVERFLOW` (building the frame), `EIO` (a short write), or the raw `write(2)` errno — treat those as you would any I/O failure on the source fd.

## Putting it together

A source's handler for a payload-bearing op is: decode the request, gather the result from storage into the flat arrays the helper wants, call the helper. For a status-only op: decode, do the work, `rsi_respond_status`. On any error along the way — a decode failure, a missing key, an I/O problem — `rsi_respond_status` with the appropriate non-OK `RSI_*` code. That uniformity is what keeps a source's dispatch table readable no matter how many ops it supports.

## Next

- **[`rsi/response.h` reference](~peios/sdk-rsi-response/rsi-response-h-building-responses)** — every helper, struct, and error in full.
- **[Serving requests](~peios/registry-sources/serving-requests)** — the loop these replies live in.
