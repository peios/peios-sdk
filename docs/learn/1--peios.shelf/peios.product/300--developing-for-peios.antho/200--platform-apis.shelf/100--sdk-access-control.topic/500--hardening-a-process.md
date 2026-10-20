---
title: Hardening a Process
type: how-to
description: Turn on process mitigations to harden your program, and understand the one-way, fail-closed semantics.
related:
  - peios/sdk-processes/process-h
  - kernel/internals/kacs/process-security/psb-fields
---

Peios lets a process opt into hardening — exploit mitigations enforced by the
kernel on that process's security block. The SDK exposes this through a single
call,
[`peios_process_set_mitigations`](~peios/sdk-processes/process-h#setting-mitigations).
This short guide covers using it well; the full flag set and semantics are in
[`process.h`](~peios/sdk-processes/process-h) and the [operator
docs](~kernel/internals/kacs/process-security/psb-fields).

## Harden yourself at startup

The common case is a program hardening itself early in `main`, before it
processes any untrusted input:

```c
#include <stdio.h>

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

`pidfd == -1` targets the calling process. `mitigations` is a mask of
`KACS_MIT_*` bits (from `<pkm/psb.h>`); combine the ones you want and set them
together.

## Three things to know

**It's one-way.** Mitigation bits can only be *set*, never cleared — once on,
they stay on for the life of the process. That's the point: a mitigation you
could turn off is one an attacker could turn off. Treat each call as a
permanent commitment.

**It's all-or-nothing, and fails closed.** If any requested protection can't
actually be activated, the call changes *nothing* and returns `-1`. You never
end up believing a mitigation is on when it isn't. So request the set you
require together and check the result once — success means the whole set is
active. (Bits from earlier successful calls stay on regardless.)

**Targeting another process is privileged.** To harden a process other than
your own, you need `PROCESS_SET_INFORMATION` on it *and* PIP dominance over it
— you can't reach into a process you don't already dominate. For
self-hardening (`-1`), neither is needed.

## Choosing what to enable

The bit catalog and what each mitigation defends against is documented
operator-side under [process mitigations](~kernel/internals/kacs/process-security/psb-fields)
(and in the Peios Kernel TRM §3.3, the Process Security Block). A couple of
notes for the SDK caller:

- `KACS_MIT_ALL` is the mask of all valid bits — useful for validating input,
  not usually what you'd blanket-enable without thought.
- `KACS_MIT_CFI` is a legacy alias that expands to `KACS_MIT_CFIF |
  KACS_MIT_CFIB`.

Enable the specific protections your program can tolerate, verify the call
succeeded, and prefer to do it before you touch untrusted data.

## Activation and lifecycle

A valid mask can still fail because a platform cannot activate the protection
or existing mappings violate it. Late WXP, TLP, or LSV requests validate
existing state; generating incompatible mappings first does not exempt them.
TLP and LSV inspect file-backed executable mappings, while WXP also constrains
anonymous executable memory. Plan JIT and library-loading behavior before
choosing a policy.

Architecture support is part of activation. The maintained kernel refuses
live CFIF requests with `ENODEV`; CFIB activation is self-only. Having rights
to another process does not overcome either limitation. SML can be satisfied
by a platform reporting itself unconditionally unaffected. UI_ACCESS remains
reserved; do not use the valid-bit mask as a recommended hardening profile.

Fork inherits committed mitigations. Exec preserves them, while recomputing
PIP from the executable. PIE governs subsequent exec and NO_CHILD governs
subsequent process creation; neither retroactively changes the running image
or removes existing children. NO_CHILD permits thread creation and exec.

Handle failures before accepting untrusted work. Read back the committed
mask and correlate it with the process GUID; a requested mask or an earlier
process's report is not evidence about a replacement process. Self-targeting
removes the cross-process authority gates, not input and activation checks.
A failed request leaves earlier successful commitments intact.

Do not assume a generic service launcher applies a mitigation mask. Verify
the deployed launcher and the running process. The maintained Peinit launch
path does not add a per-service mitigation request; inherited flags and
application self-hardening remain separate.

## Next

- **[`process.h` reference](~peios/sdk-processes/process-h)** — the call in
  full.
- **[Process mitigations](~kernel/internals/kacs/process-security/psb-fields)** — every
  mitigation and its threat model.
