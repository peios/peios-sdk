---
title: Securing files
type: how-to
description: Open files the native KACS way, and read and write a file's security descriptor by path or by fd.
related:
  - peios/sdk-files/file-h-file-security
  - peios/sdk-security/security-h-security-descriptors
  - peios/file-access/overview
---

Peios files carry real security descriptors, and the SDK opens them with a native KACS open rather than POSIX `open()`. This guide covers the two everyday tasks: opening a file with a specific access, and reading or changing a file's security. The full surface is in [`file.h`](~peios/sdk-files/file-h-file-security).

To keep the fragments readable, most error checks are elided here — every call below returns `-1` with `errno` on failure, and real code must check each one (see [Library conventions](~peios/sdk-conventions/library-conventions)).

## The native open

[`peios_file_open`](~peios/sdk-files/opening-a-file) is shaped like `NtCreateFile`: you state the access you want, what to do about existence (the *disposition*), any create options, and — when creating — the security descriptor to stamp on the new file. It returns an ordinary Linux fd whose **granted access is fixed for the fd's lifetime**, which means you can safely hand it to another process by `SCM_RIGHTS`, `dup`, or across `exec`: the fd carries exactly the access it was opened with.

> [!IMPORTANT]
> At kernel `8e0e22de3a59cad506bbbf8873de456e16ad272d`, `OPEN_IF` or `OVERWRITE_IF` with a creator SD fails with `EINVAL` for an existing path. Plain `OPEN` with a creator SD fails with `EOPNOTSUPP`, and `OVERWRITE` with one fails with `EINVAL`. The [pinned source findings](~peios/advanced-peios/peios-kernel/kacs/kacs-abi-notes#pinned-source-findings) resolve these cases, not every [open-interface discrepancy](~peios/advanced-peios/peios-kernel/kacs/kacs-abi-notes#open-interface-documentation-discrepancies). No deployed build was tested.

Create a new file, readable and writable, with a creator SD. An existing path is
an error:

```c
struct peios_open_params p = {
    .desired_access = KACS_FILE_READ_DATA | KACS_FILE_WRITE_DATA,
    .disposition    = KACS_DISPOSITION_CREATE,    /* fail if the path exists */
    .sd             = creator_sd, .sd_len = creator_sd_len,
};

uint32_t status = 0;
int fd = peios_file_open(AT_FDCWD, "state.db", &p, &status);
if (fd < 0) { perror("open"); return -1; }

/* On success, status == KACS_STATUS_CREATED. */
```

To open an existing file, use `KACS_DISPOSITION_OPEN` with `sd = NULL` and
`sd_len = 0`. If the create-only call returns `EEXIST`, decide whether accepting
the existing object is appropriate before making that separate open call.
The two calls are not an atomic create-or-open transaction; checking `stat`
first does not remove the race either.

If parent inheritance is appropriate for a newly created file, `OPEN_IF` with
`sd = NULL` and `sd_len = 0` can report created versus opened in `status_out`
without a separate `stat`. Only `DIRECTORY` and `DELETE_ON_CLOSE` create-option
bits are accepted by the pinned kernel; put no-follow in `flags`. See the
[opening reference](~peios/sdk-files/opening-a-file) for the SDK fields.

## Reading a file's security descriptor

To see who can do what to a file, read its SD. `secinfo` selects which components you want — owner, group, DACL, SACL — so you fetch only what you need:

```c
/* Probe, allocate, read (two-call). */
ssize_t need = peios_file_get_sd(AT_FDCWD, "state.db",
                                 KACS_SECINFO_OWNER | KACS_SECINFO_DACL,
                                 NULL, 0, 0);
void *sd = malloc(need);
peios_file_get_sd(AT_FDCWD, "state.db",
                  KACS_SECINFO_OWNER | KACS_SECINFO_DACL, sd, need, 0);

/* Parse it with a security.h view. */
peios_sd_view v;
peios_sd_parse(sd, need, &v);
peios_acl_view dacl;
if (peios_sd_view_dacl(&v, &dacl) == 0) {
    unsigned n = peios_acl_view_count(&dacl);
    /* iterate ACEs … */
}
free(sd);
```

If you already hold a file fd, use the fd-targeted [`peios_fd_get_sd`](~peios/sdk-files/reading-and-writing-a-file-s-security-descriptor#by-fd) instead of a path — no second path resolution, and for a normal file fd the check uses the access already baked in at open.

## Changing a file's security descriptor

Writing an SD is component-selective too: name the components you're changing in `secinfo`, and everything you *don't* name is preserved. To tighten a file's DACL without touching its owner or SACL:

```c
/* Build an SD carrying only a DACL. */
peios_sd_builder *b = peios_sd_builder_new();
peios_sd_builder_dacl(b, new_acl, new_acl_len);
size_t sd_len; const void *sd_bytes = peios_sd_builder_bytes(b, &sd_len);

peios_file_set_sd(AT_FDCWD, "state.db", KACS_SECINFO_DACL, sd_bytes, sd_len, 0);
peios_sd_builder_free(b);
```

Because only `KACS_SECINFO_DACL` is selected, the owner, group, and SACL are left exactly as they were. The [`security.h` builders](~peios/sdk-security/building-security-descriptors) are how you assemble the SD to apply.

## Raw file-security interface

The SDK helpers above wrap `kacs_get_sd` and `kacs_set_sd`. The following low-level details belong to programs implementing descriptor tools; shell users should use [Managing file security](~peios/file-permissions/managing-file-security). The generated [KACS ABI](~peios/advanced-peios/peios-kernel/kacs/kacs-abi) is the source for syscall numbers and `KACS_SECINFO_*` constants; the `*_SECURITY_INFORMATION` names below are their specification spellings.

Raw access to `security.peios.sd` or `system.ntfs_security` is denied for reads, writes and removal. In particular, a raw read would expose the SACL without the component-specific `ACCESS_SYSTEM_SECURITY` gate. The syscall pair unifies access rules, validates descriptors and abstracts filesystem storage; see [File Descriptor Storage](~peios/advanced-peios/peios-kernel/kacs/facs/descriptor-storage).

## kacs_get_sd

The read syscall:

```
size = kacs_get_sd(dirfd, path, security_info, buf, buf_len, flags)
```

Returns the SD bytes for the requested components.

| Parameter | Meaning |
|---|---|
| `dirfd`, `path` | The file to read. Standard dirfd-relative resolution. |
| `security_info` | A bitmask saying which components to return. See below. |
| `buf`, `buf_len` | Output buffer. |
| `flags` | AT_EMPTY_PATH (use dirfd as fd-relative), AT_SYMLINK_NOFOLLOW (operate on the terminal symlink itself). |

The kernel:

1. Resolves the path.
2. Checks the access rights corresponding to the requested components (covered below). Paths and `O_PATH` handles use a live AccessCheck; ordinary file descriptors use their cached granted mask.
3. Reads the file's SD from its filesystem-native storage.
4. Constructs a **subset SD** containing only the requested components. Other components have offset 0 in the header and the corresponding PRESENT bit is clear.
5. Returns the subset SD in `buf`, with the total length as the return value.

### Probe mode

Calling with `buf_len = 0` and `buf = NULL` is a **probe** — the kernel computes the size the SD would take and returns it without writing to the buffer. The probe returns the same size value that a non-probe call would; the caller can use this size to allocate exactly the right buffer.

The probe call always returns the size on success. It does not return `-ERANGE` — the probe is itself a question about size, and answering it is the kernel's job. `-ERANGE` would be the wrong signal for a deliberate probe.

A non-probe call with `buf_len < required` fails with `-ERANGE` without truncating the result. Probe again, allocate and retry; the size can change between calls. Do not assume an out-parameter receives the required size: this signature has none. The [SDK two-call protocol](~peios/sdk-conventions/the-two-call-buffer-protocol) describes the wrapper convention.

### Access rules

Different components need different rights:

| Requested component (via `security_info` flag) | Required right |
|---|---|
| `OWNER_SECURITY_INFORMATION` | `READ_CONTROL` |
| `GROUP_SECURITY_INFORMATION` | `READ_CONTROL` |
| `DACL_SECURITY_INFORMATION` | `READ_CONTROL` |
| `SACL_SECURITY_INFORMATION` | `ACCESS_SYSTEM_SECURITY` |
| `LABEL_SECURITY_INFORMATION` | `READ_CONTROL` (the integrity label is in the SACL but its read is gated by READ_CONTROL, not ACCESS_SYSTEM_SECURITY) |

(The numeric flag values are catalogued in [Other constants](~peios/advanced-peios/constants-and-catalogs/other-constants).)

`READ_CONTROL` is the standard "read SD" right. Its implicit grant to the owner is subject to [OWNER RIGHTS and the other access layers](~peios/security-descriptors/ownership). `ACCESS_SYSTEM_SECURITY` is gated by `SeSecurityPrivilege`; an administrator label alone is not sufficient.

A caller asking for components they do not have rights for gets `-EACCES`. The component check is all-or-nothing: if any requested component fails its access check, the whole call fails rather than returning the permitted subset.

For combining requested components: just OR the flags. `kacs_get_sd(..., OWNER | DACL, ...)` returns owner and DACL but not SACL or group.

### SACL and LABEL are mutually exclusive

`SACL_SECURITY_INFORMATION` and `LABEL_SECURITY_INFORMATION` cannot be combined in one call. Setting both flags returns `-EINVAL`. The reasoning: `LABEL_SECURITY_INFORMATION` is a focused query for just the integrity label (which lives in the SACL); it has different access requirements than reading the full SACL. The kernel keeps the two paths separate.

## kacs_set_sd

The write syscall:

```
result = kacs_set_sd(dirfd, path, security_info, sd_buf, sd_len, flags)
```

| Parameter | Meaning |
|---|---|
| `dirfd`, `path` | Target file. |
| `security_info` | Which components to update. |
| `sd_buf`, `sd_len` | The new SD bytes (self-relative format). |
| `flags` | AT_EMPTY_PATH, AT_SYMLINK_NOFOLLOW (operate on the terminal symlink itself). |

The kernel:

1. Resolves the path.
2. Parses the SD blob. Rejects malformed SDs (size limit, bad ACL structure, etc.) with `-EINVAL`.
3. Checks the required rights per the components being updated, using a live AccessCheck for a path or `O_PATH` handle and the cached granted mask for an ordinary file descriptor.
4. Validates additional rules — owner SID is the caller's own or a SE_GROUP_OWNER group (unless SeRestorePrivilege), MANDATORY-flagged resource attributes are not removed (unless SeTcbPrivilege), integrity label is not raised above caller's own (unless SeRelabelPrivilege).
5. Writes the SD to the file's native storage.
6. Returns 0 on success.

Validation and component-right checks must succeed before the selected components are merged; unselected components are preserved. This is not a concurrent-update transaction. The [Kernel TRM storage rules](~peios/advanced-peios/peios-kernel/kacs/facs/descriptor-storage#caching) describe a last-writer-wins window: the xattr write and cache publication are not atomic with each other, although readers do not see a partially written descriptor. Coordinate writers rather than treating a read/edit/write sequence as compare-and-swap.

### Access rules for writes

| Component | Required right |
|---|---|
| Owner | `WRITE_OWNER` (plus the owner SID validation) |
| Group | `WRITE_OWNER` |
| DACL | `WRITE_DAC` |
| SACL | `ACCESS_SYSTEM_SECURITY` |
| LABEL (integrity label only) | `WRITE_OWNER` (plus integrity constraint) |

`WRITE_DAC` is the standard "modify DACL" right; the owner implicit grant is subject to OWNER RIGHTS suppression and the other access layers. `WRITE_OWNER` is needed to change the owner field (and the validation rules apply per [Ownership](~peios/security-descriptors/ownership)). `ACCESS_SYSTEM_SECURITY` is the SACL gate.

### The integrity label

`LABEL_SECURITY_INFORMATION` (0x10) is the focused write path for setting just the integrity label. The label lives in the SACL as a `SYSTEM_MANDATORY_LABEL_ACE`, but setting it via this flag is treated as a separate operation from setting the full SACL — with different access requirements:

- The right needed is `WRITE_OWNER`, not `ACCESS_SYSTEM_SECURITY`.
- The caller cannot raise the integrity label above the calling token's own integrity level (without `SeRelabelPrivilege`).
- Lowering the integrity label to at or below the caller's own integrity level is allowed.

`LABEL_SECURITY_INFORMATION` and `SACL_SECURITY_INFORMATION` cannot be combined in one call — same rule as for reading.

The use case: a process that wants to lower its files' integrity labels without holding `SeSecurityPrivilege`. The label is in the SACL conceptually, but setting it gets the `WRITE_OWNER` gate rather than the SACL gate, because adjusting the label down is a less sensitive operation than rewriting the audit policy.

### SD parsing and validation

The provided SD blob must be:

- In self-relative format (`SE_SELF_RELATIVE` flag set in control bits).
- Within the 65,535-byte size limit.
- Internally consistent — the offsets in the header point to valid locations, the ACLs parse cleanly, the SIDs are well-formed.

Any failure of validation returns `-EINVAL`. The original SD on the file is unchanged.

### Setting only some components

The `security_information` flags tell the kernel which components of the provided SD to apply. A blob containing owner + DACL with only `DACL_SECURITY_INFORMATION` set updates only the DACL; the file's existing owner is preserved.

The blob structure must still be valid — components not being applied are typically absent (offset 0, PRESENT bit clear) in the blob, but the blob's header still needs to be a valid SD header. After the merge, the resulting descriptor must retain a non-null owner; its group may be null.

This is the pattern for updating one part of an SD without touching the others. Read the SD (probe + fetch), update the relevant component, write back with only that component's flag set.

### Ownership transfer rules

Setting the owner via `kacs_set_sd` triggers the rules described in [Ownership](~peios/security-descriptors/ownership):

- The new owner must be the caller's own user_sid, **or** a SID in the caller's groups with `SE_GROUP_OWNER` set, **or** the caller must hold `SeRestorePrivilege`.
- The operation must pass the `WRITE_OWNER` gate. `SeTakeOwnershipPrivilege` can supply that right through a live access check, subject to mandatory policy; possession does not bypass a cached fd mask or the new-owner SID restriction. See [Ownership](~peios/security-descriptors/ownership) for availability and recovery limits.

A failed component-right check returns `-EACCES`; a failed owner-SID constraint without `SeRestorePrivilege` returns `-EPERM`.

The same rules apply whether you set ownership via `OWNER_SECURITY_INFORMATION` alone or in combination with other components.


## File-security errors

These are raw kernel error names; the SDK wrappers return `-1` and set `errno`.

| Error | Cause |
|---|---|
| `-EACCES` | Access check failed for the requested components. |
| `-EINVAL` | Malformed SD blob, invalid security_information combination, size limit exceeded, or other validation failure. |
| `-EPERM` | Owner SID validation failed without SeRestorePrivilege, or integrity label too high without SeRelabelPrivilege, or attempted MANDATORY attribute removal without SeTcbPrivilege. |
| `-ERANGE` | (`kacs_get_sd` only) Non-probe buffer too small; probe again and retry. |
| `-ENOENT` | The target path does not exist. |
| `-EROFS` | The selected path reaches the object through a read-only mount. |

The [Kernel TRM raw-call contract](~peios/advanced-peios/peios-kernel/kacs/facs/native-open#special-nodes-and-symlinks) says `AT_SYMLINK_NOFOLLOW` on `kacs_get_sd` and `kacs_set_sd` selects the symlink object, unlike native open, which fails with `ELOOP` in that case. The flag descriptions above follow that kernel contract.

> [!IMPORTANT]
> The [SDK query reference](~peios/sdk-files/reading-and-writing-a-file-s-security-descriptor#by-path) now records the pinned-source result: `AT_SYMLINK_NOFOLLOW` queries the terminal link's own SD, whereas native open rejects that link with `ELOOP`. libpeios forwards the query's flags. This resolves the former blanket no-follow query error; it does not establish that a link-SD write succeeds. The set-security description above retains its separate rights and storage requirements. No runtime test or historical release boundary was established.

For live-check versus cached-fd restore behavior, mandatory SACL attributes and full-SACL label constraints, see [The Set-Security Interface](~peios/advanced-peios/peios-kernel/kacs/facs/set-security). A full SACL write replaces the entire SACL. A label-only write preserves non-label ACEs; its input is either no SACL, to remove the explicit label, or a SACL containing exactly one non-inherit-only mandatory-label ACE.

## Pre-flighting an open

Sometimes you want to know whether a caller *could* open a file before you actually do. Read the file's SD, then run an [access check](~peios/sdk-access-control/checking-access) against the caller's token with `peios_file_generic_mapping` — no open, no side effects, just the verdict.

## Next

- **[`file.h` reference](~peios/sdk-files/file-h-file-security)** — every parameter, plus the fd-targeted calls and mount policy.
- **[File access](~peios/file-access/overview)** — the operator-side model of native file security.
