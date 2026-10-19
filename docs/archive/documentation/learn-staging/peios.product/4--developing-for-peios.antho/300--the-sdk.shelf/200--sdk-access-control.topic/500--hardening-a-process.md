---
title: Hardening a process
type: how-to
description: Turn on process mitigations to harden your program, and understand the one-way, fail-closed semantics.
related:
  - peios/sdk-processes/process-h
  - peios/process-mitigations/overview
---

Peios lets a process opt into hardening — exploit mitigations enforced by the kernel on that process's security block. The SDK exposes this through a single call, [`peios_process_set_mitigations`](~peios/sdk-processes/process-h#setting-mitigations). This short guide covers using it well; the full flag set and semantics are in [`process.h`](~peios/sdk-processes/process-h) and the [operator docs](~peios/process-mitigations/overview).

## Harden yourself at startup

The common case is a program hardening itself early in `main`, before it processes any untrusted input:

```c
#include <peios/process.h>

int main(void)
{
    /* Enforce W^X and speculation mitigation lock; abort on failure. */
    if (peios_process_set_mitigations(-1, KACS_MIT_WXP | KACS_MIT_SML) != 0) {
        perror("set_mitigations");
        return 1;   /* refuse to run unhardened */
    }
    /* ... the rest of the program runs with those protections on ... */
}
```

`pidfd == -1` targets the calling process. `mitigations` is a mask of `KACS_MIT_*` bits (from `<pkm/psb.h>`); combine the ones you want and set them together.

## Three things to know

**It's one-way.** Mitigation bits can only be *set*, never cleared — once on, they stay on for the life of the process. That's the point: a mitigation you could turn off is one an attacker could turn off. Treat each call as a permanent commitment.

**It's all-or-nothing, and fails closed.** If any requested protection can't actually be activated, the call changes *nothing* and returns `-1`. You never end up believing a mitigation is on when it isn't. So request the set you require together and check the result once — success means the whole set is active. (Bits from earlier successful calls stay on regardless.)

**Targeting another process is privileged.** To harden a process other than your own, you need `PROCESS_SET_INFORMATION` on it *and* PIP dominance over it — you can't reach into a process you don't already dominate. For self-hardening (`-1`), neither is needed.

## Choosing what to enable

The bit catalogue and what each mitigation defends against is documented operator-side under [process mitigations](~peios/process-mitigations/overview) (and in the Peios Kernel TRM §3.3, the Process Security Block). A couple of notes for the SDK caller:

- `KACS_MIT_ALL` is the mask of all valid bits — useful for validating input, not usually what you'd blanket-enable without thought.
- `KACS_MIT_CFI` is a legacy alias that expands to `KACS_MIT_CFIF | KACS_MIT_CFIB`.

Enable the specific protections your program can tolerate, verify the call succeeded, and prefer to do it before you touch untrusted data.

## Next

- **[`process.h` reference](~peios/sdk-processes/process-h)** — the call in full.
- **[Process mitigations](~peios/process-mitigations/overview)** — every mitigation and its threat model.
