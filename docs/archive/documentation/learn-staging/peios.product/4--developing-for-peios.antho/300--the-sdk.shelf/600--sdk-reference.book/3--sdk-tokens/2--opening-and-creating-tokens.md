---
title: Opening and creating tokens
description: The entry points that hand back a token fd — opening your own, another process's, or minting a new one.
---

Each of these returns a token fd (or `-1` with `errno`).

```c
int peios_token_open_self(unsigned flags, uint32_t access);
int peios_token_open_process(int pidfd, uint32_t access);
int peios_token_open_thread(int pidfd, int tid, uint32_t access);
int peios_token_open_peer(int conn_fd);
int peios_token_create_raw(const void *spec, size_t len);
```

| Function | Opens |
|---|---|
| `peios_token_open_self` | The calling thread's token. `flags` may be `KACS_TOKEN_OPEN_REAL` to get the **primary** token even while the thread is impersonating; otherwise you get the effective (impersonation-aware) token. `access` is the desired handle rights. |
| `peios_token_open_process` | The **primary** token of the process named by `pidfd`. Subject to a process-query access check and PIP dominance over the target. |
| `peios_token_open_thread` | Thread `tid`'s **impersonation** token if it is impersonating, else the process primary token. |
| `peios_token_open_peer` | The peer-identity token captured at `connect()` on a connected Unix stream/seqpacket socket `conn_fd` — how a server learns *who* is on the other end of a socket. Reads the `KACS_SO_PEER_TOKEN` socket option (level `SOL_KACS`, `<pkm/socket.h>`). The handle carries fixed `QUERY | IMPERSONATE | DUPLICATE` rights (no `access` argument). |
| `peios_token_create_raw` | Mints a token from a pre-built token-spec buffer. This is the escape hatch — **prefer the builder below**. Requires `SeCreateTokenPrivilege`. |

Errors, per call:

