---
title: process.h — Process security
description: Complete reference for <peios/process.h> — setting process mitigation controls on the process security block (PSB), and reading a process's PSB.
related:
  - peios/sdk-tokens/token-h-tokens-and-sessions
  - peios/process-mitigations/overview
---

`<peios/process.h>` is the process-security surface of KACS. Today it is a small module with two jobs: turning on **process mitigations** — the hardening controls that live on a process's security block (PSB) — and reading a process's PSB back. More process-security surface will land here as it appears.

The mitigation bits are the `KACS_MIT_*` flags from `<pkm/psb.h>` (`KACS_MIT_WXP` through `KACS_MIT_SML`, with `KACS_MIT_ALL` as the mask of all valid bits). `KACS_MIT_CFI` is a legacy alias that expands to `KACS_MIT_CFIF | KACS_MIT_CFIB`. The full catalogue and semantics are in the Peios Kernel TRM §3.3, the Process Security Block.

## Setting mitigations

```c
int peios_process_set_mitigations(int pidfd, uint32_t mitigations);
```

Turns on the mitigation bits named in `mitigations` (a mask of `KACS_MIT_*`). Returns `0` on success, or `-1` with `errno`.

Three properties define how this call behaves, and each matters:

- **It is one-way.** Mitigation bits can only be *set*, never cleared. Once a protection is on, it stays on for the life of the process. This is deliberate — a mitigation you could turn off is a mitigation an attacker could turn off — so treat each call as a permanent, additive commitment.
- **It targets a process by pidfd.** `pidfd == -1` targets the **calling** process, which is the common case: a program hardens itself early in startup. Targeting *another* process requires `PROCESS_SET_INFORMATION` on it **plus** PIP dominance over it — you cannot harden (or interfere with) a process you don't already dominate.
- **It is activation-backed and fails closed.** If a requested protection cannot actually be activated, the call **fails without mutating anything** — you never end up believing a mitigation is on when it isn't. Either every requested bit is activated and the call succeeds, or nothing changes and it returns `-1`.

```c
/* Harden the current process: enforce W^X and shadow-stack, refuse to
   proceed if either can't be activated. */
if (peios_process_set_mitigations(-1, KACS_MIT_WXP | KACS_MIT_SML) != 0) {
    perror("set_mitigations");
    /* nothing was changed; decide whether to continue unhardened or abort */
}
```

Because the call is all-or-nothing, request the bits you require together and check the result once: a success means the whole set is active, a failure means none of *this call's* bits were applied (bits set by earlier successful calls remain on).

## Reading a PSB

```c
struct peios_psb {
    uint32_t pip_type;         /* 0 none, 512 Protected */
    uint32_t pip_trust;        /* 8192 PeiosTcb */
    uint32_t mitigations;      /* committed KACS_MIT_* bits */
    uint8_t  process_guid[16];
};

int peios_process_psb(int pid, struct peios_psb *out);
```

Reads process `pid`'s PSB, from `/proc/<pid>/psb`, into `*out`. `pid <= 0` reads your own. Returns `0`, or `-1` with `errno`.

- **Your own** is always readable.
- **Another process's** needs `PROCESS_QUERY_LIMITED` on its descriptor — which the default process descriptor gives Everyone — and **not** PIP dominance. A protected process's PIP type and trust are therefore readable when nothing else about it is, which is what lets a tool explain why it cannot see more.

`mitigations` is the committed set: bits a call asked for but could not activate are not in it, and neither is the `KACS_MIT_CFI` alias. `process_guid` identifies the process for its whole life, unlike its pid, and is what events carry.

Errors: `ENOENT` (no such process, or it has exited), `EACCES` (refused), `EPROTO` (a line this library cannot read), `EINVAL` (`NULL` `out`).

```c
struct peios_psb psb;
if (peios_process_psb(pid, &psb) == 0 && psb.pip_type != 0)
    printf("%d is protected (trust %u)\n", pid, psb.pip_trust);
```

## See also

- **[`<peios/token.h>`](~peios/sdk-tokens/token-h-tokens-and-sessions)** — PIP dominance is determined by the subject's token; process targeting other than self depends on it.
- **[Process mitigations](~peios/process-mitigations/overview)** — the operator-side account of each mitigation and what it defends against.
