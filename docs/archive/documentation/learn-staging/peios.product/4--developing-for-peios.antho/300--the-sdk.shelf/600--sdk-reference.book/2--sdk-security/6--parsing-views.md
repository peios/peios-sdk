---
title: Parsing — views
description: Reading a descriptor, ACL or ACE without copying — the caller-allocated view structs and what each exposes.
---

To *read* a security descriptor, ACL, or ACE you use zero-copy **views**. A view is a caller-allocated, opaque, stack-friendly struct that borrows the buffer you parse — see the [view rules](~peios/sdk-conventions/memory-ownership). Every accessor that yields a SID, a nested ACL, or a blob returns a pointer *into the original buffer*, so that buffer must outlive the view and everything derived from it.

```c
typedef struct peios_sd_view        { uint64_t _opaque[8]; } peios_sd_view;
typedef struct peios_acl_view       { uint64_t _opaque[4]; } peios_acl_view;
typedef struct peios_ace_view       { uint64_t _opaque[4]; } peios_ace_view;
typedef struct peios_sid_array_view { uint64_t _opaque[4]; } peios_sid_array_view;
```

The `_opaque` arrays are sized for stack allocation with headroom — declare a view as a local and never read its fields.

### Security-descriptor views

```c
int      peios_sd_parse(const void *sd, size_t len, peios_sd_view *out);
uint16_t peios_sd_view_control(const peios_sd_view *v);
int      peios_sd_view_owner(const peios_sd_view *v, const void **sid, size_t *len);
int      peios_sd_view_group(const peios_sd_view *v, const void **sid, size_t *len);
int      peios_sd_view_dacl(const peios_sd_view *v, peios_acl_view *out);
int      peios_sd_view_sacl(const peios_sd_view *v, peios_acl_view *out);
```

`peios_sd_parse` validates a self-relative SD and populates `out`, returning `0` or `-1` (`EINVAL`). `peios_sd_view_control` returns the raw control-bit word.

The four component accessors return `0` with their out-params set on success, or `-1` if the component is **absent**. For the DACL and SACL, `-1` also covers the NULL-DACL case — since an absent DACL and a NULL DACL are the same thing in KACS, a `-1` from `peios_sd_view_dacl` uniformly means "no DACL constrains this object."

### ACL and ACE views

You can also parse a bare ACL directly — a token's default DACL, for instance, arrives as an ACL, not wrapped in an SD:

```c
int      peios_acl_parse(const void *acl, size_t len, peios_acl_view *out);
unsigned peios_acl_view_count(const peios_acl_view *a);
int      peios_acl_view_ace(const peios_acl_view *a, unsigned i, peios_ace_view *out);
```

`peios_acl_view_count` gives the number of ACEs; `peios_acl_view_ace` populates `out` for ACE `i` (0-based, in stored order), returning `0` or `-1` (`ERANGE` for an out-of-range index). Iterate in the obvious way:

```c
unsigned n = peios_acl_view_count(&dacl);
for (unsigned i = 0; i < n; i++) {
    peios_ace_view ace;
    peios_acl_view_ace(&dacl, i, &ace);
    /* inspect ace … */
}
```

Each ACE is read through its own accessors:

```c
uint8_t  peios_ace_view_type(const peios_ace_view *e);
uint8_t  peios_ace_view_flags(const peios_ace_view *e);
uint32_t peios_ace_view_mask(const peios_ace_view *e);
int      peios_ace_view_sid(const peios_ace_view *e, const void **sid, size_t *len);
int      peios_ace_view_object_type(const peios_ace_view *e, const uint8_t **guid16);
int      peios_ace_view_inherited_object_type(const peios_ace_view *e,
                                              const uint8_t **guid16);
int      peios_ace_view_app_data(const peios_ace_view *e, const void **data,
                                 size_t *len);
```

| Accessor | Yields |
|---|---|
| `_type` / `_flags` / `_mask` | The ACE's `KACS_ACE_TYPE_*` type, `KACS_ACE_FLAG_*` flags, and 32-bit access mask. |
| `_sid` | The trustee SID (a pointer into the buffer). `0` / `-1`. |
| `_object_type` | The object GUID of an object ACE — `0` with `*guid16` set to the 16 bytes, or `-1` if not present / not an object ACE. |
| `_inherited_object_type` | The inherited-object GUID, same convention. |
| `_app_data` | Trailing application data of a callback or resource-attribute ACE — for a callback ACE, this is the conditional-expression bytecode you can render with [`peios_sddl_format_condition`](~peios/sdk-security/sddl-text-codec#conditional-expressions). |

### SID-and-attributes arrays

Several token classes — `GROUPS`, `RESTRICTED_SIDS`, `DEVICE_GROUPS`, `CAPABILITIES` — return a packed `[count][sid_len][sid][attrs]…` blob rather than an ACL. Parse those with the SID-array view:

```c
int      peios_sid_array_parse(const void *blob, size_t len, peios_sid_array_view *out);
unsigned peios_sid_array_count(const peios_sid_array_view *a);
int      peios_sid_array_get(const peios_sid_array_view *a, unsigned i,
                             const void **sid, size_t *len, uint32_t *attrs);
```

`peios_sid_array_get` yields the `i`-th entry's SID (a pointer into the blob), its length, and its 32-bit attribute word (the `KACS_SE_GROUP_*` flags — enabled, mandatory, deny-only, and so on).
