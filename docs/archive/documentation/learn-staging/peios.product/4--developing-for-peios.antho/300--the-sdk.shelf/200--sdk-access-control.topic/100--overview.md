---
title: Access control overview
type: concept
description: How the KACS pieces fit together from a developer's seat — identities, tokens, security descriptors, and access checks — and which part of the SDK you reach for.
related:
  - peios/sdk-security/security-h-security-descriptors
  - peios/sdk-tokens/token-h-tokens-and-sessions
  - peios/sdk-access/access-h-access-checks
  - peios/identity/overview
---

KACS — the Kernel Access Control Subsystem — is the heart of Peios's security model, and it is the part of the SDK you are most likely to reach for. This section is a tour: it explains how the pieces fit together and points you at the right guide (and the right reference page) for each task. If you have not yet read the [library conventions](~peios/sdk-conventions/library-conventions), read those first — everything here assumes the error and buffer rules they describe.

## The four nouns

Almost everything in KACS is built from four things:

| Noun | What it is | SDK home |
|---|---|---|
| **SID** | The unique binary name of a principal — a user, group, machine, or well-known system actor. | [`security.h`](~peios/sdk-security/sids) |
| **Security descriptor (SD)** | What protects an object: its owner, its group, and the ACLs that grant or deny access. Built from ACEs, each naming a SID and an access mask. | [`security.h`](~peios/sdk-security/building-security-descriptors) |
| **Token** | The runtime object that carries an identity — a user SID, groups, privileges, an integrity level, claims. Every access decision is made *against a token*. A token is a file descriptor. | [`token.h`](~peios/sdk-tokens/token-h-tokens-and-sessions) |
| **Access check** | The act of deciding whether a token may perform a desired access on an object, given the object's SD. | [`access.h`](~peios/sdk-access/access-h-access-checks) |

The relationship is simple to state: an **access check** asks whether a **token** (the subject) is granted some access to an object protected by a **security descriptor**, and both the token and the SD are expressed in terms of **SIDs**.

## The shape of a decision

When you need to make an authorisation decision in your own code, the pattern is almost always the same three steps:

1. **Get the subject's token.** Usually the caller's — often via [`peios_token_open_peer`](~peios/sdk-tokens/opening-and-creating-tokens) on a socket, so you learn who connected — or your own effective token (`token_fd = -1`).
2. **Get the object's security descriptor.** You either hold it already, read it off a file with [`peios_file_get_sd`](~peios/sdk-files/file-h-file-security), or build one with a [`peios_sd_builder`](~peios/sdk-security/building-security-descriptors).
3. **Run the check.** [`peios_access_check`](~peios/sdk-access/the-check) tells you whether the desired rights are granted, and exactly which subset was granted.

The [checking access](~peios/sdk-access-control/checking-access) guide walks that end to end.

## Advisory versus enforced

There is one distinction worth internalising early. The SDK's access check is **advisory**: it computes what the answer *would* be. It does not *enforce* anything — enforcement of a real operation (opening a file, adjusting a token) happens inside the kernel against the subject's own process security block.

So you use `peios_access_check` when **your** code is the resource manager: you own some object — a record in your database, a slot in your service — that isn't a kernel object, and you want to make the grant/deny decision using Peios identities and the same rules the kernel would apply. For actual kernel objects (files, tokens, registry keys), you don't pre-check and then act; you just act, and the kernel enforces, returning `EACCES` if denied.

## Identity is not always *your* identity

A recurring theme in KACS is acting as someone other than yourself:

- **Impersonation** lets a service temporarily adopt a caller's identity so its access checks run as *them* — the way a server does work "on behalf of" a client without running as root and hand-rolling permission logic. See [working with tokens](~peios/sdk-access-control/working-with-tokens).
- **Restricted tokens** let you *drop* power — derive a strictly less-privileged token to hand to less-trusted code.
- **Integrity levels** and **confinement** bound what a token can touch regardless of its SIDs.

These are what make KACS more than POSIX uid/gid, and the token module is where you reach for them.

## Where to go in this section

- **[Working with tokens](~peios/sdk-access-control/working-with-tokens)** — who am I, who is calling me, and how to act as someone else.
- **[Checking access](~peios/sdk-access-control/checking-access)** — building a security descriptor and making a decision end to end.
- **[Securing files](~peios/sdk-access-control/securing-files)** — the native open and reading/writing file security descriptors.
- **[Hardening a process](~peios/sdk-access-control/hardening-a-process)** — turning on process mitigations.

For the exhaustive per-function detail behind any of these, the [reference section](~peios/sdk-security/security-h-security-descriptors) documents every symbol.
