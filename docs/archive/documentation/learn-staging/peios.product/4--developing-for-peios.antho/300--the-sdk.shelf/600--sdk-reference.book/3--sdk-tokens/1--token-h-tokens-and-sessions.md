---
title: token.h — Tokens and sessions
description: What token.h covers — opening, creating and adjusting tokens, querying them, and logon sessions — plus the constants it assumes.
---

`<peios/token.h>` is the token surface of KACS. A **token** is the runtime object that carries an identity — a user SID, group SIDs, privileges, an integrity level, claims — and every access decision is made against one. This module lets you open the tokens that already exist (your own, another process's, a socket peer's), mint new ones, read their contents, transform them, and install or impersonate them.

**A token handle is a file descriptor.** Every open/create/duplicate call returns a raw `int` fd, `O_CLOEXEC` by default, that you close with `close()`. The `access` argument several calls take is the desired *handle-right* mask (`KACS_TOKEN_*`), access-checked against the token's own security descriptor and cached on the fd — a handle only lets you do what its rights allow.

The wire constants (`KACS_TOKEN_*`, `KACS_IMLEVEL_*`, `KACS_SE_*_PRIVILEGE`, `KACS_TOKEN_CLASS_*`, `KACS_LOGON_TYPE_*`) and the ioctl arg structs (`kacs_priv_entry`, `kacs_group_entry`) come from `<pkm/token.h>`. Query payloads that are SID arrays or ACLs are read with the [views in `<peios/security.h>`](~peios/sdk-security/parsing-views).

The module divides into: **opening & creating**, the **token-spec builder**, **query**, **adjust/transform**, and **logon sessions**.

## See also

- **[`<peios/security.h>`](~peios/sdk-security/security-h-security-descriptors)** — the SID/ACL/SD vocabulary and the views used to parse group and privilege query payloads.
- **[`<peios/access.h>`](~peios/sdk-access/access-h-access-checks)** — checking access with a token fd.
- **[Tokens](~peios/tokens/overview)** and **[Impersonation](~peios/impersonation/overview)** — the operator-side model.
