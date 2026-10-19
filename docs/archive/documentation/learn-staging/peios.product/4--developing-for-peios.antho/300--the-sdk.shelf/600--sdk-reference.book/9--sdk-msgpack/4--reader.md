---
title: Reader
description: A stack-allocatable cursor over a borrowed buffer — peeking at the next value, then reading scalars, strings and containers.
---

The reader is a **cursor over a borrowed buffer** — stack-allocatable, no heap, no free. It decodes one value at a time, advancing the cursor.

```c
struct peios_mp_reader { uint64_t _opaque[4]; };   /* opaque — do not inspect */

void   peios_mp_reader_init(struct peios_mp_reader *r, const void *buf, size_t len);
size_t peios_mp_reader_remaining(const struct peios_mp_reader *r);
```

Declare a `struct peios_mp_reader` locally and `peios_mp_reader_init` it over your buffer before use. `buf` may be `NULL` only when `len` is zero. Borrowed `str`/`bin`/`ext` pointers the reader hands back point **into the original buffer** and are valid for as long as it lives. `peios_mp_reader_remaining` reports the unconsumed byte count.

### Peeking

```c
enum peios_mp_type {
    PEIOS_MP_NIL, PEIOS_MP_BOOL, PEIOS_MP_INT, PEIOS_MP_FLOAT,
    PEIOS_MP_STR, PEIOS_MP_BIN, PEIOS_MP_ARRAY, PEIOS_MP_MAP, PEIOS_MP_EXT,
};

int peios_mp_peek(const struct peios_mp_reader *r);
```

`peios_mp_peek` returns the `peios_mp_type` of the next value **without consuming it**, or `-1` at end-of-input or on an invalid lead byte. Note that integers of every width and sign report as `PEIOS_MP_INT` — read them with `peios_mp_read_int` or `peios_mp_read_uint` as you prefer. Peek is how you drive a dispatch over a value whose type you don't know ahead of time.

### Reading scalars

```c
int peios_mp_read_nil(struct peios_mp_reader *r);
int peios_mp_read_bool(struct peios_mp_reader *r, bool *out);
int peios_mp_read_int(struct peios_mp_reader *r, int64_t *out);
int peios_mp_read_uint(struct peios_mp_reader *r, uint64_t *out);
int peios_mp_read_float(struct peios_mp_reader *r, double *out);
```

Each **consumes one value on success** (returns `0`) and leaves the cursor **untouched on a type mismatch or truncation** (`-1` with `errno == EINVAL`) — so a failed read is safe to follow with a different-typed read or a `peek`. The `out` pointer is optional: pass `NULL` to consume/type-check a value without receiving its payload.

### Reading strings, bytes, containers, extensions

```c
ssize_t peios_mp_read_str(struct peios_mp_reader *r, const char **out);
ssize_t peios_mp_read_bin(struct peios_mp_reader *r, const void **out);
ssize_t peios_mp_read_array(struct peios_mp_reader *r);
ssize_t peios_mp_read_map(struct peios_mp_reader *r);
ssize_t peios_mp_read_ext(struct peios_mp_reader *r, int8_t *type_out, const void **out);
int     peios_mp_skip(struct peios_mp_reader *r);
```

- `peios_mp_read_str` / `peios_mp_read_bin` **borrow** the bytes (a pointer into the reader's buffer via `out`) and return the length, or `-1`. Strings are **not** NUL-terminated — use the length — and `peios_mp_read_str` rejects invalid UTF-8.
- `peios_mp_read_array` returns the **element count**; `peios_mp_read_map` returns the **key/value pair count** (so read `2 * count` values). After the header you read that many values yourself.
- `peios_mp_read_ext` borrows an extension value's bytes, reporting its signed type id through `type_out` (both `type_out` and `out` are independently optional), and returns the data length.
- `peios_mp_skip` consumes **exactly one complete value**, descending into nested containers — the way to ignore a value (or a whole subtree) you don't care about. `0` / `-1`.

```c
struct peios_mp_reader r;
peios_mp_reader_init(&r, payload, payload_len);

ssize_t pairs = peios_mp_read_map(&r);          /* top-level map */
for (ssize_t i = 0; i < pairs; i++) {
    const char *key; ssize_t klen = peios_mp_read_str(&r, &key);
    /* dispatch on key… then read or skip the value */
    peios_mp_skip(&r);
}
```
