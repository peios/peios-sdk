---
title: Building security descriptors
description: Binding an owner, group, DACL, SACL and control flags into one self-relative buffer with the descriptor builder.
---

A **security descriptor** binds an owner, a group, a DACL, a SACL, and control flags into one self-relative buffer. Its builder mirrors the ACL builder's shape.

```c
typedef struct peios_sd_builder peios_sd_builder;

peios_sd_builder *peios_sd_builder_new(void);
void              peios_sd_builder_free(peios_sd_builder *b);
void              peios_sd_builder_reset(peios_sd_builder *b);
```

### Setting components

```c
void peios_sd_builder_owner(peios_sd_builder *b, const void *sid, size_t len);
void peios_sd_builder_group(peios_sd_builder *b, const void *sid, size_t len);
void peios_sd_builder_control(peios_sd_builder *b, uint16_t set, uint16_t clear);
void peios_sd_builder_dacl(peios_sd_builder *b, const void *acl, size_t len);
void peios_sd_builder_dacl_null(peios_sd_builder *b);
void peios_sd_builder_sacl(peios_sd_builder *b, const void *acl, size_t len);
```

- **Owner / group.** Omit the call to leave the component absent. That is exactly what you want when building a *partial* SD to set only some components via `kacs_set_sd` — the SD then carries only what you set.
- **Control bits.** `peios_sd_builder_control` sets the bits in `set` and clears those in `clear` (`KACS_SD_DACL_PROTECTED`, and friends). You do **not** manage `SELF_RELATIVE` or the `*_PRESENT` bits — the builder maintains those for you as you add components.
- **DACL / SACL.** Pass ACL bytes, typically straight from `peios_acl_builder_bytes`. An ACL with zero ACEs is a *present-but-empty* DACL, which grants only the owner's implicit rights.

The DACL has one subtlety worth stating plainly. KACS has **no NULL-DACL encoding** — there is no "DACL present, pointer null" form; the kernel's parser rejects it. So "grant everyone everything" is expressed as an **absent** DACL (the `DACL_PRESENT` control bit clear). `peios_sd_builder_dacl_null` requests exactly that: it clears any DACL you set earlier and produces the same bytes as never setting a DACL at all. It exists so you can state the grant-all intent explicitly rather than by omission — but be clear that it means *grant all*, not *deny all*.

### Taking the SD bytes

Identical in shape to the ACL builder:

```c
const void *peios_sd_builder_bytes(peios_sd_builder *b, size_t *len_out);
ssize_t     peios_sd_builder_finish(peios_sd_builder *b, void *buf, size_t cap);
int         peios_sd_builder_error(const peios_sd_builder *b);
```

`_bytes` borrows (valid until the next mutation/reset/free, `NULL` if errored), `_finish` copies out getxattr-style, `_error` returns the latched errno.
