---
title: The token-spec builder
description: Assembling the token wire format with typed setters instead of by hand — core and advanced fields, flags, claims and credentials.
---

Minting a token means assembling a 192-byte-header wire format with many optional sections. The builder is the ergonomic path — typed setters, no hand-packed offsets — and follows the standard [sticky-error builder rules](~peios/sdk-conventions/memory-ownership): the setters return `void`, the first error latches, you check `peios_token_builder_error` at the end, and you `_free` every builder.

```c
typedef struct peios_token_builder peios_token_builder;

peios_token_builder *peios_token_builder_new(void);
void                 peios_token_builder_free(peios_token_builder *b);
void                 peios_token_builder_reset(peios_token_builder *b);
```

### The index convention

Three fields — the owner, the primary group, and the restrict/deny indices — refer to SIDs *by index* into the token's own SID list rather than by value. The convention is fixed:

> **Index 0 is the user SID. Indices 1..N are the 1st..Nth group** you added with `peios_token_builder_add_group`, in order.

So to make the second group the primary group, you set `primary_group_index` to `2`. **Do not add the logon SID yourself** — the kernel injects it.

### Core fields

```c
void peios_token_builder_user(peios_token_builder *b, const void *sid, size_t len);
void peios_token_builder_add_group(peios_token_builder *b, const void *sid,
                                   size_t len, uint32_t attrs);
void peios_token_builder_privileges(peios_token_builder *b, uint64_t present,
                                    uint64_t enabled);
void peios_token_builder_type(peios_token_builder *b, uint8_t type, uint8_t imp_level);
void peios_token_builder_integrity(peios_token_builder *b, uint32_t rid);
void peios_token_builder_session(peios_token_builder *b, uint64_t session_id);
void peios_token_builder_owner_index(peios_token_builder *b, uint32_t index);
void peios_token_builder_primary_group_index(peios_token_builder *b, uint32_t index);
void peios_token_builder_default_dacl(peios_token_builder *b, const void *acl, size_t len);
```

