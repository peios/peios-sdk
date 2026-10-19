---
title: msgpack.h — MessagePack codec
description: A general MessagePack codec whose reason for being is event payloads — its writer, reader and validator.
---

`<peios/msgpack.h>` is a small, self-contained [MessagePack](https://msgpack.org) codec. It exists because KMES event payloads *are* MessagePack: the kernel only *structurally validates* a payload on emit — it does not build or interpret it — so userspace owns the encode and decode. This codec is that path, and its validator's acceptance is deliberately **matched to the kernel's emit-time check**, so a payload this codec produces and validates is guaranteed to be accepted by [`peios_event_emit`](~peios/sdk-events-api/emitting-events).

You can use it as a general MessagePack codec, but its reason for being is [events](~peios/sdk-events-api/event-h-events-kmes).

It has three parts: a heap-backed **writer**, a stack-allocatable **reader**, and a **validator**.

## See also

- **[`<peios/event.h>`](~peios/sdk-events-api/event-h-events-kmes)** — the KMES events these payloads travel in.
- **[Library conventions](~peios/sdk-conventions/library-conventions)** — the sticky-error builder model the writer follows.
