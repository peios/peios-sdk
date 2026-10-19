---
title: Watching and transactions
type: how-to
description: React to registry changes through a pollable key fd, and apply multi-step edits atomically with transactions.
related:
  - peios/sdk-registry-api/registry-h-the-registry-lcs
  - peios/sdk-registry/reading-and-writing
---

Beyond simple reads and writes, LCS gives you two higher-order tools: **watches** to react to change, and **transactions** to apply several edits atomically. Both are in [`registry.h`](~peios/sdk-registry-api/registry-h-the-registry-lcs); this guide shows the shape of using them. To keep that shape visible, the fragments elide most error checks — every call returns `-1` with `errno` on failure, and real code must check each one (see [Library conventions](~peios/sdk-conventions/library-conventions)).

## Watching for changes

Ask a key to notify you when it changes, then poll its fd. Because the armed key fd is itself pollable, a watch integrates directly into whatever event loop you already run:

```c
int key = peios_reg_open_key(-1, "/System/MyApp", KEY_READ | KEY_NOTIFY, 0);

/* Watch values and subkeys, across the whole subtree. */
peios_reg_notify(key, REG_NOTIFY_VALUE | REG_NOTIFY_SUBKEY, 1 /* subtree */);

struct pollfd pfd = { .fd = key, .events = POLLIN };
for (;;) {
    poll(&pfd, 1, -1);
    if (pfd.revents & POLLIN) {
        /* read() the key fd to drain the change records, then re-read
           whatever you care about. */
        char records[512];
        ssize_t n = read(key, records, sizeof records);
        /* ... process change records ... */
    }
}
```

`filter` is a mask of `REG_NOTIFY_VALUE`, `REG_NOTIFY_SUBKEY`, and `REG_NOTIFY_SD`; `REG_NOTIFY_ALL` covers all three. The `subtree` flag extends the watch to descendants. Arming needs `KEY_NOTIFY` on the key. Call `peios_reg_notify(key, 0, 0)` to disarm.

Each `read()` drains as many complete change records as fit — every record starts `[total_len: u32][event_type: u16][name_len: u16][name]`, so you step through the buffer by `total_len`. The full layout, the `REG_WATCH_*` event types, and the extra path fields a subtree watch appends are in [the reference](~peios/sdk-registry-api/subkeys-metadata-and-watches#watching-for-changes). Two practical notes: a buffer too small for even one record fails `EINVAL`, so size it generously rather than exactly; and a `REG_WATCH_OVERFLOW` record means events were dropped — re-read the key's state instead of trusting the stream.

This is how a service picks up configuration changes live — no polling loop re-reading values on a timer, just a blocking `poll` that wakes when something actually changed.

## Transactions

When a configuration change spans several operations — create a key, set a few values, delete a stale one — you usually want it to be all-or-nothing. That's a transaction.

Begin one, pass its fd as the `txn_fd` argument to each operation you want enlisted, then commit:

```c
int txn = peios_reg_begin_transaction();

int key = peios_reg_create_key(-1, "/System/MyApp/v2", KEY_WRITE, 0,
                               NULL /* base layer */, txn, NULL);
uint32_t one = 1;
peios_reg_set_value(key, "Enabled", 7, REG_DWORD, &one, sizeof one,
                    NULL, 0, txn, 0);
peios_reg_delete_value(key, "LegacyFlag", 10, NULL, 0, txn);

int rc = peios_reg_commit(txn);
if (rc == 0) {
    /* everything applied atomically; txn is terminal — close it */
} else if (errno == EBUSY || errno == EIO) {
    /* transient: the transaction is still active — retry the commit */
}
close(txn);
close(key);
```

Two things to keep in mind:

- **Abort is the default.** If you close the transaction fd without committing — including on any early-return error path — nothing is applied. So you don't need explicit rollback logic; just don't commit.
- **Commit can be retried.** `EBUSY` (write-lock contention) and `EIO` (source failure) leave the transaction **active**, so you can retry `peios_reg_commit`. Only `0` (committed — the fd is now terminal) and `EINVAL` (already committed or never bound) are final. Check state at any point with [`peios_reg_txn_status`](~peios/sdk-registry-api/transactions).

## Backup and restore

To snapshot a key and its whole subtree, or replace one from a snapshot, use [`peios_reg_backup`](~peios/sdk-registry-api/backup-and-restore) and `peios_reg_restore`. They stream to and from an fd and are gated by `SeBackupPrivilege` / `SeRestorePrivilege`; restore applies in a single transaction.

## Next

- **[`registry.h` reference](~peios/sdk-registry-api/registry-h-the-registry-lcs)** — the notify filters, transaction states, and every error in full.
- **[Reading and writing](~peios/sdk-registry/reading-and-writing)** — the value operations you enlist in a transaction.
