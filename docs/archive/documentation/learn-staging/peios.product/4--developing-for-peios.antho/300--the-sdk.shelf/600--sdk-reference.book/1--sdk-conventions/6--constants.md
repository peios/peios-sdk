---
title: Constants
description: libpeios does not rename the kernel's wire constants — where each set comes from, and why the spelling matches the headers.
---

libpeios does **not** invent its own names for the kernel's wire constants. The access-right bits, ACE types, control flags, and mapping structs all come straight from the `<pkm/*.h>` UAPI headers, and you use those published names directly: `KACS_ACCESS_*`, `KACS_ACE_TYPE_*`, `KACS_SD_*`, `struct kacs_generic_mapping`, and so on. There is no parallel `PEIOS_*` aliasing to translate in your head — the name in the PSD, the name in the kernel header, and the name you write in your code are the same name.

The handful of constants that *are* libpeios's own — buffer-size ceilings like `PEIOS_SID_MAX_BYTES`, and enums for convenience selectors like `enum peios_wks` (well-known SIDs) — are prefixed `PEIOS_` and documented with the module that defines them.
