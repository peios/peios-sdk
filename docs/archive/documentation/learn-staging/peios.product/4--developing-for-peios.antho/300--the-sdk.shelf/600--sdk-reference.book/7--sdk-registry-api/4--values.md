---
title: Values
description: Reading, writing, deleting and tombstoning a value, and enumerating a key's values — with names, types and layers.
---

A value is **named** (length-counted; an empty name is the key's *default* value), **typed** (`REG_*`), and written **into a layer**. A base-layer target is `layer == NULL` with `layer_len == 0`; a non-`NULL` pointer with a zero length is rejected `EINVAL`.

### Reading a value

```c
struct peios_reg_value {
    uint64_t sequence;  /* out: effective entry's sequence number */
    void    *data;      /* in:  buffer for the value data (NULL to probe) */
    void    *layer;     /* in:  buffer for the layer name (NULL to probe/skip) */
    uint32_t type;      /* out: value type (REG_*) */
    uint32_t data_cap;  /* in */   uint32_t data_len;  /* out: actual/required */
    uint32_t layer_cap; /* in */   uint32_t layer_len; /* out: actual/required */
};

int peios_reg_query_value(int key_fd, const void *name, uint32_t name_len, int txn_fd,
                          struct peios_reg_value *v);
```

`peios_reg_query_value` reads the **effective** value `name` on `key_fd` — the winner of the layer precedence resolution. `name_len == 0` reads the default value; `txn_fd` reads within a transaction, or `-1` for none. It fills `v->data` with the value bytes and `v->layer` with the name of the layer that won, and reports the resolved `type` and `sequence`. Pass a `NULL` `layer` buffer if you don't care which layer won. Errors: `ENOENT` (no effective value, or a tombstone masks it), `ERANGE`, `EACCES`, `EINVAL`.

### Writing, deleting, tombstoning

```c
int peios_reg_set_value(int key_fd, const void *name, uint32_t name_len, uint32_t type,
                        const void *data, uint32_t data_len, const void *layer,
                        uint32_t layer_len, int txn_fd, uint64_t expected_seq);
int peios_reg_delete_value(int key_fd, const void *name, uint32_t name_len,
                           const void *layer, uint32_t layer_len, int txn_fd);
int peios_reg_blanket_tombstone(int key_fd, const void *layer, uint32_t layer_len,
                                int set, int txn_fd);
```

- **`peios_reg_set_value`** writes value `name` of `type` into a specific `layer` (`NULL`/`0` = base). `type` may be `REG_TOMBSTONE` to place a *per-value* tombstone that masks lower layers. `expected_seq` is a **compare-and-swap guard**: `0` disables it; otherwise the write applies only if the value's current sequence matches, else `EAGAIN`. This is how you do lost-update-safe read-modify-write — read the `sequence` from `peios_reg_query_value`, then set with `expected_seq` set to it. Errors: `EINVAL`, `EAGAIN`, `ENOSPC`, `ENAMETOOLONG`, `EPERM`, `EACCES`.
- **`peios_reg_delete_value`** removes *a layer's* entry for `name` (`NULL`/`0` = base). It is idempotent, and removing a layer's entry lets any lower-layer value **re-emerge** — deletion is per-layer, not global.
- **`peios_reg_blanket_tombstone`** sets (`set != 0`) or clears (`set == 0`) a *blanket* tombstone on a layer, masking **all** lower-precedence values of this key on that layer at once — the wholesale version of a per-value tombstone. `set` must be `0` or `1` (else `EINVAL`).

### Enumerating values

```c
int peios_reg_query_values_batch(int key_fd, int txn_fd, void *buf, uint32_t cap,
                                 uint32_t *len_out, uint32_t *count_out);

struct peios_reg_enum_value {
    void    *name;      /* in:  buffer for the value name (NULL to probe) */
    void    *data;      /* in:  buffer for the value data (NULL to probe) */
    uint32_t type;      /* out */
    uint32_t name_cap;  /* in */  uint32_t name_len;  /* out: actual/required */
    uint32_t data_cap;  /* in */  uint32_t data_len;  /* out: actual/required */
};
int peios_reg_enum_value(int key_fd, uint32_t index, int txn_fd,
                         struct peios_reg_enum_value *v);
```

Two ways to read every effective value of a key:

- **`peios_reg_query_values_batch`** reads them all into one `buf` in a single call — the efficient path. Each record is packed little-endian, back to back: `[name_len: u32][name][type: u32][data_len: u32][data]`, for `count` records. `len_out` receives the bytes written (or the required size on `ERANGE`); `count_out` receives the record count. Both may be `NULL`.
- **`peios_reg_enum_value`** reads one value at a time by `index`, dense over the key's tombstone-resolved values — walk from `0` until `ENOENT`. Use it when you want to process values incrementally rather than buffer them all.
