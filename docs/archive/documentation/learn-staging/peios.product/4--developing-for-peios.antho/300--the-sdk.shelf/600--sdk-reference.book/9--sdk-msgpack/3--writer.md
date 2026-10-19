---
title: Writer
description: Building a MessagePack value — scalars, containers, extensions and raw bytes, and taking the finished bytes.
---

```c
typedef struct peios_mp_writer peios_mp_writer;

peios_mp_writer *peios_mp_writer_new(void);
void             peios_mp_writer_free(peios_mp_writer *w);
void             peios_mp_writer_reset(peios_mp_writer *w);
```

Create a writer, append values, take the bytes, free it (or `reset` to reuse). All the append calls return `void` — errors latch.

### Scalars

```c
void peios_mp_write_nil(peios_mp_writer *w);
void peios_mp_write_bool(peios_mp_writer *w, bool v);
void peios_mp_write_int(peios_mp_writer *w, int64_t v);
void peios_mp_write_uint(peios_mp_writer *w, uint64_t v);
void peios_mp_write_float(peios_mp_writer *w, double v);
void peios_mp_write_str(peios_mp_writer *w, const char *s, size_t len);  /* UTF-8 */
void peios_mp_write_bin(peios_mp_writer *w, const void *b, size_t len);
```

Use `peios_mp_write_int` for signed and `peios_mp_write_uint` for unsigned values; both are stored in the smallest form. `peios_mp_write_str` takes UTF-8 with an explicit length (no NUL needed); `peios_mp_write_bin` takes arbitrary bytes.

### Containers

```c
void peios_mp_write_array(peios_mp_writer *w, uint32_t count);
void peios_mp_write_map(peios_mp_writer *w, uint32_t count);
```

Write the header, then exactly the promised number of values. **A map of `count` needs `2 * count` values** — `count` key/value *pairs* — written key, value, key, value…. An under- or over-filled container is not caught at the `write` call; it surfaces at `peios_mp_writer_bytes`, when the whole structure is validated.

```c
/* {"user": "alice", "ok": true} */
peios_mp_write_map(w, 2);
peios_mp_write_str(w, "user", 4);  peios_mp_write_str(w, "alice", 5);
peios_mp_write_str(w, "ok", 2);    peios_mp_write_bool(w, true);
```

### Extensions and raw bytes

```c
void peios_mp_write_ext(peios_mp_writer *w, int8_t ext_type, const void *b, size_t len);
void peios_mp_write_raw(peios_mp_writer *w, const void *b, size_t len);
```

- `peios_mp_write_ext` writes a MessagePack extension value with a signed type id.
- `peios_mp_write_raw` appends **pre-encoded** MessagePack bytes verbatim — the escape hatch for splicing in a value you already have encoded. The result is still structurally validated as a whole at `peios_mp_writer_bytes`, so you can't smuggle malformed bytes through it.

### Taking the bytes

```c
ssize_t peios_mp_writer_bytes(peios_mp_writer *w, const void **out);
int     peios_mp_writer_error(const peios_mp_writer *w);
```

`peios_mp_writer_bytes` **confirms the buffer is exactly one well-formed top-level value**, then borrows it: it writes a pointer to the encoded bytes through `out` (valid until the next mutating call on `w`) and returns the length. Pass `out == NULL` to validate and get the length without borrowing. It returns `-1` with `errno` — `EINVAL` on a latched error or a malformed/under-filled structure, `ENOMEM` on a prior allocation failure. `peios_mp_writer_error` returns the latched errno directly, or `0`.

Because this call validates, a successful `peios_mp_writer_bytes` is your guarantee the bytes are emit-ready.
