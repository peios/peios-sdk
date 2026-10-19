---
title: rsi/source.h — Becoming a source
description: The one job of this header — declaring which hives your process backs, registering with the kernel, and getting a source fd.
---

`<rsi/source.h>` is where a registry **source** begins. A source is a storage backend for the [LCS registry](~peios/sdk-registry/overview) — the provider counterpart to libpeios's registry *client*. Where a client opens keys and reads values, a source is what actually *holds* those keys and values and answers the kernel's requests for them.

This header has one job: **registration**. You declare which hives your process backs, register with the kernel, and get back a **source fd**. From that point on you serve the RSI (Registry Source Interface) protocol on that fd — [reading requests](~peios/sdk-rsi-request/rsi-request-h-decoding-requests) and [writing responses](~peios/sdk-rsi-response/rsi-response-h-building-responses). Registration requires `SeTcbPrivilege`. The RSI wire constants (`RSI_HIVE_PRIVATE`, `RSI_*`) come from `<pkm/lcs.h>`.

This is part of **librsi**, a separate library from libpeios — link `-lrsi` and include `<rsi.h>` (or the individual `<rsi/*.h>`). It follows the same [library conventions](~peios/sdk-conventions/library-conventions): raw fds, `int` returning `0`/`-1`+errno, and the errno passed straight through from the kernel.

## See also

- **[Registry sources overview](~peios/registry-sources/overview)** — what a source is and how the RSI protocol flows.
- **[The registry](~peios/registry-concepts/overview)** — the operator-side model of hives, layers, and sources.
