---
title: Reading and writing a file's security descriptor
description: Getting and setting a file's descriptor by path or by fd, with the secinfo mask that selects which components are touched.
---

A file's SD can be accessed **by path** or **by fd**. In both cases `secinfo` is a mask of `KACS_SECINFO_*` bits selecting which components (owner, group, DACL, SACL, …) the operation touches — you read or write just the parts you name and leave the rest alone.

The rights required scale with the components you touch (see [Managing file security](~peios/file-permissions/managing-file-security)):

| Component (`KACS_SECINFO_*`) | Reading needs | Writing needs |
|---|---|---|
| `OWNER` / `GROUP` | `READ_CONTROL` | `WRITE_OWNER` (plus owner-SID validation) |
| `DACL` | `READ_CONTROL` | `WRITE_DAC` |
| `SACL` | `ACCESS_SYSTEM_SECURITY` | `ACCESS_SYSTEM_SECURITY` |
| `LABEL` | `READ_CONTROL` | `WRITE_OWNER` (the label cannot rise above the caller's integrity without `SeRelabelPrivilege`) |

`ACCESS_SYSTEM_SECURITY` is itself gated by `SeSecurityPrivilege`; `READ_CONTROL` and `WRITE_DAC` are implicitly granted to the owner. `SACL` and `LABEL` cannot be combined in one call (`EINVAL`). The check is all-or-nothing: if any requested component fails its check, the whole call fails.

### By path

```c
ssize_t peios_file_get_sd(int dirfd, const char *path, uint32_t secinfo,
                          void *buf, size_t cap, uint32_t at_flags);
int     peios_file_set_sd(int dirfd, const char *path, uint32_t secinfo,
                          const void *sd, size_t len, uint32_t at_flags);
```

- `peios_file_get_sd` reads the `secinfo`-selected components of `path`'s SD into `buf`, getxattr-style ([two-call protocol](~peios/sdk-conventions/the-two-call-buffer-protocol) — probe with `cap == 0`, and a too-small non-zero buffer fails `ERANGE` without truncating). `at_flags` accepts `AT_SYMLINK_NOFOLLOW`; at the pinned source below, it queries the terminal symlink's own SD instead of following it. Errors: `EACCES` (component right missing), `EINVAL` (`SACL` + `LABEL` together; `NULL` path, or `NULL` buffer with non-zero `cap`), `ERANGE` (non-probe buffer too small), `ENOENT` (path doesn't exist), `ELOOP` (a path-resolution failure, not a blanket rejection of a terminal symlink under no-follow).
- `peios_file_set_sd` writes the `secinfo` components of `sd` onto `path`, **preserving the components you did not select**. So to change only the DACL, build an SD with a DACL, pass `secinfo = KACS_SECINFO_DACL`, and the owner/group/SACL are untouched. Errors: `EACCES` (component right missing), `EPERM` (owner-SID validation failed without `SeRestorePrivilege`; label raised without `SeRelabelPrivilege`; MANDATORY attribute removed without `SeTcbPrivilege`), `EINVAL` (malformed SD, `SACL` + `LABEL` together, `NULL` or zero-length `sd`), `ENOENT`, `ELOOP`.

> [!IMPORTANT]
> No-follow has different meanings for these two operations: native open rejects a terminal symlink with `ELOOP`, while the by-path SD query selects the link itself. This is verified in [kernel `8e0e22de3a59cad506bbbf8873de456e16ad272d`](https://github.com/peios/pkm/blob/8e0e22de3a59cad506bbbf8873de456e16ad272d/kacs/sd_access.c#L312-L327) and its [path resolver](https://github.com/peios/pkm/blob/8e0e22de3a59cad506bbbf8873de456e16ad272d/kacs/sd_access.c#L506-L536). The [libpeios 0.5.8 query wrapper](https://github.com/peios/libpeios/blob/de0018bdcaed14abb796aeccec9c8387fd30e452/src/file.rs#L220-L312) at `de0018bdcaed14abb796aeccec9c8387fd30e452` forwards `at_flags`; it does not turn no-follow into a link rejection. The [link-query test definition](https://github.com/peios/pkm/blob/8e0e22de3a59cad506bbbf8873de456e16ad272d/kacs/kunit_file.c#L7586-L7625) is supporting source evidence, not a test run. This query finding does not establish a successful link-SD write: set-security has separate rights, validation and storage requirements. No deployed version or historical release boundary was verified.

### By fd

```c
ssize_t peios_fd_get_sd(int fd, uint32_t secinfo, void *buf, size_t cap);
int     peios_fd_set_sd(int fd, uint32_t secinfo, const void *sd, size_t len);
```

The same operations against the object `fd` already refers to. The access check they perform depends on the fd type: a normal file fd is checked against its **cached granted mask** (the one baked in at open), while an `O_PATH`, pidfd, or token fd triggers a **live** check. That distinction — cached for the fixed-grant file fd, live for the others — is documented in the Peios Kernel TRM §3.9, FACS; the practical upshot is that a file fd already opened with the right access can get/set its SD without a second path resolution.

The required rights and errors match the by-path calls, minus the path-resolution failures (`ENOENT`/`ELOOP`), plus `EBADF` (bad fd).

### By System V IPC kind and id

```c
ssize_t peios_sysv_get_sd(uint32_t kind, int id, uint32_t secinfo,
                          void *buf, size_t cap);
int     peios_sysv_set_sd(uint32_t kind, int id, uint32_t secinfo,
                          const void *sd, size_t len);
```

System V message queues, shared memory segments and semaphore arrays carry security descriptors too (Peios Kernel TRM §3.11), but have neither a path nor an fd. `kind` is one of `KACS_SD_AT_SYSV_SHM`, `KACS_SD_AT_SYSV_MSG` or `KACS_SD_AT_SYSV_SEM` from `<pkm/ipc.h>`, and `id` is what `shmget`, `msgget` or `semget` returned. The object is looked up in the caller's IPC namespace and the check is always live against its descriptor: `READ_CONTROL` (or `ACCESS_SYSTEM_SECURITY` for the SACL) to read, `WRITE_DAC` / `WRITE_OWNER` to write, with the same preserve-unselected-components merge as the file calls. Errors as for the by-fd calls, with `EINVAL` for an id that names nothing and `EIDRM` for an object that has been removed. The Rust crate exposes the same pair as `file::sysv_get_sd` / `file::sysv_set_sd` with a `SysvKind` enum.
