---
title: Logon sessions
description: The lightweight kernel bookkeeping a token references, the entry points for creating and destroying one, and listing the live ones.
---

A **logon session** is the lightweight kernel bookkeeping a token references — the "login" a token belongs to. Creating and destroying them requires `SeTcbPrivilege`.

```c
struct peios_session_spec {
    uint8_t     logon_type;     /* KACS_LOGON_TYPE_* */
    const char *auth_package;   /* UTF-8; may be "" */
    const void *user_sid;
    size_t      user_sid_len;
};

int peios_session_create(const struct peios_session_spec *spec, uint64_t *id_out);
int peios_session_destroy_empty(uint64_t session_id);
```

- `peios_session_create` creates a logon session of type `logon_type` (`KACS_LOGON_TYPE_*` — interactive, network, service, …) for `user_sid`, attributing it to `auth_package`. `id_out` is mandatory and receives the new session id, which you then pass to `peios_token_builder_session`. Errors: `EPERM` (`SeTcbPrivilege` missing), `EINVAL` (`NULL` spec, `id_out`, or field; malformed SID; oversized spec), `EFAULT` (bad pointer), `ENOMEM` (allocation failed).
- `peios_session_destroy_empty` destroys a session that has **no live tokens** — it fails rather than orphaning tokens. Clean up sessions only after every token referencing them is closed. Errors: `EPERM` (`SeTcbPrivilege` missing), `ENOENT` (no such session), `EBUSY` (live tokens, linked-pair state, or in-flight references).

## Listing sessions

```c
struct peios_logon_session {
    uint64_t       logon_session_id;  /* a token's auth_id */
    uint64_t       created_at;        /* seconds since the Unix epoch */
    uint32_t       logon_type;        /* KACS_LOGON_TYPE_* */
    uint32_t       user_sid_len;
    const uint8_t *user_sid;          /* binary SID */
    const char    *auth_package;      /* UTF-8, not NUL-terminated */
    uint32_t       auth_package_len;
    uint32_t       reserved;
};

peios_logon_sessions *peios_logon_sessions_open(void);
int  peios_logon_sessions_next(peios_logon_sessions *s, struct peios_logon_session *out);
void peios_logon_sessions_close(peios_logon_sessions *s);
```

The kernel's own list of every live logon session, from `/sys/kernel/security/kacs/sessions`. Reading it needs `BUILTIN\Administrators` or SYSTEM.

- `peios_logon_sessions_open` takes the whole listing at once, so a walk sees one moment. `NULL` with `errno` on failure: `EACCES` (not an administrator), `ENOENT` (securityfs not mounted), `ENOMEM`.
- `peios_logon_sessions_next` fills `out` with the next session and returns `1`, or `0` when there are no more. The pointers in `out` stay valid until the next call. `-1` with `EPROTO` means a line it could not read; call again to step past it.
- `peios_logon_sessions_close` frees the reader.

A session is listed while anything holds it, which includes a session nothing has used yet, so a listing can hold sessions with no process in them. SYSTEM's session is `999`. To find a session's processes, read each process's token's `auth_id` (`TokenStatistics`) and match it.

```c
peios_logon_sessions *s = peios_logon_sessions_open();
struct peios_logon_session one;
while (s && peios_logon_sessions_next(s, &one) == 1)
    printf("%llu type %u\n", (unsigned long long)one.logon_session_id, one.logon_type);
peios_logon_sessions_close(s);
```
