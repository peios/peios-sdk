---
title: Registering
description: rsi_register opens the registry device and registers every declared hive, returning the source fd the serve loop runs on.
---

```c
int rsi_register(const struct rsi_hive *hives, uint32_t count, uint64_t max_sequence);
```

Opens `/dev/pkm_registry` and registers all `count` hives in one call, returning the **source fd** — the descriptor you then `read(2)` requests and `write(2)` responses on — or `-1` with `errno`.

| Argument | Meaning |
|---|---|
| `hives` / `count` | The hives this source serves. `count` must be `>= 1`; the kernel enforces its configured `MaxHivesPerSource` limit. |
| `max_sequence` | The highest sequence number this source has **already persisted**. The kernel resumes its global sequence counter *past* this value, so a source that has durable state from a previous run must report it here to avoid reusing sequence numbers. A fresh source with no persisted state passes `0`. |

Errors include `EPERM` (no `SeTcbPrivilege`), `EINVAL`, `ENOSPC` (over the hive limit), `ENOMEM`, `EFAULT`, and any error from the underlying `/dev/pkm_registry` `open(2)`.

```c
struct rsi_hive hive = {
    .name = "MyStore", .name_len = 7,
    .flags = 0,                                  /* global hive */
    .root_guid = { /* … 16 bytes … */ },
};

int src = rsi_register(&hive, 1, /*max_sequence=*/0);
if (src < 0) { perror("rsi_register"); return -1; }
/* `src` is now the source fd — serve the RSI protocol on it. */
```

The `max_sequence` parameter is the one piece of state a durable source must get right: on restart, scan your persisted data for the highest sequence you ever wrote and pass it, so the kernel never hands out a sequence number you've already used.
