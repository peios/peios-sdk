---
title: access.h — Access checks
description: What access.h is for, the two things worth knowing before using it, and the conventions it assumes.
---

`<peios/access.h>` answers the central question of the whole access-control model: *may this subject perform this access on this object?* You hand it a token, a security descriptor, and a desired access mask, and it runs the full KACS AccessCheck pipeline and tells you whether access is granted and exactly which rights were granted.

Two things are worth saying up front:

- **These calls are advisory.** They *evaluate*, they do not *enforce*. `peios_access_check` tells you what the answer would be; enforcement of a real operation always runs inside the kernel against the subject's own process security block. Use these when *your* code is the resource manager — you hold an object, you have its security descriptor, and you need to make the grant/deny decision yourself.
- **A denial is a normal result, not an error.** Per the [library conventions](~peios/sdk-conventions/how-results-are-returned#structured-results-out-parameters), a denied check returns `-1` with `errno == EACCES`, and the granted mask is still written out. Only a genuine failure (a bad token fd, a malformed SD) is an error in the usual sense.

## See also

- **[`<peios/security.h>`](~peios/sdk-security/security-h-security-descriptors)** — building the security descriptors and reading the generic-mapping tables this check consumes.
- **[`<peios/token.h>`](~peios/sdk-tokens/token-h-tokens-and-sessions)** — obtaining the `token_fd` to check, and `peios_token_generic_mapping`.
- **[Access decisions](~peios/access-decisions/overview)** — the operator-side account of how KACS reaches a grant/deny decision.
