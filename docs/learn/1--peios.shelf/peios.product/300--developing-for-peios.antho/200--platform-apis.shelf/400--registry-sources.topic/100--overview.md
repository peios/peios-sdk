---
title: Registry Sources
type: concept
description: Understanding storage backend responsibilities and the request flow through librsi.
---

A registry source stores registry data and answers requests from the
kernel. Use librsi when implementing that storage backend. Applications
that only read or write registry keys use libpeios instead.

The Registry Source Interface (RSI) carries framed requests and responses
on a source file descriptor. The libraries share a release and C calling
conventions, but each exposes its own shared library.

## Request flow

A registry client reaches storage through the kernel:

```mermaid
flowchart LR
    client[Registry client: libpeios] --> kernel[Kernel registry service]
    kernel <-->|RSI requests and responses| source[Storage backend: librsi]
```

The source lifecycle has three stages:

1. Register the hives that the source backs and receive a source descriptor.
2. Read each request, decode it, perform the storage operation, and respond.
3. Close the descriptor when the source stops serving requests.

Registration requires `SeTcbPrivilege`. The source supplies its highest
persisted sequence number so that the kernel can continue sequence
allocation without reusing older values.

## Storage responsibilities

The kernel mediates client access, checks security, resolves registry
layers, and coordinates transactions. The source stores entries, key
metadata, values, and transaction state.

A successful response means that the source performed the operation with
the persistence semantics required by the registry protocol. librsi
encodes and validates messages; it does not implement a database,
transaction journal, or recovery policy for the backend.

## Buffer lifetime

Request decoders return pointers into the received frame. Process or copy
those bytes before reusing the frame buffer. If storage work outlives the
read loop iteration, keep the frame alive or make an owned copy first.

The source sends one response for each request. Preserve its request ID
and operation code when constructing the response. Do not send a success
status for an operation that the backend has not completed.

Start with [registering a
source](~peios/registry-sources/registering-a-source),
then [serving requests](~peios/registry-sources/serving-requests) and
[building responses](~peios/registry-sources/building-responses).
