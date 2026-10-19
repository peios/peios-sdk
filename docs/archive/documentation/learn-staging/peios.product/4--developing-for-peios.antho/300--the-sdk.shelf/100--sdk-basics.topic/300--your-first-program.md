---
title: Your first program
type: how-to
description: A small, complete C program that parses a security descriptor, reads its owner, and formats the SID back to text — putting the two-call protocol, views, and the error model to work.
related:
  - peios/sdk-conventions/library-conventions
  - peios/sdk-basics/installing-and-linking
  - peios/sdk-security/security-h-security-descriptors
---

This is a complete program you can compile and run. It does not touch the kernel — it works entirely with the security-descriptor *vocabulary* from `<peios/security.h>`, which makes it a safe first outing: no privileges, no live tokens, just the conventions from [Library conventions](~peios/sdk-conventions/library-conventions) put to work.

The task: take a security descriptor written in SDDL text, turn it into wire bytes, read its owner, and print that owner's SID in string form. Along the way you exercise the two-call buffer protocol twice, parse with a zero-copy view, and handle errors the libpeios way.

## The program

```c
#include <peios/security.h>

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void)
{
    /* An SDDL descriptor: owner and group are Administrators (BA), with a
       DACL granting them full access (FA). */
    const char *sddl = "O:BAG:BAD:(A;;FA;;;BA)";

    /* 1. Parse the SDDL text into self-relative SD wire bytes.
          First call probes for the size (cap == 0), then we retrieve. */
    ssize_t need = peios_sddl_parse_sd(NULL, 0, sddl);
    if (need < 0) {
        fprintf(stderr, "parse probe failed: %s\n", strerror(errno));
        return 1;
    }

    unsigned char *sd = malloc((size_t)need);
    if (!sd)
        return 1;

    ssize_t sd_len = peios_sddl_parse_sd(sd, (size_t)need, sddl);
    if (sd_len < 0) {
        fprintf(stderr, "parse failed: %s\n", strerror(errno));
        free(sd);
        return 1;
    }
    printf("security descriptor: %zd bytes\n", sd_len);

    /* 2. Parse the SD into a zero-copy view. The view borrows `sd` — it
          must stay alive and unmodified until we are done reading. */
    peios_sd_view view;
    if (peios_sd_parse(sd, (size_t)sd_len, &view) != 0) {
        fprintf(stderr, "sd parse failed: %s\n", strerror(errno));
        free(sd);
        return 1;
    }

    /* 3. Pull out the owner SID. On success `owner` points INTO `sd`. */
    const void *owner;
    size_t owner_len;
    if (peios_sd_view_owner(&view, &owner, &owner_len) != 0) {
        printf("no owner set\n");
        free(sd);
        return 0;
    }

    /* 4. Format the owner SID as its "S-1-…" string. The byte form of a
          SID is bounded, but its string form is variable, so we probe.
          A string length excludes the NUL, so allocate len + 1. */
    ssize_t slen = peios_sid_format(owner, owner_len, NULL, 0);
    if (slen < 0) {
        fprintf(stderr, "sid format probe failed: %s\n", strerror(errno));
        free(sd);
        return 1;
    }

    char *str = malloc((size_t)slen + 1);
    if (!str) {
        free(sd);
        return 1;
    }
    if (peios_sid_format(owner, owner_len, str, (size_t)slen + 1) < 0) {
        fprintf(stderr, "sid format failed: %s\n", strerror(errno));
        free(str);
        free(sd);
        return 1;
    }

    printf("owner: %s\n", str);

    /* 5. Only now free `sd` — every pointer the view gave us pointed into
          it, so it had to outlive the last read. */
    free(str);
    free(sd);
    return 0;
}
```

## Building and running

```sh
cc first.c $(pkg-config --cflags --libs peios) -o first
./first
```

Expected output:

```
security descriptor: 44 bytes
owner: S-1-5-32-544
```

`S-1-5-32-544` is the well-known SID for the local **Administrators** group — which is exactly the `BA` you wrote in the SDDL string. (The byte count may differ across versions; the owner will not.)

## What just happened

Every convention from the previous page showed up here:

- **The two-call protocol, twice.** `peios_sddl_parse_sd` and `peios_sid_format` were each called first with a `NULL`/`0` buffer to learn the size, then again to fill a right-sized allocation. Neither could ever truncate: a too-small buffer would have returned `-1` with `ERANGE`, not a partial result.
- **A string length excludes the NUL.** That is why the SID string buffer was `slen + 1`.
- **A zero-copy view borrowed our buffer.** `owner` pointed *into* `sd`, so `sd` had to stay alive and unmodified until after the final `peios_sid_format`. Freeing it earlier would have left `owner` dangling — the one mistake this pattern invites, and the reason step 5 frees last.
- **Errors came back through the return value and `errno`.** Every call was checked; `strerror(errno)` explained any failure. No exceptions, no out-of-band error channel.

## A shortcut worth knowing

Not every buffer needs a probe. Some results have a known ceiling, and the library gives you a constant so you can use a fixed stack buffer and skip the first call entirely. A SID is the classic case — it is never larger than `PEIOS_SID_MAX_BYTES`:

```c
unsigned char sid[PEIOS_SID_MAX_BYTES];
ssize_t n = peios_sid_well_known(sid, sizeof sid, PEIOS_WKS_ADMINISTRATORS);
/* n > 0: `sid` holds S-1-5-32-544 in wire form, no malloc, no probe. */
```

Use the probe when a length is genuinely unbounded (strings, whole descriptors, ACLs); use the fixed-size shortcut when the module documents a ceiling for the thing you are building.

## Where to go next

You now have the mechanics. From here, pick the subsystem you need:

- **[Access control (KACS)](~peios/sdk-access-control/overview)** — the security vocabulary you just used, plus tokens, access checks, file security, and process security.
- **[The registry (LCS)](~peios/sdk-registry/overview)** — reading and writing Peios's configuration store.
- **[Events (KMES)](~peios/sdk-events/overview)** — emitting and consuming the audit/event stream.
