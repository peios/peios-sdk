---
title: file.h — File security
description: Opening files with an explicit rights mask, reading and writing their descriptors, and the mount policy that covers filesystems without native storage.
---

`<peios/file.h>` is the file surface of KACS. Where ordinary POSIX `open()` gives you a file descriptor governed by mode bits, `peios_file_open` performs a **native** KACS open — an `NtCreateFile`-shaped call carrying a desired access mask, a create disposition, create options, and an optional creator security descriptor — and hands back an ordinary Linux file fd whose **granted access mask is fixed for the fd's lifetime**. Because the grant is baked into the fd, it can be delegated safely by `dup`, `SCM_RIGHTS`, or across `exec`: whoever holds the fd holds exactly the access it was opened with, no more.

Alongside the open, this module reads and writes a file's security descriptor (by path or by fd) and governs how a superblock without native SD storage is treated.

The wire constants (`KACS_DISPOSITION_*`, `KACS_CREATE_OPT_*`, `KACS_FILE_*`, `KACS_SECINFO_*`, `KACS_MOUNT_POLICY_*`, `KACS_STATUS_*`) come from `<pkm/file.h>` and `<pkm/sd.h>`. The security descriptors these calls exchange are built and parsed with [`<peios/security.h>`](~peios/sdk-security/security-h-security-descriptors).

## See also

- **[`<peios/security.h>`](~peios/sdk-security/security-h-security-descriptors)** — building the creator SDs and parsing the SDs these calls return.
- **[`<peios/access.h>`](~peios/sdk-access/access-h-access-checks)** — evaluating a file SD with `peios_file_generic_mapping`.
- **[File access](~peios/file-access/overview)** and **[Mount policies](~peios/mount-policies/overview)** — the operator-side model of native file security.
