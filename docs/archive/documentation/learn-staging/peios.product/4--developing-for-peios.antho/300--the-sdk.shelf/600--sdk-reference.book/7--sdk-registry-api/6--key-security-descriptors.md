---
title: Key security descriptors
description: Getting and setting a registry key's security descriptor, with the same secinfo mask the file API uses.
---

```c
int peios_reg_get_security(int key_fd, uint32_t security_info, void *sd, uint32_t cap,
                           uint32_t *sd_len_out);
int peios_reg_set_security(int key_fd, uint32_t security_info, const void *sd,
                           uint32_t sd_len, int txn_fd);
```

Keys are KACS-secured, so their SDs are read and written with the same [`<peios/security.h>`](~peios/sdk-security/security-h-security-descriptors) vocabulary as files and tokens; `security_info` selects components (owner/group/DACL/SACL).

- **`peios_reg_get_security`** reads the selected components into `sd` (KACS binary form), writing the length to `*sd_len_out` (may be `NULL`); a too-small buffer returns `ERANGE` with the required size there, and a zero `cap` probes. Owner/group/DACL need `READ_CONTROL`; the SACL needs `ACCESS_SYSTEM_SECURITY`.
- **`peios_reg_set_security`** applies the selected components of `sd`, merging with the rest (the kernel parses and validates). The DACL needs `WRITE_DAC`, the owner `WRITE_OWNER`, the SACL `ACCESS_SYSTEM_SECURITY`. Here `txn_fd` gives **atomicity, not layer qualification** (SDs are not layered), or `-1` to apply immediately. SD changes affect only **future** opens — handles already open keep their fixed grant.
