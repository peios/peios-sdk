---
title: Registering a Source
type: how-to
description: Declare the hives your source backs and register with the kernel to obtain the source fd.
related:
  - peios/sdk-rsi-source/rsi-source-h-becoming-a-source
  - peios/registry-sources/serving-requests
---

Every source starts by registering. You tell the kernel which hives your
process backs, and it hands you a source fd to serve on. The full detail is in
[`rsi/source.h`](~peios/sdk-rsi-source/rsi-source-h-becoming-a-source); this
guide walks the decisions.

## Declare your hives

A hive is a registry subtree with its own root key. Fill in one `struct
rsi_hive` per hive your source backs:

```c
struct rsi_hive hive = {
    .name     = "MyStore",
    .name_len = 7,
    .flags    = 0,                    /* global hive */
    .root_guid = { /* 16-byte GUID of the hive root key */ },
    /* scope_guid left zero for a global hive */
};
```

The two decisions per hive:

- **Global or private?** A **global** hive (`flags = 0`, `scope_guid` zero) is
  visible system-wide. A **private** hive (`flags = RSI_HIVE_PRIVATE`,
  `scope_guid` non-zero) is scoped — only tokens carrying that scope GUID in
  their [LCS
credentials](~peios/sdk-tokens/the-token-spec-builder#lcs-registry-credentials)
  can resolve it. Use a private hive for per-application or per-tenant state
  that shouldn't be system-visible.
- **The root GUID.** Every hive is anchored at a root key identified by
  `root_guid`. This is the GUID paths in the hive resolve from, and it must
  match the root your storage actually holds.

## Register

Hand the kernel your hive array and the highest sequence number you've already
persisted:

```c
int src = rsi_register(&hive, 1, /*max_sequence=*/0);
if (src < 0) {
    perror("rsi_register");   /* EPERM if you lack SeTcbPrivilege */
    return -1;
}
/* `src` is the source fd — serve the RSI protocol on it. */
```

Two things to get right:

- **`SeTcbPrivilege` is required.** Registration is a trusted operation;
  without the privilege you get `EPERM`. Sources run as trusted system
  components.
- **`max_sequence` is your durability contract.** It is the highest sequence
  number this source has *already persisted*. The kernel resumes its global
  sequence counter past it, so it never hands out a number you've used. A
  brand-new source with no stored state passes `0`; a source restarting with
  durable data must scan that data for the highest sequence it ever wrote and
  pass *that*. Getting this wrong risks sequence reuse, so make it the first
  thing your restart path computes.

You can register several hives in one call by passing an array and a `count`
greater than one; the kernel enforces its configured `MaxHivesPerSource` limit
(`ENOSPC` if you exceed it).

## Optional transaction lifetime control

The kernel also supports lifetime extension version 1. A source using it opens
`/dev/pkm_registry`, calls `REG_SRC_NEGOTIATE` with version 1 and zero flags,
retains the returned session identifier, and then calls `REG_SRC_REGISTER`.
The current `rsi_register` convenience function uses the legacy registration
sequence; negotiate through the explicit device/ioctl sequence when adopting
this extension.

After reading each request, query `REG_SRC_REQUEST_CONTEXT` with its request
id and the negotiated session. It returns the original request deadline and
any transaction lifetime and cleanup-lease deadlines, all in `CLOCK_MONOTONIC`
nanoseconds. Renew live transactions using `REG_SRC_RENEW`, independently of
the normal request queue. Renewal cannot extend the transaction lifetime or
revive an expired lease. Both read-write transactions and read-only snapshots
use the same ownership rules. Serialize expiry cleanup with transaction
operations and wait for an admitted commit's actual result before reclaiming
its state.

The data-frame layout is unchanged. Sources without negotiation retain the
legacy protocol. Current loregd requires the extension and must be deployed
with a kernel that implements it. The PKM TRM, §5.8.3, specifies the control
ioctls and their error and lifetime rules.

## After registration

The returned fd *is* your serve endpoint — you `read(2)` requests and
`write(2)` responses on it directly (through the
[request](~peios/sdk-rsi-request/rsi-request-h-decoding-requests) and
[response](~peios/sdk-rsi-response/rsi-response-h-building-responses)
helpers). Closing it deregisters the source and signals EOF to the kernel.
Keep it open for the life of the source, and move on to the [serve
loop](~peios/registry-sources/serving-requests).

## Next

- **[Serving requests](~peios/registry-sources/serving-requests)** — the loop
  that runs on the source fd.
- **[`rsi/source.h`
  reference](~peios/sdk-rsi-source/rsi-source-h-becoming-a-source)** — every
  field and error.
