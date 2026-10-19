---
title: Payload-bearing responses
description: The five operations that return data on success, and the flat arrays each helper takes to encode the frame for you.
---

Five operations return data on success. Each takes the result as flat arrays and encodes the frame for you.

### LOOKUP

```c
struct rsi_path_entry {
    const void *layer;  uint32_t layer_len;
    uint8_t     target_type;   /* RSI_PATH_TARGET_GUID (0) / RSI_PATH_TARGET_HIDDEN (1) */
    uint8_t     target_guid[16];
    uint64_t    sequence;
};
struct rsi_key_metadata {
    uint8_t     guid[16];
    const void *sd;  uint32_t sd_len;
    uint8_t     volatile_key;
    uint8_t     symlink;
    uint64_t    last_write_time;
};

int rsi_respond_lookup(int fd, const struct rsi_request *req,
                       const struct rsi_path_entry *entries, uint32_t entry_count,
                       const struct rsi_key_metadata *metadata, uint32_t metadata_count);
```

Answers a `LOOKUP` with the resolved path `entries` for the child — one per layer that has a view of it, each either a **GUID** target or a **HIDDEN** (tombstone) target — plus the `metadata` for every key the entries reference. A `RSI_PATH_TARGET_HIDDEN` entry must carry an all-zero `target_guid`; the metadata must exactly cover the GUID targets. `EINVAL` if `req` is not a `LOOKUP`, a nonzero count has a NULL array, or an entry has invalid target/boolean fields, missing/duplicate metadata, or unreferenced metadata.

### ENUM_CHILDREN

```c
struct rsi_child_entry {
    const void            *child_name;  uint32_t child_name_len;
    const struct rsi_path_entry *entries;  uint32_t entry_count;
};

int rsi_respond_enum_children(int fd, const struct rsi_request *req,
                              const struct rsi_child_entry *children, uint32_t child_count,
                              const struct rsi_key_metadata *metadata, uint32_t metadata_count);
```

Answers an `ENUM_CHILDREN` with each `child` — its name and the [path entries](#lookup) that resolve it — plus the `metadata` for every referenced key. The same target/boolean/metadata-coverage rules as `LOOKUP` apply. `EINVAL` on the same conditions, scoped to `ENUM_CHILDREN`.

### READ_KEY

```c
int rsi_respond_read_key(int fd, const struct rsi_request *req, const void *name,
                         uint32_t name_len, const uint8_t *parent_guid, const void *sd,
                         uint32_t sd_len, uint8_t volatile_key, uint8_t symlink,
                         uint64_t last_write_time);
```

Answers a `READ_KEY` with the key's non-layered metadata: its `name`, `parent_guid`, security descriptor (`sd`), the `volatile_key`/`symlink` flags, and `last_write_time`. `EINVAL` if `req` is not a `READ_KEY`, `parent_guid` is NULL, or a boolean field is invalid.

### QUERY_VALUES

```c
struct rsi_value_entry {
    const void *value_name;  uint32_t value_name_len;
    const void *layer_name;  uint32_t layer_name_len;
    uint32_t    value_type;
    const void *data;  uint32_t data_len;
    uint64_t    sequence;
};
struct rsi_blanket_entry {
    const void *layer_name;  uint32_t layer_name_len;
    uint64_t    sequence;
};

int rsi_respond_query_values(int fd, const struct rsi_request *req,
                             const struct rsi_value_entry *entries, uint32_t entry_count,
                             const struct rsi_blanket_entry *blankets, uint32_t blanket_count);
```

Answers a `QUERY_VALUES` with the value `entries` — each value's name, the layer it lives in, its type, data, and sequence — plus the `blankets` (the blanket tombstones on this key, each a layer and sequence). The kernel resolves precedence across the layers you report. `EINVAL` if `req` is not a `QUERY_VALUES` or a nonzero count has a NULL array.

### DELETE_LAYER

```c
int rsi_respond_delete_layer(int fd, const struct rsi_request *req,
                             const uint8_t *orphaned_guids, uint32_t orphaned_count);
```

Answers a `DELETE_LAYER` with the GUIDs of the keys the deleted layer orphaned — a flat `orphaned_count * 16`-byte array. The GUIDs must be nonzero and unique. `EINVAL` if `req` is not a `DELETE_LAYER` or a nonzero count has a NULL array, a nil GUID, or a duplicate.
