---
title: Compiling a First Program
type: how-to
description: Parsing a security descriptor and formatting its owner from a complete C program.
---

Parse a security descriptor, read its owner, and print the owner's security
identifier (SID). This example runs entirely in userspace and does not
open tokens or change system security.

## Before you start

Prepare these inputs:

- A C compiler and `pkg-config`.
- The libpeios development package and its matching runtime library.
- A writable directory for the source and executable.

## Creating the program

Save this complete program as `first.c`:

```c
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <peios/security.h>

int main(void)
{
    /* An SDDL descriptor: owner and group are Administrators (BA), with a
       DACL granting them full access (FA). */
    const char *sddl = "O:BAG:BAD:(A;;FA;;;BA)";

    /* 1. Parse the SDDL text into self-relative SD wire bytes.
          First call probes for the size (cap == 0), then retrieve. */
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
          must stay alive and unmodified until the final read. */
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
        fprintf(stderr, "owner lookup failed: %s\n", strerror(errno));
        free(sd);
        return 1;
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

    /* 5. Free `sd` — every pointer the view returned pointed into
          it, so it had to outlive the last read. */
    free(str);
    free(sd);
    return 0;
}
```

The Security Descriptor Definition Language (SDDL) string sets the owner
and group to Administrators. Its discretionary access control list (DACL)
grants that group full file access. Parsing this string does not grant any
access to the process running the example.

## Building and running

1. Compile the source with the package's compiler and linker options:

   ```sh
   cc first.c $(pkg-config --cflags --libs peios) -o first
   ```

2. Run the executable:

   ```sh
   ./first
   ```

## Checking the result

The program prints:

```text
security descriptor: 84 bytes
owner: S-1-5-32-544
```

The owner SID identifies Administrators. The byte count describes the
encoded descriptor produced by this example, including its owner, group,
and access control list.

## Understanding ownership

The program probes the output size before allocating each variable-sized
buffer. The string allocation adds one byte for the terminating NUL.

The parsed view and owner pointer borrow the descriptor allocation. The
program frees that allocation after formatting the owner, so every borrowed
pointer remains valid throughout its use.

Read [library conventions](~peios/sdk-conventions/library-conventions) for
the result and ownership rules used by the other SDK functions.
