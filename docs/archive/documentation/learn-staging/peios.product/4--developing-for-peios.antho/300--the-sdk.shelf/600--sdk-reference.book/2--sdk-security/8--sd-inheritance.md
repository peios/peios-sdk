---
title: SD inheritance
description: Computing a child object's ACEs from its parent's inheritable ones, in userspace, with both helper entry points.
---

Inheritance — computing a child object's ACEs from its parent's inheritable ones — is also pure userspace (MS-DTYP §2.5.3.4). Both helpers take and produce self-relative SDs and use the two-call protocol.

```c
ssize_t peios_sd_reinherit(void *out, size_t cap, const void *parent_sd,
                           size_t parent_len, const void *child_sd,
                           size_t child_len, int is_container);
ssize_t peios_sd_strip_inherited(void *out, size_t cap, const void *sd,
                                 size_t sd_len, uint32_t info);
```

**`peios_sd_reinherit`** recomputes a child SD's inherited ACEs from its parent. It strips the ACEs carrying `ACE_FLAG_INHERITED` from the child DACL, re-derives them from the parent DACL, and appends them *after* the child's explicit ACEs; the child's owner, group, SACL, and control bits pass through unchanged. `is_container` is non-zero if the child is itself a container (which determines how container-inherit and object-inherit flags propagate). This is what you call when a parent's ACL changed and you need to push the new inheritance down to a child.

**`peios_sd_strip_inherited`** drops the `ACE_FLAG_INHERITED` ACEs from the ACLs selected by `info` — a mask of `*_SECURITY_INFORMATION` bits, of which `DACL_SECURITY_INFORMATION` and `SACL_SECURITY_INFORMATION` are honoured and the rest ignored (selecting neither copies the input verbatim). Owner, group, and control bits pass through. Use it to reduce a descriptor to just its *explicit* ACEs — for example before storing a "protected" descriptor that should not carry inherited entries.

Both return the new SD's byte length, or `-1` with `EINVAL` (malformed input) or `ERANGE` (a non-zero buffer too small).
