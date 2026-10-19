---
title: SIDs
description: Constructing, formatting and inspecting SIDs, the well-known set, and the integrity-level helpers.
---

A **SID** (Security Identifier) is the unique binary name of a principal. For the full account of what a SID *is* — its string and binary forms, the mixed endianness, the equality rule — see the operator-side page on [SIDs](~peios/identity/sids). This section is the API for handling them.

A SID is small and bounded. The largest possible encoding is `PEIOS_SID_MAX_BYTES` (68) bytes, so a buffer of that size holds any valid SID and the SID builders below never need a two-call probe — you can always pass a `PEIOS_SID_MAX_BYTES` stack buffer and skip straight to the retrieve call.

```c
#define PEIOS_SID_MAX_BYTES 68u
```

### Constructing SIDs

Each of these encodes a SID into your buffer and returns its length (or `-1` with `errno`). Because a SID fits in `PEIOS_SID_MAX_BYTES`, the probe is optional — but these are still `ssize_t`/two-call functions, so passing `cap == 0` to probe works too.

| Function | Builds |
|---|---|
| `peios_sid_build(out, cap, id_authority, sub_auths, count)` | An arbitrary SID from its parts: a 48-bit identifier authority (numeric, encoded big-endian) and `count` sub-authorities (encoded little-endian). `count` is `0..KACS_SID_MAX_SUB_AUTHORITIES`. |
| `peios_sid_parse_string(out, cap, sddl)` | A binary SID from its SDDL string form (`"S-1-5-21-…"`). |
| `peios_sid_integrity(out, cap, level_rid)` | An integrity-label SID `S-1-16-<rid>` (see [`peios_integrity_level`](#integrity-levels)). |
| `peios_sid_logon(out, cap, session_id)` | A logon SID `S-1-5-5-<hi>-<lo>` from a 64-bit session id. |
| `peios_sid_well_known(out, cap, which)` | A well-known SID selected by [`enum peios_wks`](#well-known-sids). |

```c
ssize_t peios_sid_build(void *out, size_t cap, uint64_t id_authority,
                        const uint32_t *sub_auths, unsigned count);
ssize_t peios_sid_parse_string(void *out, size_t cap, const char *sddl);
ssize_t peios_sid_integrity(void *out, size_t cap, uint32_t level_rid);
ssize_t peios_sid_logon(void *out, size_t cap, uint64_t session_id);
ssize_t peios_sid_well_known(void *out, size_t cap, enum peios_wks which);
```

`peios_sid_build` fails with `EINVAL` if `count` exceeds the maximum, and (like all of these) with `ERANGE` if a non-zero `cap` is too small.

### Formatting and inspecting SIDs

| Function | Returns |
|---|---|
| `peios_sid_format(sid, len, out, cap)` | The SDDL string form (`"S-1-…"`), as a string length excluding the `NUL` — allocate `len + 1`. |
| `peios_sid_valid(sid, len)` | `true` if `sid` is a structurally valid SID of *exactly* `len` bytes. |
| `peios_sid_length(sid)` | The encoded length of `sid`, read from its sub-authority count. **You must have already validated `sid`, or bounded it to `PEIOS_SID_MAX_BYTES`** — this trusts the buffer. |
| `peios_sid_equal(a, alen, b, blen)` | `true` for exact binary equality — the *only* equality KACS defines for SIDs. |
| `peios_sid_rid(sid, len)` | The RID (last sub-authority), or `0` if the SID has none. |

```c
ssize_t  peios_sid_format(const void *sid, size_t len, char *out, size_t cap);
bool     peios_sid_valid(const void *sid, size_t len);
size_t   peios_sid_length(const void *sid);
bool     peios_sid_equal(const void *a, size_t alen, const void *b, size_t blen);
uint32_t peios_sid_rid(const void *sid, size_t len);
```

The split between `peios_sid_valid` and `peios_sid_length` is deliberate: validation is the safe check that bounds an untrusted buffer; `peios_sid_length` is the fast reader you use *after* you trust the bytes (or when you have already capped the buffer at `PEIOS_SID_MAX_BYTES`). When in doubt, validate first.

### Well-known SIDs

`peios_sid_well_known` constructs any of the standard system principals without you memorising their numbers:

```c
enum peios_wks {
    PEIOS_WKS_NULL,                 /* S-1-0-0    Nobody */
    PEIOS_WKS_EVERYONE,             /* S-1-1-0    World */
    PEIOS_WKS_LOCAL,                /* S-1-2-0    Local */
    PEIOS_WKS_CREATOR_OWNER,        /* S-1-3-0 */
    PEIOS_WKS_CREATOR_GROUP,        /* S-1-3-1 */
    PEIOS_WKS_OWNER_RIGHTS,         /* S-1-3-4    suppresses owner WRITE_DAC */
    PEIOS_WKS_ANONYMOUS,            /* S-1-5-7 */
    PEIOS_WKS_SELF,                 /* S-1-5-10   PRINCIPAL_SELF */
    PEIOS_WKS_AUTHENTICATED_USERS,  /* S-1-5-11 */
    PEIOS_WKS_SYSTEM,               /* S-1-5-18   Local System */
    PEIOS_WKS_LOCAL_SERVICE,        /* S-1-5-19 */
    PEIOS_WKS_NETWORK_SERVICE,      /* S-1-5-20 */
    PEIOS_WKS_ADMINISTRATORS,       /* S-1-5-32-544 */
};
```

For the meaning of each principal, see [Well-known principals](~peios/identity/well-known-principals).

### Integrity levels

Integrity-label SIDs have the form `S-1-16-<rid>`, where the RID names a level. `peios_sid_integrity` takes that RID; the standard levels are:

```c
enum peios_integrity_level {
    PEIOS_IL_UNTRUSTED = 0,
    PEIOS_IL_LOW       = 4096,
    PEIOS_IL_MEDIUM    = 8192,
    PEIOS_IL_HIGH      = 12288,
    PEIOS_IL_SYSTEM    = 16384,
};
```

These are the labels that appear in a SACL as a `SYSTEM_MANDATORY_LABEL` ACE (see [`peios_acl_builder_label`](~peios/sdk-security/building-acls#adding-aces)).
