---
title: Backup and restore
description: Streaming a key and everything beneath it out to a descriptor, and restoring it back.
---

```c
int peios_reg_backup(int key_fd, int output_fd);
int peios_reg_restore(int key_fd, int input_fd);
```

- **`peios_reg_backup`** exports the key and its entire subtree to `output_fd` (needs `SeBackupPrivilege`). It takes a read-only snapshot and performs no per-key access check — the privilege is the gate. Errors: `EPERM`/`EACCES`, `EBADF` (output not writable), `ENOENT`, `ENOTSUP`, `EBUSY`.
- **`peios_reg_restore`** replaces the key and its entire subtree from `input_fd` (needs `SeRestorePrivilege`), applied in **one transaction**. Errors: `EPERM`/`EACCES`, `EBADF` (input not readable), `EINVAL` (malformed stream), `EEXIST` (GUID collision), `EOVERFLOW`.
