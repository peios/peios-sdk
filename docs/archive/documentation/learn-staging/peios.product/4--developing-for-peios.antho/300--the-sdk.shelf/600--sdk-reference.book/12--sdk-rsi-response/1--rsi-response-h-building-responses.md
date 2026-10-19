---
title: rsi/response.h — Building responses
description: Echoing the request id and op-code with an RSI status — the response helpers, and why most operations are status-only.
---

`<rsi/response.h>` is the sending half of a source's serve loop. After you handle a request, you reply on the source fd with a framed response. This header builds those frames for you: you pass the result as flat arrays, and librsi validates and heap-encodes the wire frame — you never hand-pack a byte.

Every response echoes the request's id and its op-code (OR'd with the response bit) and carries an `RSI_*` status. **Most operations are status-only**; five carry a payload on success. Any operation can report a *non-OK* status with the status-only helper.

For the wire, a response is a 14-byte header (echoed request id, op-code | `RSI_RESPONSE_BIT`) plus a 4-byte `RSI_*` status, followed by an op-specific payload for payload-bearing successes; multi-byte integers are little-endian and names/data are length-prefixed. You don't assemble any of that — the helpers do. Status and target-type constants (`RSI_OK`, `RSI_PATH_TARGET_GUID`, …) come from `<pkm/lcs.h>`.

## See also

- **[`<rsi/request.h>`](~peios/sdk-rsi-request/rsi-request-h-decoding-requests)** — decoding the request each of these replies to.
- **[Building responses](~peios/registry-sources/building-responses)** — choosing and filling the right responder.
