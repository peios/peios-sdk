---
title: The five at a glance
description: A one-page summary of the payload-bearing responses, and the rule covering everything else.
---

| Success response | Op | Payload |
|---|---|---|
| `rsi_respond_lookup` | `LOOKUP` | path entries + referenced key metadata |
| `rsi_respond_enum_children` | `ENUM_CHILDREN` | children (name + path entries) + metadata |
| `rsi_respond_read_key` | `READ_KEY` | one key's non-layered metadata |
| `rsi_respond_query_values` | `QUERY_VALUES` | value entries + blanket tombstones |
| `rsi_respond_delete_layer` | `DELETE_LAYER` | orphaned key GUIDs |

Every other op — and every failure of these — is `rsi_respond_status`.
