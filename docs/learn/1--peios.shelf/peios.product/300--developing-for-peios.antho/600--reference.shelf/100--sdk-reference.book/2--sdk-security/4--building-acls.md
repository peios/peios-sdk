---
title: Building ACLs
description: Assembling an ordered list of ACEs with an ACL builder — adding each ACE type, and taking the serialized bytes.
---

An **ACL** is an ordered list of ACEs. You assemble one with a
`peios_acl_builder` — create it, add ACEs, take the serialized bytes, free it.
Builders follow the [sticky-error
rules](~peios/sdk-conventions/memory-ownership): the adders return `void`, the
first error latches, and you check `peios_acl_builder_error` at the end.

```c
typedef struct peios_acl_builder peios_acl_builder;

peios_acl_builder *peios_acl_builder_new(void);   /* NULL on OOM */
void               peios_acl_builder_free(peios_acl_builder *b);
void               peios_acl_builder_reset(peios_acl_builder *b);
```

`peios_acl_builder_reset` drops every accumulated ACE *and* clears the sticky
error, so you can reuse one builder for several ACLs.

## Adding ACEs

The common single-SID families have convenience adders. `flags` is a mask of
`KACS_ACE_FLAG_*` and is usually `0` — the flags carry inheritance semantics,
which matter only for container/inheritable ACEs.

```c
void peios_acl_builder_allow(peios_acl_builder *b, const void *sid, size_t len,
                             uint32_t mask, uint8_t flags);
void peios_acl_builder_deny (peios_acl_builder *b, const void *sid, size_t len,
                             uint32_t mask, uint8_t flags);
void peios_acl_builder_audit(peios_acl_builder *b, const void *sid, size_t len,
                             uint32_t mask, uint8_t flags);
```

| Adder | Appends |
|---|---|
| `_allow` | An `ACCESS_ALLOWED` ACE — grants `mask` to `sid`. |
| `_deny` | An `ACCESS_DENIED` ACE — denies `mask` to `sid`. Order matters: put denies before allows. |
| `_audit` | A `SYSTEM_AUDIT` ACE — logs access by `sid` matching `mask`. Belongs in a SACL, not a DACL. |

For an integrity label there is a dedicated adder:

```c
void peios_acl_builder_label(peios_acl_builder *b, uint32_t integrity_rid,
                             uint32_t policy_mask);
```

It appends a `SYSTEM_MANDATORY_LABEL` ACE for integrity level
`S-1-16-<integrity_rid>`. `policy_mask` is a mask of the
`KACS_SYSTEM_MANDATORY_LABEL_NO_{READ,WRITE,EXECUTE}_UP` bits (from
`<pkm/sd.h>`) that says which accesses a lower-integrity caller is denied.
Like `_audit`, a label ACE belongs in a SACL.

For everything else — object ACEs, callback ACEs, resource-attribute ACEs —
there is the general adder and a fully-specified ACE struct:

```c
struct peios_ace_spec {
    uint8_t       type;      /* KACS_ACE_TYPE_* */
    uint8_t       flags;     /* KACS_ACE_FLAG_* */
    uint32_t      mask;
    const void   *sid;       /* trustee */
    size_t        sid_len;
    const uint8_t *object_type;            /* 16-byte GUID, or NULL */
    const uint8_t *inherited_object_type;  /* 16-byte GUID, or NULL */
    const void   *app_data;  /* trailing callback/resource data */
    size_t        app_data_len;
};

void peios_acl_builder_add(peios_acl_builder *b, const struct peios_ace_spec *ace);
```

Fill in only the fields the `type` uses; leave the rest `NULL`/`0`:

- **Object ACEs** (`KACS_ACE_TYPE_*_OBJECT`) read `object_type` and
  `inherited_object_type` — each a 16-byte GUID, or `NULL` when absent.
- **Callback and resource-attribute ACEs** carry trailing `app_data` (which is
  `NULL` only when `app_data_len` is `0`). For callback ACEs this is the
  conditional-expression bytecode you can produce with
[`peios_sddl_parse_condition`](~peios/sdk-security/sddl-text-codec#conditional-expressions).

The convenience adders are exactly `peios_acl_builder_add` with a pre-filled
spec for the common cases; reach for `_add` when you need object, callback, or
resource-attribute ACEs.

The SDDL encoder includes zero padding after a variable-length payload so the
total `AceSize` is a multiple of four, as required by PCDS. This includes
resource claims ending in an odd-length octet or string payload. The padding
is included in `AceSize`; it does not change lengths or offsets within the
claim itself. The raw C ACL builder requires the supplied ACE body to already
be four-byte aligned; it rejects an unaligned body with `EINVAL`.

## Taking the ACL bytes

```c
const void *peios_acl_builder_bytes(peios_acl_builder *b, size_t *len_out);
ssize_t     peios_acl_builder_finish(peios_acl_builder *b, void *buf, size_t cap);
int         peios_acl_builder_error(const peios_acl_builder *b);
```

- `peios_acl_builder_bytes` borrows: it returns a pointer into the builder
  (valid until the next mutation, `_reset`, or `_free`), writing the length to
  `len_out` if non-`NULL`. It returns `NULL` if the sticky error is set.
- `peios_acl_builder_finish` copies the serialized ACL out using the two-call
  protocol.
- `peios_acl_builder_error` returns the latched errno, or `0` if the builder
  is healthy.

The usual next step is to hand these bytes to `peios_sd_builder_dacl` or
`_sacl`.
