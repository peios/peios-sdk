---
title: What is the Peios SDK
type: concept
description: The Peios SDK is the C-ABI library family you use to talk to Peios from your own programs — access control, the registry, and events. Its first and largest member is libpeios.
related:
  - peios/sdk-basics/installing-and-linking
  - peios/sdk-conventions/library-conventions
  - peios/identity/overview
---

The Peios SDK is how your own software talks to Peios.

Peios exposes its security world — identities, tokens, access checks, file security, the registry, and the audit/event stream — through a kernel interface built out of raw syscalls, ioctls, and packed byte buffers in the MS-DTYP wire formats. That interface is precise, but it is not something you want to hand-assemble from C. The SDK is the layer that lifts it into ordinary functions you can call: build a SID, open a token, run an access check, read a registry value, emit an event.

It is a **C ABI**, not a C-only library. The shipping product is a set of hand-written C headers and a shared object with a frozen application binary interface. Anything that can call C — Go via cgo, Rust via `bindgen`, Python via `ctypes`, Zig, C++ — can link it and get the same surface. Go applications can instead use the independent, pure-Go [peios-go SDK](~peios/sdk-basics/using-the-sdk-from-go), which requires no C FFI. The library happens to be implemented in Rust, but that is invisible across the boundary: callers see `peios_*` functions, `<peios/*.h>` headers, and errno.

## Who it's for

You want the SDK if you are **building software that runs on Peios and needs to participate in its security model** — a service that checks whether a caller may perform an action, a tool that reads or writes security descriptors, an agent that emits audit events, or anything that reads and writes the registry.

You do *not* need it to simply *run* on Peios. Ordinary POSIX programs run under Peios's Linux-compatibility surface without ever linking libpeios. The SDK is for programs that want to reach past POSIX and speak KACS, LCS, and KMES directly. If you are administering a running system rather than writing code against it, the [Peios operator documentation](~peios/identity/overview) is the place to start.

## The three surfaces

Peios's kernel boundary has three subsystems, and the SDK mirrors them one-to-one. Everything in the SDK belongs to one of these:

| Subsystem | What it is | SDK headers |
|---|---|---|
| **KACS** — Kernel Access Control Subsystem | Identities, tokens, access decisions, file security, and process security. The heart of the model. | `<peios/security.h>`, `<peios/token.h>`, `<peios/access.h>`, `<peios/file.h>`, `<peios/process.h>` |
| **LCS** — the registry | A hierarchical, transactional, secured key/value store — Peios's system configuration database. | `<peios/registry.h>` |
| **KMES** — the event system | The msgpack-framed audit and event stream: emit events, consume them. | `<peios/msgpack.h>`, `<peios/event.h>` |

The umbrella header `<peios.h>` pulls in all of them; you can also include the individual concept headers for a tighter compile surface.

## libpeios, librsi, and the substrate

The SDK is **two libraries** on a shared foundation, each with its own role:

- **libpeios** — the userspace C-ABI library for KACS, LCS, and KMES. It is the registry *client* and the whole of the access-control and event surface, and it is what the bulk of this documentation describes. Link `-lpeios`, include `<peios.h>`.
- **librsi** — the library for implementing a registry *source* (a storage backend): the provider counterpart to libpeios's registry client. Where libpeios *reads and writes* the registry, librsi is what a program uses to *be* the thing that holds the data and answers the kernel. Link `-lrsi`, include `<rsi.h>`. See [Registry sources](~peios/registry-sources/overview).
- **peios-cabi** — the internal C-ABI substrate both libraries are built on (the allocator wiring, the errno slot, the syscall/ioctl wrappers, the getxattr-style buffer helpers). You never link it directly; it is shared plumbing, documented here only so the conventions it sets make sense.

So the registry has two sides in this SDK: the **client** side in libpeios (open keys, read/write values) and the **source** side in librsi (back the keys and values, serve the kernel's requests). Most programs are clients; you reach for librsi only when you are providing storage.

Because the two libraries share one substrate, one error model, and one audience, they share one doc set. Learn the conventions once and they hold across both.

## What this documentation promises

This is user-facing documentation, not a specification. The authoritative byte-level contracts live in the PCSA specifications — PCDS for the core data structures, PGSS, PSPK and PSPU for the interfaces — and in the `<pkm/*.h>` kernel headers; where a detail is normative, this documentation points you at the specification that owns it. Where a detail is about *this implementation* rather than the contract, it points at the Peios Kernel manual instead. What you get *here* is the working knowledge to use the library well: what every function does, what it returns, how memory and errors flow, and how the pieces fit together — with enough coverage that you should not need to open the library's source to answer a question. When you find a gap, that is a documentation bug worth reporting.

## Where to go next

- **[Installing and linking](~peios/sdk-basics/installing-and-linking)** — the packages, the headers, and how to build against the library.
- **[Library conventions](~peios/sdk-conventions/library-conventions)** — the error model, the memory rules, and the buffer protocol that every function in the SDK follows. Read this once; it saves you re-learning it per module.
- **[Your first program](~peios/sdk-basics/your-first-program)** — a small, complete program you can compile and run.
