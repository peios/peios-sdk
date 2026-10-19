---
title: The generic mapping
description: The canonical generic-to-specific rights mapping for the file object class, exported for you to pass to an access check.
---

```c
extern const struct kacs_generic_mapping peios_file_generic_mapping;
```

The canonical generic→specific rights mapping for the **file** object class. Pass it to [`peios_access_map_generic`](~peios/sdk-security/access-masks), or as the `mapping` in a [`peios_access_request`](~peios/sdk-access/the-request) when checking access against a file's SD — for example to pre-flight whether a caller could open a file before you actually open it.
