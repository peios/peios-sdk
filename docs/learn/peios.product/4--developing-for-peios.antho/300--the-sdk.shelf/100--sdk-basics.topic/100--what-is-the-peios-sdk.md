---
title: The Peios C SDK
type: concept
description: Choosing the C library for security, registry access, events, or registry storage.
---

The Peios C software development kit (SDK) provides libraries for programs
that use Peios security, registry, and event services. Use libpeios for
application services and librsi for a registry storage backend.

Both libraries expose a C application binary interface (ABI). C++ and other
languages can call that interface without depending on the libraries'
internal Rust implementation.

## Choosing a library

The libraries have separate roles and can be linked into the same program:

| Library | Role | Header | Link option |
|---|---|---|---|
| libpeios | Security, registry clients, and events | `<peios.h>` | `-lpeios` |
| librsi | Registry storage backends | `<rsi.h>` | `-lrsi` |

The internal `peios-cabi` crate supplies allocation, error handling, and
system-call helpers. Applications do not link it directly.

The SDK repository releases these components together. The shared objects
retain separate compatibility boundaries: `libpeios.so.0` and `librsi.so.0`.
A package version identifies a release; the shared object's SONAME identifies
its binary interface. An SDK release does not imply a kernel release.

## Portable data and kernel services

Security identifiers, access control lists, security descriptors, and
MessagePack values can be built and parsed in userspace. These operations
do not need a running Peios kernel.

Opening tokens, accessing registry keys, and sending or receiving kernel
events require the corresponding Peios kernel services. Building a program
on a development host does not establish that these operations work there.

The SDK pins kernel headers and shared semantic crates when it is built.
That source pin identifies the build input. It is not a requirement to run
that exact kernel commit, and it does not establish compatibility with
every kernel version.

## Starting with C

Begin with [installing and linking](~peios/sdk-basics/installing-and-linking),
then [compile a complete program](~peios/sdk-basics/your-first-program).
Read [library conventions](~peios/sdk-conventions/library-conventions)
before managing buffers or file descriptors.

For a registry backend, start with
[registry sources](~peios/registry-sources/overview). The installed headers
own the complete function declarations and parameter contracts. The SDK
reference explains how to use those contracts.
