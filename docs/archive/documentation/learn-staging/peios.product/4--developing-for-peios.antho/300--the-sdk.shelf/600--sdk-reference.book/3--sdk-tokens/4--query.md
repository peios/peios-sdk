---
title: Query
description: Reading a token's contents by information class — the generic reader, and the typed wrappers over the common classes.
---

You read a token's contents by **information class**. The generic reader handles any class getxattr-style; typed convenience wrappers cover the common ones.

```c
ssize_t peios_token_query(int fd, uint32_t info_class, void *buf, size_t cap);
ssize_t peios_token_user(int fd, void *sid_buf, size_t cap);   /* CLASS_USER */
```

- `peios_token_query` reads the class `info_class` (`KACS_TOKEN_CLASS_*`) into `buf` using the [two-call protocol](~peios/sdk-conventions/the-two-call-buffer-protocol). Classes that return SID arrays or ACLs are parsed afterward with the [`<peios/security.h>` views](~peios/sdk-security/parsing-views) — e.g. read `CLASS_GROUPS` into a buffer, then `peios_sid_array_parse` it.
- `peios_token_user` is the same two-call read specialised to the user SID (`CLASS_USER`): probe with `sid_buf == NULL, cap == 0`, then retrieve.

For the common scalar classes there are typed helpers that write through a mandatory non-`NULL` out-pointer and return `0` / `-1`:

```c
struct peios_privilege_set {
    uint64_t present;
    uint64_t enabled;
    uint64_t enabled_by_default;
    uint64_t used;
};

struct peios_token_statistics {
    uint64_t token_id;
    uint64_t auth_id;       /* LogonSession LUID */
    uint64_t modified_id;
    uint32_t token_type;
    uint32_t reserved;
    uint64_t expiration;
};

int peios_token_type(int fd, uint32_t *out);            /* CLASS_TYPE */
int peios_token_impersonation_level(int fd, uint32_t *out); /* CLASS_IMPERSONATION_LEVEL */
int peios_token_interactivity_scope(int fd, uint32_t *out); /* CLASS_INTERACTIVITY_SCOPE */
int peios_token_guid(int fd, uint8_t guid_out[16]);     /* CLASS_GUID */
int peios_token_statistics(int fd, struct peios_token_statistics *out); /* CLASS_STATISTICS */
int peios_token_integrity(int fd, uint32_t *level_rid_out); /* CLASS_INTEGRITY_LEVEL */
int peios_token_privileges(int fd, struct peios_privilege_set *out); /* CLASS_PRIVILEGES */
```

`peios_token_impersonation_level` reads the token's impersonation level (`KACS_IMLEVEL_*`): the ceiling on everything captured from, conveyed by, or duplicated out of it — on a *primary* token as much as an impersonation one, since the level only ever ratchets down (Kernel TRM §3.5.1). A manager handed a token over a socket reads this before deciding what the token may become; a token below Impersonation cannot be turned into a primary token for a new process.

`peios_token_interactivity_scope` reads the token's interactive-environment scope. It is not a logon-session identifier. To read the token's `uint64_t` LogonSession LUID, use `peios_token_statistics` and inspect `auth_id`; that helper returns the complete fixed-width statistics record.

`peios_token_guid` writes the immutable 16-byte token GUID used by KMES. The caller supplies 16 writable bytes; the helper leaves them unchanged on failure. Rust exposes `Token::guid() -> Result<[u8; 16]>` and Go exposes `(*Token).GUID() ([16]byte, error)`. All return raw bytes in KMES order. They require `TOKEN_QUERY`; kernels predating `KACS_TOKEN_CLASS_GUID` return `EINVAL`. Deploy the supporting kernel before consumers. The statistics record and its LUID remain unchanged.

`peios_token_privileges` returns all four privilege words at once: which privileges are `present`, which are `enabled`, which are `enabled_by_default`, and which have been `used` (the audit trail of privilege use).

Errors (all query calls): `EACCES` (handle lacks `QUERY`), `EINVAL` (unknown class), `ERANGE` (non-probe buffer too small), `EFAULT` (bad buffer pointer). The typed helpers add `EINVAL` (`NULL` out-pointer, or an unexpected payload shape).
