---
title: The check
description: Running the full AccessCheck pipeline from userspace, and how the granted mask and audit outputs come back.
---

```c
int peios_access_check(const struct peios_access_request *req,
                       uint32_t *granted, struct peios_access_audit *audit);
```

Runs the full AccessCheck pipeline. Returns:

- **`0`** if *every* right in `desired` is granted;
- **`-1` with `errno == EACCES`** if any desired right is denied;
- **`-1` with another errno** on a real error (e.g. `EBADF` for a bad `token_fd`, `EINVAL` for a malformed SD).

`granted`, if non-`NULL`, **always** receives the granted access mask — even on denial. This is the useful part: you can request a broad `desired` and read back exactly which subset was granted, rather than probing one right at a time. `audit`, if non-`NULL`, receives the [audit outputs](~peios/sdk-access/audit-outputs).

```c
struct peios_access_request req = {
    .token_fd = -1,                      /* my own effective token */
    .sd = sd_bytes, .sd_len = sd_len,
    .desired = KACS_ACCESS_READ | KACS_ACCESS_WRITE,
    .mapping = peios_file_generic_mapping,
};

uint32_t granted = 0;
int rc = peios_access_check(&req, &granted, NULL);
if (rc == 0) {
    /* both READ and WRITE granted */
} else if (errno == EACCES) {
    /* denied; `granted` shows what WAS allowed (maybe READ only) */
} else {
    /* error: perror("access_check") */
}
```

libpeios owns the versioned `struct kacs_access_check_args` under the hood — it sets `caller_size` and zeroes the reserved fields so the request stays forward-compatible across kernel versions. You only ever fill in the `peios_access_request` above.
