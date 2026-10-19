---
title: Consuming events
description: Attaching to a ring and reading events — the high-level reader, the low-level ring, and what a consumed event points into.
---

A consumed event is described by `struct peios_event`. The kernel-stamped header is copied to you by value; the two variable parts point into the ring mapping.

```c
struct peios_event {
    uint64_t timestamp;                  /* ns since the Unix epoch (CLOCK_REALTIME) */
    uint64_t sequence;                   /* per-CPU, per-boot monotonic (gap = lost events) */
    uint16_t cpu_id;
    uint8_t  origin_class;               /* 0 = userspace, 1 = KMES, 2 = KACS, 3 = LCS */
    uint8_t  effective_token_guid[16];
    uint8_t  true_token_guid[16];
    uint8_t  process_guid[16];
    const char *event_type;              /* not NUL-terminated; use event_type_len */
    uint16_t    event_type_len;
    const void *payload;                 /* a MessagePack value */
    uint32_t    payload_len;
};
```

The trusted metadata is the point of KMES: the `timestamp`, the identity GUIDs (the effective and true tokens, and the process), and the `origin_class` are stamped by the kernel and cannot be forged by the emitter. **`sequence` is per-CPU, per-boot monotonic — a gap in it means events were lost** (overwritten before you drained them).

> **Lifetime:** `event_type` and `payload` point into the ring mapping and are valid **only until the next read advance**, and only while the slot has not been overwritten. Copy out whatever you need before continuing to the next event.

### Attaching to a ring

```c
int peios_event_attach(uint32_t cpu_id, uint64_t *capacity_out);
```

The low-level primitive: attach to CPU `cpu_id`'s ring buffer, returning a fd and writing the data-region capacity to `*capacity_out`. **Discover the CPU count** by counting up from `0` until `peios_event_attach` returns `-1` with `errno == EINVAL`. Requires `SeSecurityPrivilege` (`EPERM` otherwise). You then `mmap` the fd via [`peios_event_ring_map`](#the-low-level-ring). Most callers should use the high-level reader instead, which does the attach and mmap for you.

### The high-level reader

```c
typedef struct peios_event_reader peios_event_reader;

peios_event_reader *peios_event_reader_open(uint32_t cpu_id);
void                peios_event_reader_close(peios_event_reader *r);
int      peios_event_reader_next(peios_event_reader *r, struct peios_event *out);
int      peios_event_reader_wait(peios_event_reader *r, int timeout_ms);
uint64_t peios_event_reader_lost(const peios_event_reader *r);
```

The reader owns the attach + mmap and hides the whole lock-free drain — memory barriers, lapping recovery, sequence-gap (lost-event) accounting, buffer resize/generation handling, and the futex wait. **You just loop `next`/`wait`.**

- `peios_event_reader_open` attaches to `cpu_id` and maps its ring, ready to drain (`NULL` with `errno` on failure). `peios_event_reader_close` tears it down.
- `peios_event_reader_next` fetches the next event into `out` (non-`NULL`). Returns **`1`** (event filled), **`0`** (none available right now — consider `wait`), or **`-1`** with `errno`. The `out` pointers are valid only until the next call.
- `peios_event_reader_wait` blocks until events are available or `timeout_ms` elapses (**negative = forever**). Returns `1` (call `next`), `0` (timeout/interrupted), or `-1`.
- `peios_event_reader_lost` returns the cumulative count of lost events (from sequence gaps) — poll it to monitor whether you're draining fast enough.

The canonical consume loop, per CPU:

```c
peios_event_reader *r = peios_event_reader_open(cpu);
for (;;) {
    struct peios_event ev;
    int rc = peios_event_reader_next(r, &ev);
    if (rc == 1) {
        /* handle ev — copy out event_type/payload before the next call */
    } else if (rc == 0) {
        peios_event_reader_wait(r, -1);   /* sleep until more arrive */
    } else {
        break;                            /* error */
    }
}
peios_event_reader_close(r);
```

To consume the whole machine, run one reader per CPU (discover the count as above), each typically on its own thread.

### The low-level ring

For callers that want to drive the drain themselves — integrating the rings into a custom event loop, say — the ring API exposes the mapping directly. The accessors apply the correct memory barriers; **you** own the read position and the empty/lapping/generation checks.

```c
struct peios_event_ring { uint64_t _opaque[4]; };   /* opaque */

int  peios_event_ring_map(int fd, uint64_t capacity, struct peios_event_ring *ring);
void peios_event_ring_unmap(struct peios_event_ring *ring);

uint64_t peios_event_ring_capacity(const struct peios_event_ring *ring);
uint64_t peios_event_ring_write_pos(const struct peios_event_ring *ring);  /* acquire */
uint64_t peios_event_ring_tail_pos(const struct peios_event_ring *ring);   /* acquire */
uint64_t peios_event_ring_generation(const struct peios_event_ring *ring);
void     peios_event_ring_set_need_wake(const struct peios_event_ring *ring, int set);

ssize_t peios_event_ring_event_at(const struct peios_event_ring *ring,
                                  uint64_t read_pos, struct peios_event *out);
int     peios_event_ring_wait(const struct peios_event_ring *ring,
                              uint64_t read_pos, int timeout_ms);
```

- `peios_event_ring_map` maps and validates a ring fd from `peios_event_attach`; `ring` must be zeroed or previously unmapped (remapping an active ring fails `EBUSY`). `peios_event_ring_unmap` releases it.
- **Positions are free-running byte counters.** `write_pos` is where the producer will write next (acquire-loaded); `tail_pos` is the oldest still-live byte (advances as the ring laps); an event lives at `(read_pos & (capacity - 1))`. You drain by walking `read_pos` from `tail_pos` toward `write_pos`. `generation` changes when the buffer is resized — re-read `capacity` when it does.
- `peios_event_ring_event_at` parses the event at `read_pos` into `out` and returns its **byte size** (advance `read_pos` by that), or `-1` if the slot is corrupt. You must have confirmed `read_pos` is in `[tail_pos, write_pos)` first. Pass `out == NULL` to validate a slot and get its size without borrowing the `event_type`/`payload` pointers.
- Before sleeping, arm the advisory wake flag with `peios_event_ring_set_need_wake(ring, 1)`, then `peios_event_ring_wait` futex-waits until events past `read_pos` may be available or `timeout_ms` elapses (negative = forever): `1` (drain now), `0` (timeout/interrupted), `-1`.

The low-level loop mirrors the high-level one but with the position bookkeeping in your hands:

```c
uint64_t rp = peios_event_ring_tail_pos(&ring);
for (;;) {
    uint64_t wp = peios_event_ring_write_pos(&ring);
    while (rp < wp) {
        struct peios_event ev;
        ssize_t sz = peios_event_ring_event_at(&ring, rp, &ev);
        if (sz < 0) { /* corrupt slot — resync from tail_pos */ break; }
        /* handle ev */
        rp += (uint64_t)sz;
    }
    peios_event_ring_set_need_wake(&ring, 1);
    peios_event_ring_wait(&ring, rp, -1);
}
```

Reach for this only when the high-level reader's loop doesn't fit your event model; for almost everything, `peios_event_reader_*` is the right tool.
