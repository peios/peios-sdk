---
title: Working with tokens
type: how-to
description: Answer "who am I?", "who is calling me?", and "act as someone else" with the KACS token API — opening tokens, peer identity over sockets, impersonation, and dropping privilege.
related:
  - peios/sdk-tokens/token-h-tokens-and-sessions
  - peios/sdk-access-control/checking-access
  - peios/impersonation/overview
  - peios/job-submission/scope-and-roles
---

A **token** is the runtime carrier of an identity, and it is a file descriptor. This guide covers the everyday token tasks; [`token.h`](~peios/sdk-tokens/token-h-tokens-and-sessions) is the exhaustive reference for every call and field.

## Who am I?

To inspect your own identity, open your effective token and query it:

```c
int tok = peios_token_open_self(0, KACS_TOKEN_QUERY);
if (tok < 0) { /* errno */ }

unsigned char sid[PEIOS_SID_MAX_BYTES];
ssize_t n = peios_token_user(tok, sid, sizeof sid);   /* the user SID */

uint32_t il;
peios_token_integrity(tok, &il);                      /* integrity level RID */

struct peios_privilege_set privs;
peios_token_privileges(tok, &privs);                  /* held/enabled privileges */

close(tok);
```

`peios_token_open_self` gives you the *effective* token — if your thread is impersonating, that's the impersonated identity. Pass `KACS_TOKEN_OPEN_REAL` in the flags to get your process's real primary token regardless. The `access` argument is the handle rights you want; `KACS_TOKEN_QUERY` is enough to read.

