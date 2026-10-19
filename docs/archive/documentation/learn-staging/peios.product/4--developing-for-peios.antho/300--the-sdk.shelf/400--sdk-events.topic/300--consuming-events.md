---
title: Consuming events
type: how-to
description: Drain the per-CPU KMES rings with the high-level reader, parse payloads, track lost events, and know when to drop to the low-level ring API.
related:
  - peios/sdk-events-api/event-h-events-kmes
  - peios/sdk-msgpack/msgpack-h-messagepack-codec
  - peios/auditing/overview
---

Consuming events means draining the per-CPU ring buffers. The SDK's high-level reader hides all the hard parts, so most consumers are a short loop. This guide shows that loop and how to parse what you read; [`event.h`](~peios/sdk-events-api/event-h-events-kmes) has the full API. Consuming requires `SeSecurityPrivilege`.

## The reader loop

Open a reader for a CPU, then loop `next`/`wait`:

```c
peios_event_reader *r = peios_event_reader_open(cpu);
if (!r) { /* errno */ }

for (;;) {
    struct peios_event ev;
    int rc = peios_event_reader_next(r, &ev);
    if (rc == 1) {
        handle_event(&ev);                       /* got one */
    } else if (rc == 0) {
        peios_event_reader_wait(r, -1);          /* none right now — sleep */
    } else {
        break;                                   /* error */
    }
}
peios_event_reader_close(r);
```

`peios_event_reader_next` returns `1` (event filled), `0` (nothing available — call `wait`), or `-1` (error). `peios_event_reader_wait` blocks until events arrive or the timeout elapses (negative = forever). The reader handles the memory barriers, lapping recovery, buffer-resize handling, and futex wait internally — you just alternate the two calls.

## Parsing an event

Each `struct peios_event` gives you the trusted metadata by value and the payload as a MessagePack value. Parse it with a [reader](~peios/sdk-msgpack/reader):

```c
void handle_event(const struct peios_event *ev)
{
    /* Metadata is trustworthy — the kernel stamped it. */
    /* ev->timestamp, ev->sequence, ev->cpu_id, ev->origin_class,
       ev->process_guid, ev->effective_token_guid, ... */

    /* event_type and payload are NOT NUL-terminated — use the lengths. */
    static const char type[] = "org.example.backup.snapshot.created";

    if (ev->event_type_len == sizeof type - 1 &&
        memcmp(ev->event_type, type, sizeof type - 1) == 0) {

        struct peios_mp_reader mp;
        peios_mp_reader_init(&mp, ev->payload, ev->payload_len);

        ssize_t pairs = peios_mp_read_map(&mp);
        for (ssize_t i = 0; i < pairs; i++) {
            const char *key; ssize_t klen = peios_mp_read_str(&mp, &key);
            /* dispatch on key; read or peios_mp_skip(&mp) the value */
        }
    }
}
```

### The lifetime rule

`ev->event_type` and `ev->payload` point **into the ring mapping**. They are valid only until your next `peios_event_reader_next` call, and only while the slot hasn't been overwritten. So **copy out anything you need to keep** before continuing the loop — don't stash the raw pointers.

## Draining every CPU

Rings are per-CPU, so one reader sees only one CPU's events. To consume the whole machine, run a reader per CPU — typically one thread each. Discover the CPU count by attaching upward until it fails:

```c
uint32_t ncpu = 0;
for (;;) {
    uint64_t cap;
    int fd = peios_event_attach(ncpu, &cap);   /* low-level probe */
    if (fd < 0) break;                          /* EINVAL past the last CPU */
    close(fd);
    ncpu++;
}
/* now spawn one peios_event_reader per cpu in [0, ncpu) */
```

## Watching for loss

If a consumer falls behind, the producer laps it and events are lost. Poll [`peios_event_reader_lost`](~peios/sdk-events-api/consuming-events#the-high-level-reader) to see the cumulative count of lost events (derived from sequence gaps). A rising number means you aren't draining fast enough — process events more cheaply, hand off to a worker, or accept the loss deliberately.

## The low-level ring

If the reader's loop doesn't fit your event model — you want the ring integrated into an existing `epoll`/state-machine loop, driving the read position yourself — the [low-level ring API](~peios/sdk-events-api/consuming-events#the-low-level-ring) exposes the mapping directly: `peios_event_ring_map`, the position accessors (`write_pos`/`tail_pos`/`generation`), `peios_event_ring_event_at` to parse a slot, and `peios_event_ring_wait` to sleep. It's more bookkeeping (you own the read position and the empty/lapping/generation checks) for more control. Reach for it only when you need to; the high-level reader is the right default.

## Next

- **[`event.h` reference](~peios/sdk-events-api/event-h-events-kmes)** — the full reader and ring APIs, and every `struct peios_event` field.
- **[Auditing](~peios/auditing/overview)** — the operator-side view of the event and audit stream.