- **`peios_token_open_self`** — `EINVAL` (unknown `flags`; empty or unknown `access` bits), `EACCES` (the token's own SD denies `access`).
- **`peios_token_open_process`** — `EACCES` (any of the three checks failed — process-query right, PIP dominance, or the token SD; deliberately indistinguishable), `EBADF` (invalid pidfd), `ESRCH` (target exited), `EINVAL` (empty or unknown `access` bits).
- **`peios_token_open_thread`** — the `_open_process` set, plus `ESRCH` (thread exited, or not in `pidfd`'s process) and `EINVAL` (`tid <= 0`).
- **`peios_token_open_peer`** — `ENOTCONN` (not connected), `ENODATA` (connected but no captured identity — a socketpair end), `EOPNOTSUPP` (a datagram or non-Unix socket), `ENOTSOCK` (not a socket), `EBADF` (invalid fd).
- **`peios_token_create_raw`** — `EPERM` (privilege missing), `EINVAL` (spec failed kernel validation), `EFAULT` (bad spec pointer), `ENOMEM` (allocation failed).

`peios_token_open_peer` is the cornerstone of local authentication: accept a connection, open the peer token, and you have the caller's identity to query or impersonate — no password, no handshake, just the kernel's word for who connected.

## Peer identity on sockets

```c
int peios_token_impersonate_peer(int conn_fd);
int peios_socket_set_impersonation_level(int sock_fd, uint32_t level);
int peios_socket_get_impersonation_level(int sock_fd, uint32_t *level);
int peios_socket_set_pass_token(int sock_fd, bool on);
int peios_socket_restamp(int sock_fd);
```

- `peios_token_impersonate_peer` is `peios_token_open_peer` followed by `peios_token_impersonate` and a `close` — the one-call form for a handler that impersonates, works, and reverts on a single thread. It returns 0 or -1; the errno is whichever step failed.
- `peios_socket_set_impersonation_level` is the **client's** side of the exchange: it sets the `KACS_SO_IMPERSONATION_LEVEL` socket option (`KACS_IMLEVEL_*`), bounding the level at which the peer may capture this socket's identity — Identification to be known but not acted as, Anonymous to withhold identity entirely. Usually set before `connect()`; it may change at any time and bounds every capture from then on. The default is `KACS_IMLEVEL_IMPERSONATION`. Errors: `EINVAL` (unknown level), `EOPNOTSUPP` (a non-Unix socket).
- `peios_socket_get_impersonation_level` reads the level back into `*level`.
- `peios_socket_restamp` replaces the identity a **listening** socket conveys to connecting clients — captured when `listen()` was called — with the caller's own (`KACS_SO_RESTAMP`; `EINVAL` if the socket is not listening). Capture is symmetric: a client's `peios_token_open_peer` on its own socket right after `connect()` returns the listener's identity at Identification level, which is how a client verifies the service it reached. A service that receives listeners it did not create (from the descriptor store after a restart, or from a broker) restamps them before accepting.
- `peios_socket_set_pass_token` turns on per-message identity for a sending socket (`KACS_SO_PASS_TOKEN`): every send then carries the sender's effective identity as a `KACS_SCM_TOKEN` ancillary message, and the peer's `peios_token_open_peer` follows whoever was impersonating at each send. This is what makes a pooled or multiplexed connection convey the right identity per request without the client library knowing. To attach a *specific* token to one message instead — a router forwarding for many clients, or a submitter handing a job manager the identity a job should run as — use `peios_socket_send_message` below.

## Per-message identity and descriptors

A message on a Unix socket can carry two things beside its bytes: a token the kernel attests the sender could act as (`KACS_SCM_TOKEN`, level `SOL_KACS`) and a set of file descriptors (`SCM_RIGHTS`). Building the control buffer by hand and walking it back is the same twenty lines in every program that does it, and a wrong `CMSG_ALIGN` is silent; these calls do it once.

```c
struct peios_socket_message {
    int      token_fd;   /* out: attached token (QUERY|IMPERSONATE|DUPLICATE, O_CLOEXEC), or -1 */
    int     *fds;        /* in:  SCM_RIGHTS descriptors land here, each O_CLOEXEC */
    unsigned fd_cap;     /* in:  capacity of fds */
    unsigned fd_count;   /* out: how many were filled */
    unsigned flags;      /* out: PEIOS_SOCKET_MSG_* */
};
#define PEIOS_SOCKET_MSG_TRUNCATED  0x1u   /* data did not fit (MSG_TRUNC) */
#define PEIOS_SOCKET_MSG_CTRUNCATED 0x2u   /* ancillary data did not fit (MSG_CTRUNC) */

ssize_t peios_socket_send_message(int sock_fd, const void *buf, size_t len,
                                  int token_fd, const int *fds, unsigned fd_count,
                                  int flags);
ssize_t peios_socket_recv_message(int sock_fd, void *buf, size_t cap,
                                  struct peios_socket_message *msg, int flags);
int     peios_socket_peer_pidfd(int sock_fd);
```

- `peios_socket_send_message` sends `len` bytes from `buf` on `sock_fd` with one `sendmsg(2)`, attaching `token_fd` as a `KACS_SCM_TOKEN` when it is `>= 0` and `fds[0..fd_count]` as `SCM_RIGHTS` when `fd_count > 0`; `flags` are `sendmsg` flags. Returns the bytes sent. **The kernel gates the token attach exactly as it gates impersonating it** — the handle needs `TOKEN_IMPERSONATE`, and the token travels at the *sending socket's* impersonation level (`peios_socket_set_impersonation_level`, default Impersonation), which can only lower what the token already carries: if attaching would have to lower the token's level the send fails rather than downgrading. Ownership is unchanged by a send — the caller keeps `token_fd` and every entry of `fds`; the kernel duplicates them into the receiver on delivery.
- `peios_socket_recv_message` receives one message with `recvmsg(2)` into `buf`/`cap`, with room for **one** attached token and `msg->fd_cap` descriptors written into the caller-owned `msg->fds` (pass `NULL` with `fd_cap == 0` to accept none). `MSG_CMSG_CLOEXEC` is always added to `flags`, so every descriptor delivered is `O_CLOEXEC`. Returns the bytes received (`0` at end of stream). On return the caller **owns** `msg->token_fd` (when `>= 0`) and `msg->fds[0..fd_count]` and closes each; the token handle carries `QUERY | IMPERSONATE | DUPLICATE`, the same fixed rights as `peios_token_open_peer`. Nothing leaks on any path: a second token in one message, or a descriptor past `fd_cap`, is closed by the library before it returns, and `msg->flags` reports the loss as `PEIOS_SOCKET_MSG_CTRUNCATED`; `PEIOS_SOCKET_MSG_TRUNCATED` means the bytes did not fit `cap` (on a `SOCK_SEQPACKET` socket the tail is gone). A receiver that cannot accept a truncated message closes what it was handed. On `-1` nothing was consumed and `*msg` is untouched. `fd_cap` above the kernel's `SCM_MAX_FD` (253) is capped silently.
  The truncation flag also covers handles that fit in the syscall's ancillary
  buffer but exceed the caller's capacity. For example, `fd_cap == 0` closes
  any received rights and reports control truncation. This flag is retained
  alongside payload truncation when both occur.
- `peios_socket_peer_pidfd` returns a new pidfd (`O_CLOEXEC`) for the process on the other end of a connected Unix socket — `getsockopt(SOL_SOCKET, SO_PEERPIDFD)`, a Linux ≥ 6.5 facility that libpeios only wraps; the option itself comes from the kernel UAPI and glibc. It is the kernel's handle on the peer, which is what `peios_token_open_process` takes to open the peer's **primary** token — the identity a process *has*, as distinct from the identity it *conveyed* at `connect()` (`peios_token_open_peer`) — and never a PID the peer named, so a recycled PID can never be opened by mistake.

Errors: `peios_socket_send_message` — `EINVAL` (`NULL` buffer with `len > 0`, `NULL` `fds` with `fd_count > 0`, or `fd_count` above `SCM_MAX_FD`), `EACCES` (`token_fd` lacks `TOKEN_IMPERSONATE`), `EPERM` (attaching would lower the token's level), `EBADF` (a descriptor in `fds` is not open), plus the `sendmsg` set (`ENOTCONN`, `EPIPE`, `EMSGSIZE`, `EAGAIN`). `peios_socket_recv_message` — `EINVAL` (`NULL` `msg`), plus the `recvmsg` set (`ENOTCONN`, `EAGAIN`, `EBADF`). `peios_socket_peer_pidfd` — `ENODATA` (no connected peer to name), `ENOPROTOOPT` (a kernel without `SO_PEERPIDFD`), `ENOTSOCK`, `EBADF`, `EMFILE`.

The client side of a job submission (PSPU §7) is the archetype — one record, one identity, two descriptors:

```c
const char *request = "{\"command\":\"submit\",\"image_path\":\"/usr/bin/backup\","
                      "\"descriptors\":[\"control\"],\"output\":true}";
int fds[2] = { control_sock, STDOUT_FILENO };   /* named descriptors first, output sink last */

/* `job_token` is a token I hold with TOKEN_IMPERSONATE; the job runs as it. */
if (peios_socket_send_message(jobs_sock, request, strlen(request),
                              job_token, fds, 2, 0) < 0) { /* errno */ }
/* I still own job_token, control_sock and my stdout. */
```

and the manager's side, receiving it:

```c
char buf[64 * 1024];
int fds[8];
struct peios_socket_message msg = { .fds = fds, .fd_cap = 8 };

ssize_t n = peios_socket_recv_message(conn, buf, sizeof buf, &msg, 0);
if (n < 0) { /* errno; nothing consumed */ }
if (n == 0) { /* end of stream */ }
if (msg.flags & (PEIOS_SOCKET_MSG_TRUNCATED | PEIOS_SOCKET_MSG_CTRUNCATED)) {
    /* not the message the sender meant: refuse it and close what arrived */
}
if (msg.token_fd >= 0) {
    /* the sender attested to this identity, at its socket's level */
    uint32_t level;
    peios_token_impersonation_level(msg.token_fd, &level);   /* e.g. refuse below Impersonation */
} else {
    /* no identity conveyed: fall back to the peer process's own primary token */
    int pidfd = peios_socket_peer_pidfd(conn);
    int primary = peios_token_open_process(pidfd, KACS_TOKEN_QUERY | KACS_TOKEN_DUPLICATE);
    close(pidfd);
}
/* msg.fds[0..msg.fd_count] and msg.token_fd are mine to use and close */
```
