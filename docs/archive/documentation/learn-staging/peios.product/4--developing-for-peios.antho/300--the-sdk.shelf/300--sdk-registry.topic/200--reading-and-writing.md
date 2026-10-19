---
title: Reading and writing
type: how-to
description: Open a registry key, read its effective values, write into a layer, enumerate, and do lost-update-safe updates with compare-and-swap.
related:
  - peios/sdk-registry-api/registry-h-the-registry-lcs
  - peios/sdk-registry/watching-and-transactions
---

This guide covers the bread-and-butter registry operations. The full call signatures and error sets are in [`registry.h`](~peios/sdk-registry-api/registry-h-the-registry-lcs); here we string them together. Recall the registry's [descriptor-struct buffer convention](~peios/sdk-registry-api/the-buffer-convention-here): reads fill `*_cap`/`*_len` fields and a zero-capacity buffer probes.

The fragments below elide some error checks for brevity — every call returns `-1` with `errno` on failure, and real code must check each one (see [Library conventions](~peios/sdk-conventions/library-conventions)).

## Opening a key

Open a key for the rights you need:

```c
int key = peios_reg_open_key(-1, "/System/MyApp", KEY_READ, 0);
if (key < 0) {
    if (errno == ENOENT) { /* not there */ }
    else if (errno == EACCES) { /* not allowed */ }
    return -1;
}
```

`parent_fd == -1` means `path` is absolute. To open relative to a key you already hold, pass that key fd as the parent. Use [`peios_reg_create_key`](~peios/sdk-registry-api/opening-and-creating-keys) instead when the key might not exist yet — it opens-or-creates and reports which happened.

## Reading the effective value

Reading resolves layer precedence for you and hands back the winning value, its type, and which layer it came from:

```c
struct peios_reg_value v = {0};
unsigned char data[256];
v.data = data;  v.data_cap = sizeof data;
/* leave v.layer NULL if you don't care which layer won */

if (peios_reg_query_value(key, "Timeout", 7, -1, &v) == 0) {
    /* v.type is REG_*, v.data_len bytes valid in `data`, v.sequence is
       the effective entry's sequence number. */
} else if (errno == ENOENT) {
    /* no effective value (or a tombstone masks it) */
} else if (errno == ERANGE) {
    /* data buffer too small; v.data_len holds the required size — grow & retry */
}
```

The `name_len` is explicit (value names are length-counted; `0` reads the key's default value). Pass a transaction fd as the fourth argument to read within a transaction, or `-1` for none.

To read *every* value at once, use [`peios_reg_query_values_batch`](~peios/sdk-registry-api/values#enumerating-values) — one call fills a buffer with all effective values in a packed record format. To walk them one at a time, loop `peios_reg_enum_value` from index `0` until `ENOENT`.

## Writing a value

Writes target a specific layer. Pass `NULL`/`0` for the layer to write the base layer:

```c
uint32_t timeout = 30;
int rc = peios_reg_set_value(key, "Timeout", 7, REG_DWORD,
                             &timeout, sizeof timeout,
                             NULL, 0,      /* base layer */
                             -1,           /* auto-commit (no transaction) */
                             0);           /* no CAS guard */
```

Writing to a higher-precedence layer overrides lower ones without destroying them; [deleting](~peios/sdk-registry-api/values#writing-deleting-tombstoning) that layer's entry later lets the lower value re-emerge.

## Safe updates with compare-and-swap

To read-modify-write without clobbering a concurrent change, feed the `sequence` you read back as the `expected_seq` guard on the write. The write only lands if nothing changed underneath you; otherwise it fails with `EAGAIN` and you retry:

```c
for (;;) {
    struct peios_reg_value v = {0};
    unsigned char buf[64]; v.data = buf; v.data_cap = sizeof buf;
    if (peios_reg_query_value(key, "Counter", 7, -1, &v) != 0)
        break;                                     /* real error — don't spin */

    uint32_t n; memcpy(&n, buf, sizeof n); n++;

    int rc = peios_reg_set_value(key, "Counter", 7, REG_DWORD,
                                 &n, sizeof n, NULL, 0, -1,
                                 v.sequence);          /* CAS on the sequence */
    if (rc == 0) break;                                /* success */
    if (errno != EAGAIN) { /* real error */ break; }   /* else: retry */
}
```

Passing `expected_seq == 0` disables the guard (an unconditional write).

## Cleaning up

Close key fds with `close()` when done. Values you wrote with auto-commit (`txn_fd == -1`) are already durable to the layer; to force the source to persist a hive's pending writes at a known point, call [`peios_reg_flush`](~peios/sdk-registry-api/subkeys-metadata-and-watches#watching-for-changes).

## Next

- **[Watching and transactions](~peios/sdk-registry/watching-and-transactions)** — react to changes and batch atomic edits.
- **[`registry.h` reference](~peios/sdk-registry-api/registry-h-the-registry-lcs)** — every call, field, and error.
