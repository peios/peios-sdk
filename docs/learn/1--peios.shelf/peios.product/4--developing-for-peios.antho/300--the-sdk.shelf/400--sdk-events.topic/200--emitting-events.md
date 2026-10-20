---
title: Emitting Events
type: how-to
description: Build catalog-matching payloads and handle individual or batch emission results.
---

Use the C SDK to encode an event payload and submit it to the kernel.
Choose the type and its catalog definition first; the example follows
[Naming and Shaping Events](~peios/sdk-events/naming-and-shaping-events).
The event and MessagePack headers define the complete API.

## Before you start

- Install matching Peios SDK headers and libraries.
- Run emission calls on a Peios kernel with `SeAuditPrivilege` available
  as required by the access policy.
- Install the matching `.evman` fragment and select the declared tier.
- Include `<peios.h>`, `<errno.h>`, `<stdint.h>`, and `<string.h>` in the
  example's translation unit.

## Check emission policy

1. Open a `peios_event_policy` handle at startup and handle allocation
  failure.
2. Before constructing a payload, call `peios_event_policy_enabled` with its
   type name and declared tier.
3. If the result is `0`, skip the event. If it is `-1`, handle the failure
   according to the policy interface rather than treating it as permission.
4. Close the handle when the emitter shuts down.

The handle caches decisions and watches registry changes. Its check does not
make a later emit operation atomic with a policy update. Essential types are
always on at this layer; their own audit or reporting controls still apply.
See [Emission Policy](~peios/sdk-events-api/emitting-events#the-emission-policy).

## Build a complete payload

The following helper builds a successful snapshot event matching the fragment
in [Writing evman Pages](~peios/developing-for-peios/documenting-events/writing-evman-pages).
Supply a valid UTF-8 archive path, a 16-byte snapshot identifier, the archive
size, and the elapsed nanoseconds. The helper returns an owned writer that
the caller must free.

```c
static peios_mp_writer *snapshot_payload(
    const char *path, const uint8_t snapshot_id[16],
    uint64_t size_bytes, uint64_t elapsed_ns)
{
    if (path == NULL || snapshot_id == NULL) {
        errno = EINVAL;
        return NULL;
    }
    peios_mp_writer *writer = peios_mp_writer_new();
    if (writer == NULL) return NULL;

    peios_mp_write_map(writer, 3);
    peios_mp_write_str(writer, "object", 6);
    peios_mp_write_map(writer, 3);
    peios_mp_write_str(writer, "kind", 4);
    peios_mp_write_str(writer, "file", 4);
    peios_mp_write_str(writer, "file", 4);
    peios_mp_write_map(writer, 2);
    peios_mp_write_str(writer, "path", 4);
    peios_mp_write_str(writer, path, strlen(path));
    peios_mp_write_str(writer, "size", 4);
    peios_mp_write_uint(writer, size_bytes);
    peios_mp_write_str(writer, "org", 3);
    peios_mp_write_map(writer, 1);
    peios_mp_write_str(writer, "example", 7);
    peios_mp_write_map(writer, 1);
    peios_mp_write_str(writer, "backup", 6);
    peios_mp_write_map(writer, 1);
    peios_mp_write_str(writer, "snapshot", 8);
    peios_mp_write_map(writer, 1);
    peios_mp_write_str(writer, "id", 2);
    peios_mp_write_bin(writer, snapshot_id, 16);

    peios_mp_write_str(writer, "operation", 9);
    peios_mp_write_map(writer, 1);
    peios_mp_write_str(writer, "duration", 8);
    peios_mp_write_uint(writer, elapsed_ns);
    peios_mp_write_str(writer, "outcome", 7);
    peios_mp_write_map(writer, 1);
    peios_mp_write_str(writer, "success", 7);
    peios_mp_write_bool(writer, true);

    if (peios_mp_writer_bytes(writer, NULL) < 0) {
        int saved_errno = errno;
        peios_mp_writer_free(writer);
        errno = saved_errno;
        return NULL;
    }
    return writer;
}
```

A map count is the number of key/value pairs. A dotted catalog path becomes
a chain of nested maps, including each segment of the package-owned field.
Writer errors are sticky: check `peios_mp_writer_bytes` before using its
result.
Structural validation does not check catalog meaning or guarantee emission.

For a failed snapshot, keep the same type and identifier. Omit the unknown
archive size, write `outcome.success = false`, and include a declared reason.
Add a negative `outcome.errno` when one applies. Adjust the `file` and
`outcome` map counts to match the fields actually written. Do not append a
second outcome map or another top-level value to an already completed payload.

## Submit and release the payload

After a successful policy check, call the helper and submit its bytes:

```c
static int emit_snapshot(const char *path, const uint8_t snapshot_id[16],
                         uint64_t size_bytes, uint64_t elapsed_ns)
{
    static const char type[] = "org.example.backup.snapshot.created";
    peios_mp_writer *writer = snapshot_payload(
        path, snapshot_id, size_bytes, elapsed_ns);
    if (writer == NULL) return -1;

    const void *payload = NULL;
    ssize_t length = peios_mp_writer_bytes(writer, &payload);
    int result = -1;
    if (length >= 0 && (uint64_t)length <= UINT32_MAX) {
        result = peios_event_emit(
            type, sizeof type - 1, payload, (uint32_t)length);
    } else if (length >= 0) {
        errno = EOVERFLOW;
    }
    int saved_errno = errno;
    peios_mp_writer_free(writer);
    errno = saved_errno;
    return result;
}
```

The bytes are borrowed from the writer until it is changed or freed. The
submission copies them before returning. The length-counted UTF-8 type is
nonempty; the terminating NUL is not part of the name on the wire.

Handle emission errors, including invalid input, insufficient privilege,
oversized payloads, and rate limiting. A successful encoding does not bypass
those checks. On success, the kernel supplies trusted header metadata and
marks the origin as userspace. Your payload remains the emitter's assertion.

## Batch high-rate events

`peios_event_emit_batch` accepts an array of `peios_event_entry` values and
reports the accepted prefix through its `emitted` output. Count is bounded by
`KMES_BATCH_MAX_ENTRIES` and cannot be zero.

If an entry fails, earlier entries can already have been emitted. Do not
replay that prefix as though the batch were a transaction. Rate-limit refusal
is the special all-or-nothing case: `EAGAIN` accepts no entries in that batch.
Keep all names and payload buffers valid until the call returns.

See [Batch Emit](~peios/sdk-events-api/emitting-events#batch-emit) for the
complete signature and error contract. Shared timestamp capture and wakeup
work make batching useful when the application naturally has multiple events
ready; avoid delaying important events merely to fill a batch.

## Check the result

1. Decode a portable fixture and verify its field paths and wire types against
   the catalog, including the required snapshot identifier.
2. Test successful and failed operation payloads separately.
3. In a permitted Peios test environment, confirm policy-controlled omission,
   emission results, and the record observed by a consumer.

[Consuming Events](~peios/sdk-events/consuming-events) explains the read side.
A codec test on another kernel verifies encoding only, not Peios emission or
identity stamping.
