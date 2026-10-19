---
title: The response contract
description: The rules every response helper enforces, and what violating one costs you.
---

All `rsi_respond_*` helpers return `0`, or `-1` with `errno`. A set of rules applies to **every** helper, and violating one is an `EINVAL` caller-contract error:

- **`(ptr, len)` pairs:** a pointer may be `NULL` only when its length/count is zero.
- **Boolean fields** (`volatile_key`, `symlink`, target types) must be exactly `0` or `1`.
- **Hidden path targets** (`RSI_PATH_TARGET_HIDDEN`) must carry an **all-zero** `target_guid`.
- **`LOOKUP`/`ENUM_CHILDREN` metadata** must exactly cover the GUID path targets referenced — no missing metadata, no duplicates, no unreferenced entries.
- **`DELETE_LAYER` orphan GUIDs** must be nonzero and unique.

Beyond `EINVAL`, any helper can also fail with `ENOMEM` (during validation or frame allocation), `EOVERFLOW` (validation arithmetic or the assembled frame too large), `EIO` (a short `write`), or the raw `write(2)` errno. Per-helper `EINVAL` additions are noted below.
