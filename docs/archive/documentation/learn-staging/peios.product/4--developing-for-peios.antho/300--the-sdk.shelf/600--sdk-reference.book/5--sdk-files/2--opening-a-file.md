---
title: Opening a file
description: The open params struct — desired access, disposition, create options and creator SD — and what the call returns.
---

> [!IMPORTANT]
> The [open-interface comparison](~peios/advanced-peios/peios-kernel/kacs/kacs-abi-notes#open-interface-documentation-discrepancies) records unresolved differences and the [limited pinned-source findings](~peios/advanced-peios/peios-kernel/kacs/kacs-abi-notes#pinned-source-findings) applied here. Creator-SD rejection, the accepted create-option bits, and native-open versus SD-query no-follow behavior were checked in source; the other differences remain unresolved. No runtime test or historical release boundary is established.

```c
struct peios_open_params {
    uint32_t    desired_access; /* KACS_FILE_* | standard | generic (strict-mode) */
    uint32_t    disposition;    /* KACS_DISPOSITION_* */
    uint32_t    options;        /* KACS_CREATE_OPT_* */
    uint32_t    flags;          /* AT_SYMLINK_NOFOLLOW | KACS_BACKUP_INTENT | KACS_RESTORE_INTENT */
    const void *sd;             /* creator SD on create, else NULL */
    size_t      sd_len;
};

int peios_file_open(int dirfd, const char *path,
                    const struct peios_open_params *p, uint32_t *status_out);
```

`peios_file_open` opens `path` relative to `dirfd` (the usual `*at` convention — an absolute path ignores `dirfd`, and `AT_FDCWD` means the current directory). It returns a file fd, or `-1` with `errno`.

The parameters:

| Field | Meaning |
|---|---|
| `desired_access` | The access mask you are requesting — `KACS_FILE_*` object rights, standard rights, or (in strict mode) generic bits the file class maps. The granted subset is what the returned fd is fixed at. |
| `disposition` | What to do about existence: `KACS_DISPOSITION_*` — open-existing, create-new, open-or-create, supersede, overwrite, and so on. This is the create/open decision `open()` splits across `O_CREAT`/`O_EXCL`/`O_TRUNC`. |
| `options` | At kernel `8e0e22de3a59cad506bbbf8873de456e16ad272d`, only `KACS_CREATE_OPT_DIRECTORY` and `KACS_CREATE_OPT_DELETE_ON_CLOSE` are accepted; other bits fail with `EINVAL`. No-follow belongs in `flags`, not `options`. |
| `flags` | `AT_SYMLINK_NOFOLLOW`, plus the privilege-intent flags `KACS_BACKUP_INTENT` / `KACS_RESTORE_INTENT` that let `SeBackupPrivilege` / `SeRestorePrivilege` widen the access the open is granted. |
| `sd` / `sd_len` | The **creator** security descriptor — the SD to stamp on a newly created file. Pass `NULL` when opening an existing file (or to let the parent's inheritance decide the new file's SD). |

`status_out`, if non-`NULL`, receives a `KACS_STATUS_*` code telling you *what happened* — whether the file was opened, created, superseded, overwritten. For `OPEN_IF` with no creator SD (`sd = NULL`, `sd_len = 0`), this distinguishes "created a new file" from "opened the existing one" without a separate `stat` race. Supplying a creator SD changes the existing-file behavior described below.

Errors include `EACCES` (a requested right denied — strict mode), `EEXIST` (create-new and the file exists), `ENOENT` (open-existing and it doesn't), `ENOTDIR` (directory option, non-directory target), `ELOOP` (native open with no-follow and a final symlink), `EINVAL` (`MAXIMUM_ALLOWED` without a concrete data/execute bit, unsupported create-option bits, malformed creator SD, `NULL` `path`/`p`, `sd == NULL` with `sd_len != 0`), and `EBADF` (bad `dirfd`).

At the pinned kernel revision, supplying a creator SD with `OPEN` fails with
`EOPNOTSUPP`, and with `OVERWRITE` fails with `EINVAL`. With `OPEN_IF` or
`OVERWRITE_IF`, it fails with `EINVAL` when the path resolves to an existing
object. The SDK forwards the disposition and SD; it does not silently drop the
SD on that branch. See the [source findings](~peios/advanced-peios/peios-kernel/kacs/kacs-abi-notes#pinned-source-findings).
Use create-only when an explicit creator SD is required:

```c
struct peios_open_params p = {
    .desired_access = KACS_FILE_READ_DATA | KACS_FILE_WRITE_DATA,
    .disposition    = KACS_DISPOSITION_CREATE,    /* fail if the path exists */
    .options        = 0,
    .sd             = creator_sd, .sd_len = creator_sd_len,
};
uint32_t status = 0;
int fd = peios_file_open(AT_FDCWD, "data.bin", &p, &status);
if (fd < 0) { /* errno */ }
/* On success, status == KACS_STATUS_CREATED. */
```

To open an existing object, use `KACS_DISPOSITION_OPEN` with `sd = NULL` and
`sd_len = 0`. If create-only returns `EEXIST`, decide whether opening the existing
object is appropriate before issuing that separate call. A create/open retry is
not an atomic transaction; neither is a `stat`-then-open check.

At [libpeios `de0018bdcaed14abb796aeccec9c8387fd30e452`](https://github.com/peios/libpeios/blob/de0018bdcaed14abb796aeccec9c8387fd30e452/src/file.rs#L131-L196),
the wrapper copies `options` to `kacs_open_how.create_options` and `flags` to
`kacs_open_how.flags`, forwards the creator SD, zeroes reserved fields, and
passes the struct size as a separate syscall argument. This does not make
unsupported option bits valid on another kernel.
