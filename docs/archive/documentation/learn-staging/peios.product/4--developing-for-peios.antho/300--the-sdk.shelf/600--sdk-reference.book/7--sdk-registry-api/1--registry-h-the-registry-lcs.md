---
title: registry.h — The registry (LCS)
description: The registry client — opening keys, reading and writing values, enumerating, watching, securing, backing up and running transactions.
---

`<peios/registry.h>` is the client surface of **LCS** — the Layered Configuration Subsystem, Peios's kernel-mediated registry. LCS is modelled on the Windows registry: a hierarchy of **keys** (each with an immutable GUID identity and secured by its own KACS security descriptor) holding typed **values**. Its distinguishing feature is **layers**: every write is tagged with a precedence-ordered layer, and the *effective* view of a value resolves to the highest-precedence entry. That is what lets a base configuration, a site overlay, and a machine-local override coexist on one key and resolve deterministically.

This header is the registry **client**: open keys, read and write values, enumerate, watch, secure, back up, and run transactions. It does **not** cover the registry *source* (the storage backend) side — `REG_SRC_REGISTER` and the RSI framed protocol — which is a separate library, [**librsi**](~peios/registry-sources/overview). A client speaks only the syscalls and ioctls here.

**Handles are fds.** Three calls create file descriptors — `peios_reg_open_key`, `peios_reg_create_key`, and `peios_reg_begin_transaction`; everything else is an operation on a key fd or transaction fd, gated on the access right granted when the key was opened. The wire constants — value types (`REG_SZ` … `REG_QWORD`), key access rights (`KEY_*`), open/create flags, transaction states (`REG_TXN_*`), watch filters (`REG_NOTIFY_*`), and security-info bits — come from `<pkm/lcs.h>`.

## See also

- **[`<peios/security.h>`](~peios/sdk-security/security-h-security-descriptors)** — building and parsing the SDs that secure keys.
- **[Library conventions](~peios/sdk-conventions/library-conventions)** — the base error and buffer rules the descriptor reads specialise.
- **[The registry](~peios/registry-concepts/overview)** — the operator-side model of layers, hives, and precedence.
