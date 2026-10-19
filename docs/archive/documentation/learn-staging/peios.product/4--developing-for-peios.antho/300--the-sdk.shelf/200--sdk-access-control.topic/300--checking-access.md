---
title: Checking access
type: how-to
description: Make an authorisation decision end to end — build a security descriptor, run an access check against a token, and interpret the granted mask.
related:
  - peios/sdk-access/access-h-access-checks
  - peios/sdk-security/security-h-security-descriptors
  - peios/access-decisions/overview
---

This guide walks a complete access decision: you have an object to protect, a caller to check, and you want KACS to tell you what they're allowed. The exhaustive detail lives in [`access.h`](~peios/sdk-access/access-h-access-checks) and [`security.h`](~peios/sdk-security/security-h-security-descriptors); here we put them together.

## When to use this

Reach for [`peios_access_check`](~peios/sdk-access/the-check) when **your** program is the resource manager — you own something that isn't a kernel object (a document, an API route, a record) and you want to gate it with Peios identities and rules. For actual kernel objects, don't pre-check; just perform the operation and let the kernel enforce.

Remember the check is **advisory**: it computes the answer, you enforce it.

## Step 1 — describe what protects the object

An object is protected by a security descriptor. If you don't already have one, build it. Say the resource should be readable and writable by its owner and read-only for a "viewers" group:

```c
/* An ACL: allow OWNER full, allow VIEWERS read. */
peios_acl_builder *acl = peios_acl_builder_new();
peios_acl_builder_allow(acl, owner_sid, owner_len, KACS_ACCESS_ALL, 0);
peios_acl_builder_allow(acl, viewers_sid, viewers_len, KACS_ACCESS_READ, 0);

size_t acl_len;
const void *acl_bytes = peios_acl_builder_bytes(acl, &acl_len);

/* Wrap it in a security descriptor with an owner. */
peios_sd_builder *sd = peios_sd_builder_new();
peios_sd_builder_owner(sd, owner_sid, owner_len);
peios_sd_builder_dacl(sd, acl_bytes, acl_len);

size_t sd_len;
const void *sd_bytes = peios_sd_builder_bytes(sd, &sd_len);
```

(You can also write the descriptor as [SDDL text](~peios/sdk-security/sddl-text-codec) and parse it — often easier when the policy is fixed.)

## Step 2 — identify the subject

The subject is a token. Most often it's the caller you [opened from a socket](~peios/sdk-access-control/working-with-tokens#who-is-calling-me), or your own effective token. In the request you pass either a token fd or `-1` for your own effective token.

## Step 3 — run the check

Fill in a request and call. `desired` is what you're testing; `mapping` folds any generic rights to the object class's specific bits (use the class's published mapping — here we'll treat the object like a file):

```c
struct peios_access_request req = {
    .token_fd = caller_fd,               /* or -1 for my own token */
    .sd       = sd_bytes, .sd_len = sd_len,
    .desired  = KACS_ACCESS_READ | KACS_ACCESS_WRITE,
    .mapping  = peios_file_generic_mapping,
};

uint32_t granted = 0;
int rc = peios_access_check(&req, &granted, NULL);
```

## Step 4 — interpret the result

```c
if (rc == 0) {
    /* Every desired right was granted. */
} else if (errno == EACCES) {
    /* Denied. `granted` still holds what WAS allowed — e.g. maybe READ
       succeeded but WRITE did not. Decide per-right from `granted`. */
    bool may_read  = granted & KACS_ACCESS_READ;
    bool may_write = granted & KACS_ACCESS_WRITE;
} else {
    /* A real error: bad token fd, malformed SD, etc. */
    perror("access_check");
}
```

The key idea: a denial is **not** an error to log and bail on — it's the expected "no". And because `granted` is filled even on denial, you can ask for a broad set of rights in one call and read back exactly which subset the subject has, rather than probing right by right. Clean up the builders (`peios_acl_builder_free`, `peios_sd_builder_free`) and the token fd when done.

## Per-property checks

If your object has properties or property-sets with their own object ACEs, evaluate the whole tree in one call with [`peios_access_check_list`](~peios/sdk-access/the-object-type-list-variant): supply an object-type tree and get one result per node, so you learn (for instance) that a caller may read most of an object but not one protected field — without a separate check per field.

## Auditing a decision

Pass a non-`NULL` [`peios_access_audit`](~peios/sdk-access/audit-outputs) to learn what a `SYSTEM_AUDIT` ACE match would log, and whether a *staged* central access policy would decide differently — the signal you watch when rolling out a policy change.

If your daemon guards objects of its own, name the object in the request's [audit context](~peios/sdk-access/the-audit-context), so the kernel's audit record of the decision says what was checked:

```c
struct peios_audit_field name = {
    .key = "name", .value_type = PEIOS_AUDIT_STR,
    .bytes = "jellyfin", .len = strlen("jellyfin"),
};
uint8_t ctx[128];
ssize_t n = peios_audit_context_encode("service", &name, 1, ctx, sizeof ctx);
req.audit_context = ctx;
req.audit_context_len = (size_t)n;   /* check n >= 0 first */
```

Do not write an event of your own recording the grant or the refusal. The kernel's record is the audit record, and a SACL on the object decides whether it is written ([PGSS §6.7](~peios/pgss/events/access-decisions)).

## Next

- **[`access.h` reference](~peios/sdk-access/access-h-access-checks)** — every field of the request and the audit outputs.
- **[Access decisions](~peios/access-decisions/overview)** — the operator-side account of how KACS reaches a verdict (and how to debug a surprising denial).
