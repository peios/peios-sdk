---
title: What comes next
description: Registration is the whole of this header — where the serve loop and the response helpers live.
---

Registration is the whole of this header. Once you hold the source fd, the serve loop lives in the other two:

- **[`<rsi/request.h>`](~peios/sdk-rsi-request/rsi-request-h-decoding-requests)** — read and decode the requests the kernel sends.
- **[`<rsi/response.h>`](~peios/sdk-rsi-response/rsi-response-h-building-responses)** — build and send the replies.

The [serving requests](~peios/registry-sources/serving-requests) guide ties them together into a working serve loop.
