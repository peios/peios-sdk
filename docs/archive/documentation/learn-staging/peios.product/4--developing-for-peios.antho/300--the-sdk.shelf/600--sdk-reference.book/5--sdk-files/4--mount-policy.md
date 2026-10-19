---
title: Mount policy
description: How KACS treats a superblock that cannot store native descriptors, and the entry points for reading and setting that policy.
---

Not every filesystem can store native security descriptors. The **mount policy** governs how KACS treats a superblock that has no native SD storage — whether files there get a synthesised SD, a template SD, or are denied. These calls target the superblock the object `fd` lives on. At the pinned kernel revision below, both require enabled `SeManageVolumePrivilege` or `SeTcbPrivilege`, with successful privilege-use marking.

> [!IMPORTANT]
> The gate is verified in [kernel `8e0e22de3a59cad506bbbf8873de456e16ad272d`](https://github.com/peios/pkm/blob/8e0e22de3a59cad506bbbf8873de456e16ad272d/kacs/capability.c#L613-L637), for both [reads](https://github.com/peios/pkm/blob/8e0e22de3a59cad506bbbf8873de456e16ad272d/kacs/mount_policy.c#L479-L496) and [writes](https://github.com/peios/pkm/blob/8e0e22de3a59cad506bbbf8873de456e16ad272d/kacs/mount_policy.c#L369-L380). The [libpeios 0.5.8 wrappers](https://github.com/peios/libpeios/blob/de0018bdcaed14abb796aeccec9c8387fd30e452/src/file.rs#L451-L575) at `de0018bdcaed14abb796aeccec9c8387fd30e452` pass through to those syscalls; their TCB-only comments do not enforce an additional privilege gate. No runtime test or release boundary is established here. See [operator policy-access checks](~peios/mount-policies/managing-mounts#check-policy-access) before changing privileges.

```c
struct peios_mount_policy {
    uint32_t    policy;      /* KACS_MOUNT_POLICY_* */
    uint32_t    flags;
    uint32_t    generation;
    const void *template_sd;
    size_t      template_sd_len;
};

int peios_mount_get_policy(int fd, struct peios_mount_policy *out,
                           void *tmpl_buf, size_t tmpl_cap);
int peios_mount_set_policy(int fd, const struct peios_mount_policy *p);
```

- `peios_mount_get_policy` reads the policy for `fd`'s superblock into `out`. The template SD is returned into your `tmpl_buf` getxattr-style: on success `out->template_sd` points **into `tmpl_buf`** when that buffer was large enough, or is `NULL` if the superblock has no template. A `NULL` template buffer (or `tmpl_cap == 0`) is valid only when you don't need the template bytes. A too-small template buffer is **not** an error — the call still succeeds, reports the true length in `out->template_sd_len`, and leaves `out->template_sd` `NULL` so you can size a retry. Errors: `EPERM` (neither accepted privilege enabled, or privilege-use marking failed), `EBADF` (bad fd), `EINVAL` (`NULL` `out`, or `NULL` `tmpl_buf` with non-zero `tmpl_cap`), `EFAULT` (bad buffer pointer), `ENOMEM` (allocation failed).
- `peios_mount_set_policy` installs `p` as the superblock's policy. `policy` is a `KACS_MOUNT_POLICY_*` value; `template_sd`/`template_sd_len` supply the template SD when the policy calls for one. `flags` and `generation` must be zero on set — the kernel manages the generation counter itself and rejects a non-zero input. Errors: `EPERM` (neither accepted privilege enabled, or privilege-use marking failed), `EINVAL` (unknown or unmanaged `policy`, non-zero `flags`/`generation`, malformed or oversized template, `NULL` template with non-zero length), `EOPNOTSUPP` (superblock not KACS-managed), `EBADF` (bad fd), `EFAULT` (bad pointer).
