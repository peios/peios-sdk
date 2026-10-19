---
title: How results are returned
description: The three return shapes an entry point can have, and how each tells you where to read the result and how to detect failure.
---

Every entry point reports success or failure through its return type. There are three return shapes, and the shape tells you how to read the result.

### `int` — a file descriptor, or zero

A function returning `int` returns either:

- a **file descriptor** (a non-negative `int`), when its job is to open something — a token, a registry key, an event stream; or
- **`0`** on success, when it performs an action with no handle to hand back; and
- **`-1` on failure**, with the reason in `errno`.

```c
int fd = peios_token_open_self(/* … */);
if (fd < 0) {
    /* errno is set — perror(), strerror(errno), etc. */
}
```

### `ssize_t` — a byte length

A function returning `ssize_t` produces a **variable-length result** — a SID, a serialised security descriptor, a formatted string, a registry value. It returns:

- the **length in bytes** of the result on success (`>= 0`); or
- **`-1` on failure**, with the reason in `errno`.

These are the functions that use the [two-call buffer protocol](~peios/sdk-conventions/the-two-call-buffer-protocol) below. The returned length is always the *full* length of the result, which is what makes the protocol work.

For functions that format a **string**, the returned length excludes the terminating `NUL` — exactly like `snprintf`. So a return of `41` means "41 characters plus a NUL"; size your buffer as `len + 1`.

### Structured results — out-parameters

When a call produces more than one value, or a value that isn't naturally a length or an fd, it writes through **out-parameters** and returns `int` (`0` / `-1`). The access check is the archetype: it returns `0` when access is granted and `-1` with `errno == EACCES` when it is denied, and it writes the *granted access mask* through an out-parameter either way.

```c
uint32_t granted = 0;
int rc = peios_access_check(/* … */, &granted);
/* rc == 0: granted; rc == -1 && errno == EACCES: denied.
   `granted` is populated in both cases. */
```

A denial is a normal, expected outcome, not a bug — which is why it is reported the same disciplined way as any other errno, rather than through a separate channel.

### errno

Failure is *always* reported through the standard C `errno`. The library sets `errno` on every `-1` return and uses ordinary, portable errno values — there are no libpeios-specific or PKM-specific error numbers to learn. The ones you will see most:

| errno | Meaning in libpeios |
|---|---|
| `EINVAL` | Malformed input — a bad SID, an unparseable SDDL string, an argument out of range. |
| `ERANGE` | Your output buffer was non-zero but too small. Nothing was written. (See the protocol below.) |
| `EACCES` | An access check denied the request. |
| `ENOMEM` | An allocation failed (for the heap-backed builders). |
| `EBADF`, `ESRCH`, `EFAULT` | The usual Linux meanings — a bad fd or pidfd, a vanished process, a bad pointer. |

Because the values are standard, `strerror`, `perror`, and your language's normal errno handling all work unchanged. Check the return value first, *then* read `errno` — like any POSIX call, `errno` is only meaningful after a call that signalled failure.

> Nothing ever unwinds across the boundary. The library is compiled to abort rather than propagate a panic through the C ABI, so a call either returns a value you can inspect or the process dies — it never leaves you with a corrupt half-state to reason about.
