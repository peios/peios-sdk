---
title: Transactions
description: Batching key creates and mutating operations into an atomic unit — beginning, committing and aborting one.
---

A transaction batches key creates and mutating value/key operations into an atomic unit.

```c
int peios_reg_begin_transaction(void);
int peios_reg_commit(int txn_fd);
int peios_reg_txn_status(int txn_fd, uint32_t *state_out, int *terminal_errno_out);
```

- **`peios_reg_begin_transaction`** starts one and returns a transaction fd (initially unbound; it binds to a source on first use), or `-1`/`ENOMEM`. Pass this fd as the `txn_fd` argument to the create and mutating calls to enlist them. **Closing the fd without committing aborts** the transaction — so a transaction is abort-by-default, which makes error paths safe.
- **`peios_reg_commit`** atomically applies everything enlisted. On success the fd is **terminal** — close it. Errors tell you what to do: `EINVAL` (already committed / never bound), `EBUSY` (write-lock contention — the transaction **stays active**, retry the commit), `EIO` (source failure — stays active), `ETIMEDOUT`.
- **`peios_reg_txn_status`** reads a transaction's state: `state_out` receives the `REG_TXN_*` state, and `terminal_errno_out` receives the errno that ended it (`0` while active or after a clean commit). Both may be `NULL`.

The lifecycle: `begin` → enlist operations by passing `txn_fd` → `commit` (retry on `EBUSY`/`EIO`) → `close`, or just `close` to abort.
