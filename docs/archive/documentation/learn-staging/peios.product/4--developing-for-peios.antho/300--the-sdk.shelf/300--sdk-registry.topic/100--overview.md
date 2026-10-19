---
title: Registry overview
type: concept
description: "How LCS — the layered registry — works from a developer's seat: keys, values, layers and precedence, transactions, and watches."
related:
  - peios/sdk-registry-api/registry-h-the-registry-lcs
  - peios/registry-concepts/overview
---

LCS — the Layered Configuration Subsystem — is Peios's registry: a kernel-mediated, hierarchical, secured configuration store. If you have used the Windows registry it will feel familiar — a tree of **keys** holding typed **values** — but LCS adds one defining idea: **layers**. This section teaches the client API; [`registry.h`](~peios/sdk-registry-api/registry-h-the-registry-lcs) is the exhaustive reference.

## Keys and values

A **key** is a node in the tree. It has an immutable GUID identity, a KACS [security descriptor](~peios/sdk-security/security-h-security-descriptors) that governs who can read or change it, and it holds values and child keys. You address a key by path and open it for specific `KEY_*` rights, getting back a key fd whose granted access is fixed for its lifetime.

A **value** is a named, typed piece of data on a key (`REG_SZ`, `REG_DWORD`, `REG_BINARY`, and the rest). An empty name is the key's *default* value.

## Layers and precedence — the LCS idea

Here is what makes LCS more than a key/value store. Every value write is tagged with a **layer**, and layers have a fixed precedence order. When you read a value, LCS resolves the **effective** entry — the one from the highest-precedence layer that has something to say — and that's what you get back.

This is what lets configuration compose cleanly:

- A base layer ships default configuration.
- A site or policy layer overlays organisation-wide settings.
- A machine-local layer overrides per-host.

They all coexist on the same key. Reading gives you the winner; writing targets a *specific* layer (or the base layer by default). And because it's layers rather than destructive overwrites, removing a higher layer's entry lets the lower one **re-emerge** — you can override and then un-override without losing the original.

**Tombstones** are the tool for "hide, don't delete": a per-value tombstone masks lower layers for one value, and a blanket tombstone masks all lower values of a key on a layer at once. Keys have the same idea via [hide](~peios/sdk-registry-api/subkeys-metadata-and-watches#deleting-and-hiding-keys).

## Reads report where the answer came from

Because a value can come from any layer, the read tells you *which* layer won and gives you a **sequence number** for the effective entry. That sequence number is the basis of safe updates: pass it back as a compare-and-swap guard on a write, and the write only lands if nothing changed underneath you.

## Transactions

Mutating operations — creating keys, setting and deleting values — can be grouped into a **transaction** and committed atomically, so a multi-step configuration change either fully applies or not at all. Transactions are abort-by-default: close the transaction fd without committing and nothing happens. See [watching and transactions](~peios/sdk-registry/watching-and-transactions).

## Watches

You can ask a key to notify you when it changes — values, subkeys, or its security — optionally across its whole subtree. The elegant part: once armed, **the key fd itself becomes pollable**, so a registry watch drops straight into an `epoll` loop with no side channel.

## The client, not the source

This SDK is the registry **client** — it reads and writes the store. It does not implement a registry **source** (a storage backend); that's a separate library, [**librsi**](~peios/registry-sources/overview). As a client you speak only the calls in [`registry.h`](~peios/sdk-registry-api/registry-h-the-registry-lcs).

## Where to go in this section

- **[Reading and writing](~peios/sdk-registry/reading-and-writing)** — open a key, read effective values, write to layers, do safe updates.
- **[Watching and transactions](~peios/sdk-registry/watching-and-transactions)** — react to changes and apply atomic multi-step edits.
- **[`registry.h` reference](~peios/sdk-registry-api/registry-h-the-registry-lcs)** — every call in full.
