---
title: The object-type-list variant
description: Checking a whole object type list in one call, with a per-node result for each entry.
---

```c
int peios_access_check_list(const struct peios_access_request *req,
                            struct kacs_node_result *results, uint32_t count);
```

`peios_access_check_list` is the `AccessCheckByTypeResultList` form — a *per-node* check over an object-type tree, for objects whose properties or property sets carry their own object ACEs (a directory-service-style object, say). It evaluates the whole tree in one call and reports a separate result for each node.

- `req->object_tree` / `object_tree_count` are **mandatory** here — they describe the tree of `kacs_object_type_entry` nodes to evaluate.
- `results` receives **one `kacs_node_result` per node, in preorder**, and `count` **must equal** `req->object_tree_count`.
- Returns `0` / `-1` (`EINVAL` if `count` doesn't match, and the usual errors otherwise).

Each `kacs_node_result` carries that node's granted mask and status, so you can discover, for example, that a caller may read most of an object but not one protected property — in a single check rather than one per property.
