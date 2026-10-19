---
title: Describing a hive
description: The rsi_hive struct a source fills in per hive it backs, field by field.
---

A **hive** is a subtree of the registry with its own root key. A source declares one `struct rsi_hive` per hive it backs:

```c
struct rsi_hive {
    const void *name;         /* hive name (not NUL-terminated) */
    uint32_t    name_len;
    uint32_t    flags;        /* RSI_HIVE_PRIVATE, or 0 for a global hive */
    uint8_t     root_guid[16];/* root key GUID */
    uint8_t     scope_guid[16];/* private hives; zero for a global hive */
};
```

| Field | Meaning |
|---|---|
| `name` / `name_len` | The hive's name, length-counted (not NUL-terminated). |
| `flags` | `RSI_HIVE_PRIVATE` for a private (scoped) hive, or `0` for a global one. |
| `root_guid` | The GUID of the hive's root key — the anchor every path in the hive resolves from. |
| `scope_guid` | For a **private** hive, the scope GUID that bounds who can resolve it; **zero for a global hive**. |

A **global** hive is visible system-wide; a **private** hive is scoped by `scope_guid` and resolvable only by tokens holding that scope (see the token [LCS credentials](~peios/sdk-tokens/the-token-spec-builder#lcs-registry-credentials)). Set `RSI_HIVE_PRIVATE` and a non-zero `scope_guid` together for a private hive; leave both clear for a global one.
