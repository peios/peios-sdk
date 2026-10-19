---
title: rsi/request.h — Decoding requests
description: The shape of every serve loop — read a frame, parse the header, dispatch on the op-code, decode with the matching parser.
---

`<rsi/request.h>` is the receiving half of a registry source's serve loop. The kernel sends your source [RSI](~peios/registry-sources/overview) requests — "look up this child", "store this value", "begin this transaction" — as framed messages on the source fd. This header reads one frame, splits its header from its payload, and decodes the payload into a flat, typed struct you can act on.

The shape of the loop is always: **read a frame → parse the header → dispatch on the op-code → decode the payload with the matching parser**. The decoders are thin wrappers over the kernel's own RSI parsers, so your wire handling is guaranteed compatible with what the kernel sent.

> **Borrowing:** every decoded name/data field is a `(ptr, len)` pair that **borrows into your frame buffer**. The pointers are valid only until you reuse that buffer for the next `rsi_read_request`. Copy out anything you need to keep across iterations. This is the same [borrow discipline](~peios/sdk-conventions/memory-ownership) as libpeios's views.

Op-code and field constants (`RSI_LOOKUP`, `RSI_WRITE_KEY_FIELD_*`, `RSI_TXN_*`) come from `<pkm/lcs.h>`.

## See also

- **[`<rsi/response.h>`](~peios/sdk-rsi-response/rsi-response-h-building-responses)** — building the reply each op expects.
- **[Serving requests](~peios/registry-sources/serving-requests)** — the read/parse/dispatch/respond loop in full.
