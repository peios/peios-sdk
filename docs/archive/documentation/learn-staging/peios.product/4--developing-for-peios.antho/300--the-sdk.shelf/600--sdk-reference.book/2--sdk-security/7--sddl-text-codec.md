---
title: SDDL text codec
description: Converting between the binary forms and human-readable SDDL text, entirely in userspace, including conditional expressions.
---

The SDDL codec converts between the binary wire forms above and their human-readable SDDL text (MS-DTYP §2.5.1). This is a **pure-userspace facility** — the kernel speaks only binary — so it lives entirely in libpeios. All four entries use the two-call protocol (`cap == 0` to probe) and fail with `EINVAL` on malformed input.

```c
ssize_t peios_sddl_parse_sd(void *out, size_t cap, const char *sddl);
ssize_t peios_sddl_format_sd(char *out, size_t cap, const void *sd, size_t sd_len);
```

- `peios_sddl_parse_sd` parses SDDL text (e.g. `"O:SYG:BAD:(A;;FA;;;BA)"`) into self-relative SD wire bytes.
- `peios_sddl_format_sd` renders SD wire bytes back to a NUL-terminated SDDL string (length excludes the `NUL`, so allocate `len + 1`).

These are the friendliest way to construct a descriptor when you have one written down — parse the string rather than assembling ACEs by hand — and the friendliest way to log or display one.

### Conditional expressions

Callback ACEs carry a *conditional expression* as compiled "artx" bytecode. The codec converts between that bytecode and its SDDL expression text:

```c
ssize_t peios_sddl_parse_condition(void *out, size_t cap, const char *expr);
ssize_t peios_sddl_format_condition(char *out, size_t cap, const void *artx, size_t len);
```

- `peios_sddl_parse_condition` compiles an expression such as `@User.Title == "PM"` into the bytecode you place in a callback ACE's `app_data`.
- `peios_sddl_format_condition` renders bytecode back to text (with no outer parentheses), length excluding the `NUL`.

So the round trip for a conditional ACE is: write the condition as text → `peios_sddl_parse_condition` → put the bytecode in `peios_ace_spec.app_data` with a callback ACE `type` → add it to an ACL builder.
