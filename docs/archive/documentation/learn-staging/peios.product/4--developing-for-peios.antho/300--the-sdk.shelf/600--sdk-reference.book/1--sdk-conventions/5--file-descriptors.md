---
title: File descriptors
description: Every handle libpeios opens is a raw int file descriptor you close yourself, with the ordinary semantics that implies.
---

Handles that libpeios opens — tokens, registry keys, event streams — are **raw `int` file descriptors**, the same kind `open()` gives you. You close them with `close()`, poll them, and pass them across `exec` (or not) with the usual fd machinery.

They are created **`O_CLOEXEC` by default**: a handle does not leak across an `exec` unless you deliberately clear the flag with `fcntl`. This is the safe default for security-sensitive handles — a token or key fd will not silently end up in a child process you launch.