| Setter | Sets |
|---|---|
| `_user` | The user SID (index 0). |
| `_add_group` | Appends a group SID with its `KACS_SE_GROUP_*` attribute word (enabled, mandatory, deny-only, …). Call once per group, in the order you want them indexed. |
| `_privileges` | The privilege bitmasks: `present` (which privileges the token holds) and `enabled` (which are on). Bits are `KACS_SE_*_PRIVILEGE`. |
| `_type` | The token `type` (`KACS_TOKEN_TYPE_*` — primary or impersonation) and the impersonation level `imp_level` (`KACS_IMLEVEL_*`). Choose the level deliberately: it caps every identity derived from the token, including a primary. CreateToken accepts all four levels; Identification permits peers to identify the process without acting as it, and Anonymous hides its identity from peers. The process still acts as its own user. Older kernels reject primary levels below Impersonation. DuplicateToken retains its separate Impersonation floor for primary results. |
| `_integrity` | The integrity level, as the RID of an `S-1-16-<rid>` label (see [`peios_integrity_level`](~peios/sdk-security/sids#integrity-levels)). |
| `_session` | The logon session id the token references. |
| `_owner_index` / `_primary_group_index` | Which SID (by [index](#the-index-convention)) is the default owner / primary group. |
| `_default_dacl` | The default DACL applied to new objects the token creates (ACL bytes, e.g. from a [`peios_acl_builder`](~peios/sdk-security/building-acls)). |

### Advanced fields

These cover the rest of the token-spec and can be left unset. They are marked `[adv]` in the header for a reason — most tokens need none of them.

```c
void peios_token_builder_mandatory_policy(peios_token_builder *b, uint32_t bits);
void peios_token_builder_projected_ids(peios_token_builder *b, uint32_t uid, uint32_t gid);
void peios_token_builder_expiration(peios_token_builder *b, uint64_t when);
void peios_token_builder_source(peios_token_builder *b, const char name[8],
                                uint64_t source_id);
void peios_token_builder_audit_policy(peios_token_builder *b, uint32_t bits);
void peios_token_builder_add_restricted_sid(peios_token_builder *b, const void *sid,
                                            size_t len, uint32_t attrs);
void peios_token_builder_add_device_group(peios_token_builder *b, const void *sid,
                                          size_t len, uint32_t attrs);
void peios_token_builder_confinement(peios_token_builder *b, const void *sid, size_t len);
void peios_token_builder_supp_gids(peios_token_builder *b, const uint32_t *gids,
                                   unsigned count);
```

| Setter | Sets |
|---|---|
| `_mandatory_policy` | The mandatory-integrity policy bits governing how the integrity label is enforced. |
| `_projected_ids` | The POSIX `uid`/`gid` this token projects into the Linux-compatibility layer. |
| `_expiration` | An absolute expiry time after which the token is no longer valid. |
| `_source` | The token's source: an 8-byte `name` and a `source_id`, recording who issued it (appears in audit). |
| `_audit_policy` | Per-token audit policy bits. |
| `_add_restricted_sid` | Appends a restricting SID (a write-restricted / restricted token intersects these against the normal SIDs). |
| `_add_device_group` | Appends a device group SID (the device/machine side of a claim-aware token). |
| `_confinement` | The confinement/AppContainer package SID that sandboxes the token. |
| `_supp_gids` | Replaces the projected supplementary GIDs (pass `NULL, 0` to clear). |

### Token flags

The four boolean token-spec flags are set together, so a designated initialiser reads clearly:

```c
struct peios_token_flags {
    bool write_restricted;
    bool user_deny_only;
    bool isolation_boundary;
    bool confinement_exempt;
};
void peios_token_builder_flags(peios_token_builder *b, const struct peios_token_flags *f);
```

- `write_restricted` — the token's restricting SIDs are checked only for write access.
- `user_deny_only` — the user SID is usable for deny ACEs but not to grant access.
- `isolation_boundary` — marks an isolation boundary for confinement.
- `confinement_exempt` — the token is exempt from confinement checks.

### Claims

A **claim** is a named, typed, multi-valued security attribute — the input to conditional (callback) ACEs. Claims come in user and device flavours; both share the same shape.

```c
struct peios_token_claim_value {
    uint64_t    scalar;   /* INT64 / UINT64 / BOOLEAN (0 or 1) */
    const void *bytes;    /* STRING (UTF-8) / SID / OCTET */
    size_t      len;
};

struct peios_token_claim {
    const char *name;         /* UTF-8; transcoded to UTF-16LE on the wire */
    uint16_t    value_type;   /* KACS_CLAIM_TYPE_* */
    uint32_t    flags;        /* KACS_CLAIM_ATTR_* */
    const struct peios_token_claim_value *values;
    unsigned    value_count;
};

void peios_token_builder_add_user_claim(peios_token_builder *b,
                                        const struct peios_token_claim *claim);
void peios_token_builder_add_device_claim(peios_token_builder *b,
                                          const struct peios_token_claim *claim);
```

The `value_type` selects which member of each value carries the data:

| `value_type` | Value member |
|---|---|
| `KACS_CLAIM_TYPE_INT64` / `_UINT64` / `_BOOLEAN` | `scalar` (a boolean is `0` or `1`). |
| `KACS_CLAIM_TYPE_STRING` | `bytes`/`len` — a UTF-8 string (transcoded to UTF-16LE on the wire). |
| `KACS_CLAIM_TYPE_SID` | `bytes`/`len` — a binary SID. |
| `KACS_CLAIM_TYPE_OCTET` | `bytes`/`len` — an opaque blob. |

Each claim you add is round-tripped through the kernel's own claim parser before acceptance, so a malformed claim latches `EINVAL` on the builder immediately — you find out at build time, not at token-create time.

### LCS registry credentials

The final optional section grants the token registry-layer powers: which layer scopes it may resolve and which private layers it owns.

```c
struct peios_token_lcs_credentials {
    const uint8_t (*scope_guids)[16];    /* array of 16-byte GUIDs, each non-nil & unique */
    unsigned    scope_count;             /* <= KACS_TOKEN_LCS_MAX_SCOPE_GUIDS */
    const char *const *private_layers;   /* UTF-8 names, 1..255 bytes, no '/' or '\\', unique */
    unsigned    private_layer_count;     /* <= KACS_TOKEN_LCS_MAX_PRIVATE_LAYERS */
};
void peios_token_builder_lcs_credentials(peios_token_builder *b,
                                         const struct peios_token_lcs_credentials *creds);
```

Setting it replaces any prior credentials; it is emitted as the last token-spec section. See [`<peios/registry.h>`](~peios/sdk-registry-api/registry-h-the-registry-lcs) for what layers and scopes mean.

### Finishing the builder

```c
ssize_t peios_token_builder_bytes(peios_token_builder *b, const void **out);
int     peios_token_builder_create(peios_token_builder *b);
int     peios_token_builder_error(const peios_token_builder *b);
```

- `peios_token_builder_bytes` returns the serialised length and, if `out` is non-`NULL`, writes a pointer into the builder (valid until the next reset/free) through it. Use this if you want the raw token-spec bytes.
- `peios_token_builder_create` does it in one step: serialise and mint, returning the new token fd. This is the usual call. It requires `SeCreateTokenPrivilege`.
- `peios_token_builder_error` returns the latched errno, or `0`.

Errors: `_bytes` and `_create` first surface any latched builder error — `EINVAL` (malformed field, SID, claim, or index) or `ENOMEM` (allocation failed). A clean `_create` then adds the `peios_token_create_raw` set: `EPERM` (privilege missing), `EINVAL` (spec failed kernel validation), `ENOMEM`.

```c
peios_token_builder *tb = peios_token_builder_new();
peios_token_builder_user(tb, user_sid, user_len);
peios_token_builder_add_group(tb, admins_sid, admins_len, KACS_SE_GROUP_ENABLED);
peios_token_builder_type(tb, KACS_TOKEN_TYPE_PRIMARY, 0);
peios_token_builder_integrity(tb, PEIOS_IL_MEDIUM);
peios_token_builder_session(tb, session_id);

int tok = peios_token_builder_create(tb);       /* -1 on failure */
if (tok < 0) { int e = peios_token_builder_error(tb); /* or errno */ }
peios_token_builder_free(tb);
```
