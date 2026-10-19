---
title: Access masks
description: A 32-bit set of rights, the four generic bits that stand in for concrete ones, and how a generic mapping resolves them.
---

An access mask is a 32-bit set of rights. Masks may contain four *generic* bits (`KACS_ACCESS_GENERIC_READ/WRITE/EXECUTE/ALL`) that stand in for object-specific rights until they are mapped to a concrete object class.

```c
uint32_t peios_access_map_generic(uint32_t mask,
                                  const struct kacs_generic_mapping *m);
```

`peios_access_map_generic` folds the generic bits of `mask` into object-specific rights using the mapping `m`, and clears the generic bits from the result. Each object class publishes its canonical mapping as a data symbol you pass here — `peios_file_generic_mapping` (from [`<peios/file.h>`](~peios/sdk-files/file-h-file-security)) and `peios_token_generic_mapping` (from [`<peios/token.h>`](~peios/sdk-tokens/token-h-tokens-and-sessions)). Use it when you have a mask written in generic terms (say, from an SDDL string using `GR`/`GW`) and need the concrete rights for a specific object type.
