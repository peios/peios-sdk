---
title: The audit context
description: Naming the object a check is about, so the kernel's audit record says what was decided on — encoding the PGSS §6.7 map, validating one built by hand, and the Rust and Go equivalents.
---

A daemon that guards objects of its own (a service manager's services, an event store's namespaces) makes each access decision through `peios_access_check`. The kernel records that decision in a `kacs.audit.access.checked` event when the descriptor's SACL asks for one, but the kernel cannot tell what the descriptor belongs to. The **audit context** tells it. It is the request's `audit_context` field: one MessagePack map holding `kind` and, under a key equal to the kind, the fields that identify the object.

```text
{kind: "service", service: {name: "jellyfin"}}
```

The kernel copies it into the record as `object.kind` and `object.service.name`, and marks the record `fields.attestation.userspace`, because the kernel did not observe those values: they are the daemon's claim. It refuses any other shape with `EINVAL`. [PGSS §6.7](~peios/pgss/events/access-decisions) is the specification. A component that guards objects must pass one, and each field it names must be defined, as asserted, in that component's catalogue fragment.

## Encoding one

```c
enum peios_audit_value_type {
    PEIOS_AUDIT_STR  = 0,   /* bytes/len: a UTF-8 string, not NUL-terminated */
    PEIOS_AUDIT_UINT = 1,   /* scalar */
    PEIOS_AUDIT_INT  = 2,   /* scalar, read as a two's-complement int64_t */
    PEIOS_AUDIT_BOOL = 3,   /* scalar: 0 or 1 */
    PEIOS_AUDIT_BIN  = 4,   /* bytes/len: binary, e.g. a SID or GUID */
};

struct peios_audit_field {
    const char *key;         /* NUL-terminated; one kebab-case segment */
    uint32_t    value_type;  /* enum peios_audit_value_type */
    uint64_t    scalar;
    const void *bytes;
    size_t      len;
};

ssize_t peios_audit_context_encode(const char *kind,
                                   const struct peios_audit_field *fields,
                                   size_t count, void *buf, size_t cap);
```

`peios_audit_context_encode` writes the map for an object of kind `kind`, identified by `count` fields, using the [two-call protocol](~peios/sdk-conventions/the-two-call-buffer-protocol): pass `cap == 0` to learn the size. With no fields (`fields` may then be `NULL`), the context is `{kind: <kind>}`.

`kind` and every key must be a single segment of an event name: kebab-case, `[a-z][a-z0-9]*(-[a-z0-9]+)*`, so `service`, `log-namespace` or `unit-name`. A value is a scalar in one of the [event wire forms](~peios/pgss/events/values): a string, an unsigned or signed integer, a boolean, or binary. A SID goes in as `PEIOS_AUDIT_BIN` with its binary bytes, never as SDDL text. There is no nil: a field with no value is left out of the array.

```c
struct peios_audit_field fields[] = {
    { .key = "name", .value_type = PEIOS_AUDIT_STR,
      .bytes = "jellyfin", .len = strlen("jellyfin") },
};
uint8_t ctx[128];
ssize_t n = peios_audit_context_encode("service", fields, 1, ctx, sizeof ctx);
if (n < 0) { perror("audit context"); return -1; }

struct peios_access_request req = {
    .token_fd = caller_fd,
    .sd = sd_bytes, .sd_len = sd_len,
    .desired = SERVICE_START,
    .mapping = service_mapping,
    .audit_context = ctx, .audit_context_len = (size_t)n,
};
uint32_t granted = 0;
int rc = peios_access_check(&req, &granted, NULL);
```

The encoder fails with `EINVAL` when:

- the kind or a key is not a segment, or a key is `NULL`;
- a key appears twice;
- `value_type` is not one of the five, or a boolean's `scalar` is not 0 or 1;
- a string is not valid UTF-8, or `bytes` is `NULL` while `len` is not zero;
- the encoded map would be longer than `KACS_ACCESS_CHECK_MAX_AUDIT_CONTEXT_LEN` (4096 bytes).

The kernel itself allows a nil value, a nested value, or a key that appears twice inside the body. The encoder refuses all three. A field with no value is left out ([PGSS §6.5](~peios/pgss/events/values)), an identifying field is a scalar, and a repeated key would make `object.<kind>.<key>` ambiguous.

## Validating one built another way

```c
int peios_audit_context_validate(const void *buf, size_t len);
```

If you build the map yourself, for example with the [msgpack writer](~peios/sdk-msgpack/writer), `peios_audit_context_validate` checks it against the kernel's own rules. It returns `0` if the kernel would accept it, and `-1` with `EINVAL` otherwise, including for `NULL` or empty input. The rules are:

- the top level is a map holding a string `kind` that is one segment, and at most one other key;
- that key, if present, equals the kind, and its value is a non-empty map whose keys are segments;
- nothing follows the map;
- the bytes are well-formed MessagePack (UTF-8 strings, no reserved `0xc1` byte), nested no deeper than 31 levels. That is the event nesting limit less one, because the context lands one level below the record's root;
- the map is at most 4096 bytes.

`peios_access_check` and `peios_access_check_list` apply the same check to any non-`NULL` `audit_context` before they make the syscall. A malformed context therefore fails there with `EINVAL`, without reaching the kernel. A non-`NULL` pointer with a zero length is refused too, as the kernel refuses it.

## From Rust and Go

The `peios` crate builds the same map with an `AuditContext` and hands it to the `AccessCheck` builder. Field values convert from `&str`, the integer types, `bool`, `&[u8]` and `&SidRef`:

```rust
use peios::access::{AccessCheck, AuditContext};

let ctx = AuditContext::new("service", &[("name", "jellyfin".into())])?;
let decision = AccessCheck::new(&sd, desired, mapping)
    .token(caller.as_fd())
    .audit_context(&ctx)
    .check()?;
```

`AuditContext::new` fails with `EINVAL` for the same reasons as the C encoder. `AuditContext::from_bytes` adopts a map you encoded yourself, after validating it.

In Go, `libp-go`'s `sd` package encodes the map for `CheckRequest.AuditContext`:

```go
ctx, err := sd.EncodeAuditContext("service", sd.AuditString("name", "jellyfin"))
if err != nil {
    return err
}
dec, err := sd.Check(sd.CheckRequest{
    Token:         caller,
    SD:            sdBytes,
    DesiredAccess: serviceStart,
    Mapping:       serviceMapping,
    AuditContext:  ctx,
})
```

The field constructors are `AuditString`, `AuditUint`, `AuditInt`, `AuditBool`, `AuditBytes` and `AuditSID`. A refusal wraps `sd.ErrBadAuditContext`. `sd.ValidateAuditContext` checks a map built by hand, and `Check` and `CheckList` run that check on a non-empty `AuditContext` before the syscall.
