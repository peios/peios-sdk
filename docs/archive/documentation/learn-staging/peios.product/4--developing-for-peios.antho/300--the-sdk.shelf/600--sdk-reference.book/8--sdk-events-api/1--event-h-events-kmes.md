---
title: event.h — Events (KMES)
description: Emitting events into KMES and consuming them from the ring buffers, with MessagePack payloads built by the codec module.
---

`<peios/event.h>` is the client surface of **KMES** — Peios's sole event path. The kernel stamps every event with **trusted metadata** (timestamp, per-CPU sequence, CPU id, identity GUIDs) and writes it into a **per-CPU lock-free ring buffer**. There is no other way to emit or observe events: audit records, subsystem events, and your own application events all flow through the same rings. Producers emit; consumers attach to the rings and drain them.

Each event payload is **a single MessagePack value** — build and parse it with [`<peios/msgpack.h>`](~peios/sdk-msgpack/msgpack-h-messagepack-codec).

Two privileges gate the module: **emitting requires `SeAuditPrivilege`**, and **consuming (attaching to a ring) requires `SeSecurityPrivilege`**.

## See also

- **[`<peios/msgpack.h>`](~peios/sdk-msgpack/msgpack-h-messagepack-codec)** — building and parsing the payloads events carry.
- **[Auditing](~peios/auditing/overview)** — the operator-side view of the event and audit stream.
