---
title: Opening and creating keys
description: Opening an existing key or creating one, relative to a parent fd, with a desired-access mask.
---

```c
int peios_reg_open_key(int parent_fd, const char *path, uint32_t desired_access,
                       uint32_t flags);
int peios_reg_create_key(int parent_fd, const char *path, uint32_t desired_access,
                         uint32_t flags, const char *layer, int txn_fd,
                         uint32_t *disposition_out);
```

Both resolve `path` (NUL-terminated) against `parent_fd` — a key fd for a relative path, or `< 0` for an absolute path — and return a key fd **whose granted access mask is fixed for its lifetime** (like a file fd, so it can be delegated). `desired_access` is the requested `KEY_*` rights, checked against the key's SD.

- **`peios_reg_open_key`** opens an *existing* key. `flags` may be `REG_OPEN_LINK` to open a symlink key *itself* rather than following it. Errors: `ENOENT`, `EACCES`, `EINVAL`, `ELOOP`, `ENAMETOOLONG`, `ETIMEDOUT`, `EIO`, `ENOMEM`.
- **`peios_reg_create_key`** opens an existing key or creates a new one. `flags` may combine `REG_OPTION_VOLATILE` (a key that does not survive reboot) and `REG_OPTION_CREATE_LINK` (create a symlink key). `layer` names the target layer to create in (NUL-terminated), or `NULL` for the base layer. `txn_fd` enlists the create in a [transaction](~peios/sdk-registry-api/transactions), or `-1` to auto-commit. `disposition_out`, if non-`NULL`, receives `REG_CREATED_NEW` or `REG_OPENED_EXISTING`. Errors add `ENOSPC` and `EPERM` (privileged symlink creation) to the set above.
