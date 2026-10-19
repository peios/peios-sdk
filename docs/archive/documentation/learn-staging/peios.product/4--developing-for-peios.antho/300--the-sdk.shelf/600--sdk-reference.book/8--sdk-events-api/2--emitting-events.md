---
title: Emitting events
description: Emitting a single event or a batch — the type string, the payload, and what the call validates.
---

```c
int peios_event_emit(const char *event_type, uint16_t event_type_len,
                     const void *payload, uint32_t payload_len);
```

Emits a single event. `event_type` is a **length-counted UTF-8** event type such as `"org.example.backup.snapshot.created"`, named as PGSS §6.3 requires — *not* NUL-terminated, and its length must be non-zero. `payload` is `payload_len` bytes of MessagePack (one well-formed value). The kernel validates the payload (one well-formed MessagePack value within the configured size and nesting limits) and stamps `origin_class = userspace`. Returns `0`, or `-1` with `errno`:

| errno | Cause |
|---|---|
| `EPERM` | No `SeAuditPrivilege`. |
| `EINVAL` | Zero-length type, or a malformed payload. |
| `ENOSPC` | Payload exceeds the size caps. |
| `EAGAIN` | Rate-limited. |
| `EFAULT` | Bad pointer. |

Since the kernel's payload check matches [`peios_mp_validate`](~peios/sdk-msgpack/validator), you can validate in userspace first and turn a would-be `EINVAL` into a check you control.

```c
/* Build a payload, then emit. */
peios_mp_writer *w = peios_mp_writer_new();
peios_mp_write_map(w, 1);                      /* {outcome: {success: true}} */
peios_mp_write_str(w, "outcome", 7);
peios_mp_write_map(w, 1);
peios_mp_write_str(w, "success", 7); peios_mp_write_bool(w, true);

static const char type[] = "org.example.backup.snapshot.created";
const void *buf; ssize_t n = peios_mp_writer_bytes(w, &buf);
if (n >= 0)
    peios_event_emit(type, sizeof type - 1, buf, (uint32_t)n);
peios_mp_writer_free(w);
```

### The emission policy

```c
#define PEIOS_EVENT_TIER_ESSENTIAL 0u
#define PEIOS_EVENT_TIER_STANDARD  1u
#define PEIOS_EVENT_TIER_VERBOSE   2u
#define PEIOS_EVENT_TIER_DEBUG     3u

typedef struct peios_event_policy peios_event_policy;

peios_event_policy *peios_event_policy_open(void);
void peios_event_policy_close(peios_event_policy *policy);
int  peios_event_policy_enabled(peios_event_policy *policy, const char *event_type,
                                uint16_t event_type_len, uint32_t tier);
```

An emitter must not write an event whose type the emission policy has switched off ([PGSS §6.9](~peios/pgss/events/emission-policy)), and should find that out before it builds the payload. `peios_event_policy_enabled` answers it: `1` means build and emit, `0` means build nothing. `tier` is the tier the type declares in its fragment. An essential type is always `1`, without reading anything.

`peios_event_emit` does not check the policy itself. By the time it is called the payload is built, and skipping that work is the point of asking.

The handle caches each type's answer and watches `Machine\Generic\Events`, so a change an administrator commits applies to your next decision. If that key does not exist yet, it watches for its creation; if nothing can be watched, it reads the policy again at least once a second. If the registry cannot be read at all, the answer is the tier's default: on for `standard`, off for `verbose` and `debug`.

`peios_event_policy_open` fails only with `ENOMEM`; it does not need the registry to be there. `peios_event_policy_enabled` returns `-1` with `EINVAL` only for caller error: a `NULL` handle or type, a malformed type (empty, not UTF-8, an empty segment, or a `\`, `/` or NUL in a segment), or an unknown tier. Registry failures never make it fail.

Use a handle from one thread at a time, and don't use it across `fork()`: the child shares the parent's watch descriptor and would take the parent's notifications. A child opens its own. The Rust binding's `peios::event::EventPolicy` adds a lock so it can be shared between threads. The independent [Go SDK](~peios/sdk-basics/using-the-sdk-from-go) provides `event.Policy`, which reads policy afresh on each call and retains no watch handle.

### Batch emit

```c
struct peios_event_entry {
    const char *event_type;      /* length-counted UTF-8; not NUL-terminated */
    uint16_t    event_type_len;
    const void *payload;         /* MessagePack bytes */
    uint32_t    payload_len;
};

int peios_event_emit_batch(const struct peios_event_entry *entries,
                           uint32_t count, uint32_t *emitted_out);
```

`peios_event_emit_batch` emits several events in one call, **amortising the per-call overhead** — a single timestamp capture, identity capture, and consumer wake cover the whole batch. `count` is in `[1, KMES_BATCH_MAX_ENTRIES]`. It returns `0` if all `count` were emitted, or `-1` with the `errno` **of the first entry that failed**, with `*emitted_out` (if non-`NULL`) set to how many entries preceded the failure — so you know exactly where to resume. Rate-limiting is all-or-nothing here: an `EAGAIN` emits **none** of the batch.
