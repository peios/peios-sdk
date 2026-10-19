---
title: Subkeys, metadata, and watches
description: Enumerating subkeys, reading key metadata, deleting and hiding keys, and arming a watch for changes.
---

### Enumerating subkeys

```c
struct peios_reg_subkey {
    void    *name;             /* in:  buffer for the child's name (NULL to probe) */
    uint64_t last_write_time;  /* out: ns since the Unix epoch */
    uint32_t name_cap;         /* in */  uint32_t name_len;    /* out */
    uint32_t subkey_count;     /* out: the child's subkey count */
    uint32_t value_count;      /* out: the child's value count */
};
int peios_reg_enum_subkey(int key_fd, uint32_t index, int txn_fd,
                          struct peios_reg_subkey *v);
```

`peios_reg_enum_subkey` reads the child key at `index`, dense over visible children — walk from `0` until `ENOENT`. There is **no per-child access check** during enumeration (you see the names and counts; opening a child still checks its SD).

### Key metadata

```c
struct peios_reg_key_info {
    void    *name;                 /* in:  buffer for the key's leaf name (NULL to probe) */
    uint64_t last_write_time;      /* out */
    uint64_t hive_generation;      /* out: per-hive change epoch */
    uint32_t name_cap;  uint32_t name_len;
    uint32_t subkey_count;         /* out */
    uint32_t value_count;          /* out */
    uint32_t max_subkey_name_len;  /* out */
    uint32_t max_value_name_len;   /* out */
    uint32_t max_value_data_size;  /* out */
    uint32_t sd_size;              /* out: security-descriptor size */
    uint8_t  volatile_key;         /* out: 1 if volatile */
    uint8_t  symlink;              /* out: 1 if a symlink */
};
int peios_reg_query_key_info(int key_fd, struct peios_reg_key_info *v);
```

`peios_reg_query_key_info` reads the key's leaf name and its metadata (needs `READ_CONTROL`). Note the ordering wrinkle: the kernel reports the metadata **only once the name fits**, so a too-small (or zero-capacity) name buffer returns `ERANGE` with the required `name_len` and *no* metadata — size the name buffer from that, then call again to get everything. The `max_*` fields are sizing hints for enumerations; `hive_generation` is a per-hive change epoch you can watch to detect that *anything* under the hive changed.

### Deleting and hiding keys

```c
int peios_reg_delete_key(int key_fd, const void *layer, uint32_t layer_len, int txn_fd);
int peios_reg_hide_key(int key_fd, const void *layer, uint32_t layer_len, int txn_fd);
```

Both need `DELETE` access, take a layer (`NULL`/`0` = base) and an optional `txn_fd`, and cannot target a hive root (`EINVAL`).

- **`peios_reg_delete_key`** removes this key's path entry *in a layer*; lower-layer entries re-emerge. It fails with `ENOTEMPTY` if the key has visible children.
- **`peios_reg_hide_key`** creates a `HIDDEN` path entry that masks the key in a layer; removing that layer makes the key reappear. This is the key-level analogue of a tombstone — hide rather than destroy.

### Watching for changes

```c
int peios_reg_notify(int key_fd, uint32_t filter, int subtree);
int peios_reg_flush(int key_fd);
```

- **`peios_reg_notify`** arms change watches on `key_fd` (needs `KEY_NOTIFY`). `filter` is a mask of `REG_NOTIFY_VALUE` / `REG_NOTIFY_SUBKEY` / `REG_NOTIFY_SD` (or `REG_NOTIFY_ALL`); `subtree` (`0`/`1`) extends the watch to descendants. `filter == 0` disarms. Once armed, **the key fd itself becomes pollable** — `EPOLLIN` signals pending events, and `read()` on the fd returns the change records. So a watch integrates directly into an `epoll` loop with no side channel. Errors: `ENOENT` (orphaned key), `EINVAL`, `EACCES`.
- **`peios_reg_flush`** forces the source to persist this key's hive's pending writes (needs `KEY_SET_VALUE`) and returns once persistence is confirmed — the durability barrier.

**The change records.** A `read()` on an armed key fd returns as many **complete** records as fit in your buffer — records are never split across reads. If the buffer is too small for even the next record the read fails `EINVAL` (so size it generously — a few KiB), and a non-blocking fd with nothing pending fails `EAGAIN`. Each record is a little-endian, possibly unaligned byte stream (Peios Kernel TRM §5.6, Watches, with the header offsets in §5.A):

| Offset | Size | Field | Meaning |
|---|---|---|---|
| 0 | 4 | `total_len` | Record size in bytes — advance by this to the next record (future versions may append fields). |
| 4 | 2 | `event_type` | `REG_WATCH_VALUE_SET` / `_VALUE_DELETED` / `_SUBKEY_CREATED` / `_SUBKEY_DELETED` / `_SD_CHANGED` / `_KEY_DELETED` / `_OVERFLOW`. |
| 6 | 2 | `name_len` | Byte length of `name`; `0` for the no-name events (`SD_CHANGED`, `KEY_DELETED`, `OVERFLOW`). |
| 8 | `name_len` | `name` | The changed value or subkey name (UTF-8, not NUL-terminated). |

A **subtree** watch appends two further fields after `name`: `path_depth` (`u16`) and that many length-prefixed path components (`u16` length + UTF-8 bytes), locating the changed key relative to the watched key — depth `0` means the watched key itself.

Delivery is best-effort with an overflow fallback: if records accumulate faster than you read them, the oldest are dropped and a `REG_WATCH_OVERFLOW` record is queued — on seeing one, re-read the watched key (and subtree) to recover current state rather than trusting the stream. Records describe **effective** (layer-resolved) changes, and uncommitted transactions produce none — events fire at commit.