Group SIDs and other list-valued classes come back as buffers you parse with the [`security.h` views](~peios/sdk-security/parsing-views#sid-and-attributes-arrays): read `CLASS_GROUPS` with [`peios_token_query`](~peios/sdk-tokens/query), then `peios_sid_array_parse` it.

## Who is calling me?

The most useful token trick in a service is learning the identity of whoever connected to your socket. When a client connects over a Unix stream or seqpacket socket, KACS captures their token at `connect()` time, and you open it from the accepted connection:

```c
int conn = accept(listener, NULL, NULL);
int caller = peios_token_open_peer(conn);   /* QUERY | IMPERSONATE rights */
if (caller < 0) { /* errno */ }

/* Now query the caller's identity, or impersonate them (below). */
```

This is local authentication with no passwords and no handshake — the kernel vouches for who is on the other end. The handle comes with fixed `QUERY | IMPERSONATE | DUPLICATE` rights — enough to inspect the caller, act as them, and (for a process launched on their behalf) turn the identity into a primary token, but never to adjust it.

It works in the other direction too: right after `connect()`, `peios_token_open_peer(sock)` on the *client's* socket returns the identity of whoever is listening (at Identification level — enough to check, not to act as), so a client can verify it reached the genuine service and not something else that bound the path.

## Acting as the caller

Once you hold a caller's (impersonation) token, you can **impersonate** them: adopt their identity on your current thread so every subsequent access check runs as *them*, not as your service. This is how you do work on a client's behalf without running privileged and re-implementing their permissions.

```c
if (peios_token_impersonate(caller) != 0) { /* errno */ }

/* ... do the work here: file opens, access checks, etc. all run as the caller ... */

peios_token_revert();   /* back to your own identity */
close(caller);
```

Always pair `peios_token_impersonate` with [`peios_token_revert`](~peios/sdk-tokens/adjust-and-transform#impersonation-and-installation), ideally in the cleanup path, so a failure partway through can't leave your thread wearing someone else's identity. `peios_token_revert` is a safe no-op if you weren't impersonating.

The full flow for a request handler is: `accept` → `peios_token_open_peer` → `peios_token_impersonate` → serve the request → `peios_token_revert` → `close`. When the handler runs start to finish on one thread and has no other use for the token, `peios_token_impersonate_peer(conn)` collapses the open, impersonate and close into one call.

A client decides how far its identity travels: `peios_socket_set_impersonation_level(sock, KACS_IMLEVEL_IDENTIFICATION)` lets the server learn who it is without being able to act as it. And a client that writes on behalf of many identities over one connection — a service impersonating its callers through a connection pool — turns on `peios_socket_set_pass_token(sock, true)`, after which every send carries whoever is impersonating at that moment and the server's `peios_token_open_peer` tracks it request by request.

## Handing an identity to a manager

Impersonation acts as someone on *your* thread. The other shape is handing an identity to another process so *it* can act — the pattern behind the Peinit jobs socket ([PSPU §7](~peios/job-submission/scope-and-roles)): a submitter such as backupd holds a token for the user whose data it is backing up, and wants Peinit to run the backup job *as that user*, not as backupd. Nothing is impersonated; the token itself is attached to the request, and the kernel vouches that the sender could have acted as it.

```c
int jobs = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0);
connect(jobs, ...);                                   /* who may submit is who may connect */

const char *req = "{\"command\":\"submit\",\"image_path\":\"/usr/bin/backup\"}";
if (peios_socket_send_message(jobs, req, strlen(req), user_token, NULL, 0, 0) < 0) {
    /* EACCES: user_token lacks TOKEN_IMPERSONATE; EPERM: the attach would lower its level */
}
```

Three things make this safe, and none of them is the manager's to get wrong:

- **The kernel gates the attach like an impersonation.** You need `TOKEN_IMPERSONATE` on the handle, and the token arrives at the *sending socket's* impersonation level — so a token the sender could only identify, never act as, cannot be smuggled through. Peinit refuses anything below Impersonation. For a job that must reach remote systems as the user, set the socket to Delegation first: `peios_socket_set_impersonation_level(jobs, KACS_IMLEVEL_DELEGATION)` — and the attach then only succeeds if the token itself is at Delegation, since the level only ratchets down.
- **The receiver owns what it is handed.** `peios_socket_recv_message` gives the manager a token handle with `QUERY | IMPERSONATE | DUPLICATE` — enough to inspect the identity, check its level with `peios_token_impersonation_level`, and duplicate it into a primary token for the new process, but never to adjust it.
- **No token means the sender's own.** A request that attaches nothing is run as the connecting *process's* primary token: the manager opens it through `peios_socket_peer_pidfd` and `peios_token_open_process` — the kernel's handle on the peer, never a PID the peer named — which is `fork`-like and needs no privilege on the submitter's part.

Descriptors travel the same way: pass a socket for the job to serve on, or a pipe to receive its output, as `SCM_RIGHTS` on the same message. The full protocol — what the manager does with them and how the job is managed afterwards — is [PSPU §7](~peios/job-submission/scope-and-roles); the calls are in the [reference](~peios/sdk-tokens/opening-and-creating-tokens#per-message-identity-and-descriptors).

From Rust the same pattern is `peios::socket::send_message` / `recv_message`, with the received `Token` and `OwnedFd`s owned by the `ReceivedMessage`, and `peios::socket::peer_pidfd` for the fallback:

```rust
use std::os::fd::AsFd;
use peios::socket;
use peios::token::{ImpersonationLevel, Token, TokenAccess};

socket::send_message(jobs.as_fd(), request, Some(user_token.as_fd()), &[], 0)?;

// The manager's side. Every descriptor that arrived is owned by `msg`.
let msg = socket::recv_message(conn.as_fd(), &mut buf, 8, 0)?;
if msg.truncated || msg.control_truncated {
    return Err(refused());
}
let identity = match msg.token {
    Some(token) if token.impersonation_level()? >= ImpersonationLevel::Impersonation => token,
    Some(_) => return Err(refused()),   // below Impersonation
    None => {
        let pidfd = socket::peer_pidfd(conn.as_fd())?;
        Token::open_process(pidfd.as_fd(), TokenAccess::QUERY | TokenAccess::DUPLICATE)?
    }
};
```

## Dropping power

To run less-trusted code with less authority than you hold, derive a **restricted** token and hand it over. `peios_token_restrict` can delete privileges, demote groups to deny-only, and add restricting SIDs:

```c
struct peios_token_restrict spec = {
    .privs_to_delete = KACS_SE_DEBUG_PRIVILEGE | KACS_SE_IMPERSONATE_PRIVILEGE,
    .flags           = KACS_TOKEN_RESTRICT_WRITE_RESTRICTED,
};
int weak = peios_token_restrict(my_primary, &spec);
```

The result is a strictly less-powerful token. Combined with [integrity levels](~peios/sdk-security/sids#integrity-levels) and confinement (both set when [minting a token](~peios/sdk-tokens/the-token-spec-builder)), this is the basis of sandboxing on Peios.

## Minting tokens

Creating a token from scratch requires `SeCreateTokenPrivilege` and is the province of authentication authorities, not ordinary programs. When you do need it, the [token-spec builder](~peios/sdk-tokens/the-token-spec-builder) is the ergonomic path — typed setters for the user SID, groups, privileges, integrity, claims, and the rest, then `peios_token_builder_create`. Mind the [index convention](~peios/sdk-tokens/the-token-spec-builder#the-index-convention) for owner/primary-group references, and don't add the logon SID yourself — the kernel injects it.

## Next

- **[Checking access](~peios/sdk-access-control/checking-access)** — use a token to make an authorisation decision.
- **[`token.h` reference](~peios/sdk-tokens/token-h-tokens-and-sessions)** — every token call in full.
- **[Impersonation](~peios/impersonation/overview)** — the operator-side model and its two gates.
