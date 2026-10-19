---
title: Adjust and transform
description: Changing a token in place or deriving a new one — privileges and groups, duplicate and restrict, impersonation and linked tokens.
---

These change a token or derive a new one from it. Deriving calls return a new fd; in-place adjustments return `0` / `-1`.

### Privileges and groups

```c
int peios_token_adjust_privileges(int fd, const struct kacs_priv_entry *entries,
                                  unsigned count, uint64_t *prev_enabled);
int peios_token_reset_privileges(int fd);
int peios_token_adjust_groups(int fd, const struct kacs_group_entry *entries,
                              unsigned count, uint64_t *prev_state);
int peios_token_reset_groups(int fd);
```

- `peios_token_adjust_privileges` enables/disables the privileges named in `entries` (each a `kacs_priv_entry`); if `prev_enabled` is non-`NULL` it receives the prior enabled mask, so you can restore it later. `peios_token_reset_privileges` restores `enabled := enabled_by_default`. Errors: `EACCES` (handle lacks `ADJUST_PRIVILEGES`), `EINVAL` (empty or oversized batch, duplicate entry, enabling an absent privilege, unknown attribute bits), `EFAULT` (bad entries pointer).
- `peios_token_adjust_groups` is the group analogue. `prev_state`, if non-`NULL`, points at a caller array of `KACS_TOKEN_GROUP_MASK_WORDS` `uint64_t` words that receives the prior enabled bitmask. `peios_token_reset_groups` restores the default group state. Errors: `EACCES` (handle lacks `ADJUST_GROUPS`), `EINVAL` (mandatory, deny-only, or logon-SID group targeted; duplicate or out-of-range index; empty batch), `EFAULT` (bad entries pointer).

### Duplicate and restrict

```c
int peios_token_duplicate(int fd, uint32_t access, uint8_t type, uint8_t imp_level);

struct peios_token_restrict {
    uint64_t           privs_to_delete;
    const uint32_t    *deny_group_indices;   /* groups demoted to deny-only */
    unsigned           deny_count;
    const void *const *restrict_sids;        /* added restricting SIDs */
    const size_t      *restrict_sid_lens;
    unsigned           restrict_count;
    uint32_t           flags;                /* KACS_TOKEN_RESTRICT_WRITE_RESTRICTED */
};
int peios_token_restrict(int fd, const struct peios_token_restrict *spec);
```

- `peios_token_duplicate` copies the token, returning a new fd with handle rights `access`, token `type` (`KACS_TOKEN_TYPE_*`), and impersonation level `imp_level` (`KACS_IMLEVEL_*`). This is how you turn a primary token into an impersonation token, an impersonation token into a primary one for a process launched as that identity, or narrow a handle's rights. `imp_level` is a ratchet: it may equal or lower the source's level, never raise it, whichever direction the type goes. Errors: `EACCES` (handle lacks `DUPLICATE`, or the new token's SD denies `access`), `EINVAL` (unknown `type`/`imp_level`, a level above the source's, a primary requested below `KACS_IMLEVEL_IMPERSONATION`, empty or unknown `access` bits), `ENOMEM` (allocation failed).
- `peios_token_restrict` creates a **filtered** token — the sandboxing primitive. It can delete privileges (`privs_to_delete`), demote groups to deny-only (`deny_group_indices`, by [index](~peios/sdk-tokens/the-token-spec-builder#the-index-convention)), add restricting SIDs (`restrict_sids`/`restrict_sid_lens`), and set `KACS_TOKEN_RESTRICT_WRITE_RESTRICTED`. The result is a strictly less-powerful token you can hand to less-trusted code. Errors: `EACCES` (handle lacks `DUPLICATE`), `EINVAL` (duplicate or out-of-range deny index, malformed restricting SID, unknown `flags`, `NULL` spec or arrays), `ENOMEM` (allocation failed).

### Impersonation and installation

```c
int peios_token_install(int fd);
int peios_token_impersonate(int fd);
int peios_token_revert(void);
```

- `peios_token_install` makes this **primary** token the calling process's primary token. Errors: `EACCES` (handle lacks `ASSIGN_PRIMARY`, or `SeAssignPrimaryTokenPrivilege` missing), `EINVAL` (not a primary token), `EAGAIN` (thread set changed mid-install — retry), `ENOMEM` (allocation failed).
- `peios_token_impersonate` makes this **impersonation** token the calling thread's effective identity — subsequent access checks on that thread run as the impersonated identity. Errors: `EACCES` (handle lacks `IMPERSONATE`), `EINVAL` (not an impersonation token), `EPERM` (restricted→unrestricted same-user — the one hard deny), `ENOMEM` (allocation failed).
- `peios_token_revert` undoes it: it clears the thread's impersonation token so checks run as the thread's real (primary) identity again. It takes no argument and is a no-op (reported as success) if the thread was not impersonating. This is the inverse of `peios_token_impersonate` — always pair them, ideally with `revert` in the cleanup path. Errors: none in normal operation.

The archetypal server flow: `peios_token_open_peer` the caller → `peios_token_impersonate` it → do the work as them → `peios_token_revert`. `peios_token_impersonate_peer` (see [Opening and creating tokens](~peios/sdk-tokens/opening-and-creating-tokens)) fuses the first two steps and the close.

### Linked tokens and defaults

```c
int peios_token_link(int elevated_fd, int filtered_fd, uint64_t logon_session_id);
int peios_token_get_linked(int fd);
int peios_token_adjust_default(int fd, const void *dacl, size_t len,
                               uint16_t owner_index, uint16_t group_index);
int peios_token_set_interactivity_scope(int fd, uint32_t scope);
```

- `peios_token_link` links an elevated + filtered primary-token pair in `logon_session_id` — the UAC-style split-token model, where a filtered token is the everyday identity and its elevated linked token is available on demand. `peios_token_get_linked` opens the linked token of `fd`, returning a new fd. Errors (`_link`): `EACCES` (`SeTcbPrivilege` missing, or either handle lacks `DUPLICATE`), `EINVAL` (self-link, role/logon-session/user-SID mismatch, not primary tokens, unknown `logon_session_id`, or an fd that is not a token fd), `EBADF` (invalid fd). Errors (`_get_linked`): `EACCES` (handle lacks `QUERY`), `ENOENT` (not part of a linked pair, or the pair was destroyed), `ENOMEM` (allocation failed).
- `peios_token_adjust_default` replaces the token's default DACL and/or owner/primary-group indices. `dacl == NULL` leaves the DACL unchanged (and ignores `len`); `dacl != NULL` with `len == 0` clears it; an index of `0xFFFF` leaves that index unchanged. Errors: `EACCES` (handle lacks `ADJUST_DEFAULT`), `EINVAL` (out-of-range index; malformed or oversized DACL), `EFAULT` (bad DACL pointer).
- `peios_token_set_interactivity_scope` changes the token's `uint32_t` interactive-environment scope (requires `SeTcbPrivilege`). This is deliberately separate from the token's `uint64_t` LogonSession LUID (`auth_id`) and does not move the token between logon sessions. Errors: `EACCES` (handle lacks `ADJUST_INTERACTIVITY_SCOPE`, or `SeTcbPrivilege` missing).
