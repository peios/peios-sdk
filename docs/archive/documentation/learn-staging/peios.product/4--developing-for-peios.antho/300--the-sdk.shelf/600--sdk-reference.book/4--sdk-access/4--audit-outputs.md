---
title: Audit outputs
description: The audit struct a check can fill in — continuous audit masks, staging mismatch, and what each field means.
---

```c
struct peios_access_audit {
    uint32_t continuous_audit;   /* OR of matching alarm masks */
    int      staging_mismatch;   /* 1 if the staged CAAP result differs */
};
```

When you pass a non-`NULL` `audit`, the check reports:

- `continuous_audit` — the OR of the masks of any `SYSTEM_ALARM` ACEs that matched. An enforcement point that keeps this mask on a handle emits a `kacs.audit.handle.used` event for each later operation whose required access overlaps it.
- `staging_mismatch` — `1` if evaluating the *staged* central access policy would have produced a different result than the active one. This is the signal you watch when rolling out a [central access policy](~peios/central-access-policies/overview) change: a non-zero value means the pending policy would decide this access differently. The kernel records the same difference as a `kacs.caap.staging.diverged` event.

These are the outputs the check hands back to you. The audit records it writes to the event stream — `kacs.audit.access.checked` and `kacs.audit.privilege.used` — go to KMES, not to this struct. They name the object you checked only if the request carried an [audit context](~peios/sdk-access/the-audit-context), the map `{kind: "<kind>", "<kind>": {…}}`; anything else in that field fails the check with `EINVAL`.
