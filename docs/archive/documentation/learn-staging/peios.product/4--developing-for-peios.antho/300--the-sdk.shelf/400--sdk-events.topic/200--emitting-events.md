---
title: Emitting events
type: how-to
description: Build a MessagePack payload and emit an event — singly, and in batches for high-rate producers.
related:
  - peios/sdk-events-api/event-h-events-kmes
  - peios/sdk-msgpack/msgpack-h-messagepack-codec
---

Emitting an event is two steps: build a MessagePack payload, then hand it and an event type to the kernel. This guide shows both; [`event.h`](~peios/sdk-events-api/event-h-events-kmes) and [`msgpack.h`](~peios/sdk-msgpack/msgpack-h-messagepack-codec) are the full references. Emitting requires `SeAuditPrivilege`.

Choose the event type and fields first: [Naming and shaping your events](~peios/sdk-events/naming-and-shaping-events) covers that, and the examples below use its backup-tool event.

## Ask whether the type is on

An administrator can switch event types on and off in the registry, and an emitter must not write a type that is switched off ([PGSS §6.9](~peios/pgss/events/emission-policy)). Open a policy handle once, and ask it before building each payload. The tier is the one your fragment declares for the type:

```c
peios_event_policy *policy = peios_event_policy_open();     /* once, at startup */

static const char type[] = "org.example.backup.snapshot.created";
if (peios_event_policy_enabled(policy, type, sizeof type - 1,
                               PEIOS_EVENT_TIER_STANDARD) != 1)
    return;                                                  /* off: build nothing */
```

The handle caches its answers and watches the registry, so asking costs little. A change the administrator commits applies to your next decision. Essential types are always on. See [the emission policy](~peios/sdk-events-api/emitting-events#the-emission-policy) for the details.

## Build the payload

Use the [MessagePack writer](~peios/sdk-msgpack/writer) to encode a single top-level value: a map whose nested maps spell the field paths. This payload carries `outcome.success`:

```c
peios_mp_writer *w = peios_mp_writer_new();

peios_mp_write_map(w, 1);                              /* {outcome: {success: true}} */
peios_mp_write_str(w, "outcome", 7);
peios_mp_write_map(w, 1);
peios_mp_write_str(w, "success", 7);  peios_mp_write_bool(w, true);

const void *payload;
ssize_t plen = peios_mp_writer_bytes(w, &payload);     /* validates as it borrows */
if (plen < 0) { /* EINVAL: malformed/under-filled — check peios_mp_writer_error */ }
```

`peios_mp_writer_bytes` validates that what you built is exactly one well-formed value, so a non-negative return means the payload is emit-ready. (Remember a map of `n` needs `2*n` values — one per key *and* value.)

## Emit it

```c
static const char type[] = "org.example.backup.snapshot.created";

int rc = peios_event_emit(type, sizeof type - 1, payload, (uint32_t)plen);
peios_mp_writer_free(w);

if (rc != 0) {
    switch (errno) {
    case EPERM:  /* no SeAuditPrivilege */          break;
    case EINVAL: /* zero-length type or bad payload */ break;
    case ENOSPC: /* payload too large */            break;
    case EAGAIN: /* rate-limited — back off */      break;
    }
}
```

The event type is length-counted UTF-8 and must be non-zero length. It isn't NUL-terminated on the wire, which is why the example passes `sizeof type - 1`. On success the kernel stamps the trusted metadata (timestamp, sequence, identity GUIDs) and sets `origin_class = userspace`, which queries see as `emitter.class`; you don't provide any of that, and your payload must not repeat it.

### Validating untrusted payloads first

If a payload's shape comes from dynamic or untrusted input, validate it in userspace before emitting so you handle the failure on your terms rather than as an `EINVAL` from the kernel:

```c
if (peios_mp_validate(payload, plen, KMES_CONFIG_MAX_NESTING_DEPTH_DEFAULT) != 0) {
    /* reject it yourself */
}
```

The validator's acceptance matches the kernel's emit-time check at that depth bound.

## Emitting in batches

A high-rate producer should batch. [`peios_event_emit_batch`](~peios/sdk-events-api/emitting-events#batch-emit) emits many events in one call, so a single timestamp capture, identity capture, and consumer wake cover the whole set:

```c
#define EVENT_TYPE(s) s, sizeof(s) - 1

struct peios_event_entry entries[3] = {
    { EVENT_TYPE("org.example.backup.snapshot.created"), pa, pa_len },
    { EVENT_TYPE("org.example.backup.snapshot.created"), pb, pb_len },
    { EVENT_TYPE("org.example.backup.snapshot.deleted"), pc, pc_len },
};

uint32_t emitted = 0;
int rc = peios_event_emit_batch(entries, 3, &emitted);
if (rc != 0) {
    /* errno is from entries[emitted] — the first that failed.
       entries[0..emitted) were emitted; resume from `emitted`. */
}
```

`count` must be in `[1, KMES_BATCH_MAX_ENTRIES]`. On failure, `errno` is the reason the first failing entry failed and `emitted` tells you how many succeeded before it, so you know exactly where to resume. One caveat: rate-limiting is **all-or-nothing** for a batch — an `EAGAIN` emits *none* of it, so on `EAGAIN` back off and retry the whole batch.

## Next

- **[Consuming events](~peios/sdk-events/consuming-events)** — the other side of the pipe.
- **[`msgpack.h` reference](~peios/sdk-msgpack/msgpack-h-messagepack-codec)** — the full encoder, including containers, extensions, and raw splicing.
