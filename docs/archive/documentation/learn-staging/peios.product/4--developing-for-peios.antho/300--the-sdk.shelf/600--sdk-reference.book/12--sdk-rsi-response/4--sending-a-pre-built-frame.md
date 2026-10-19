---
title: Sending a pre-built frame
description: Writing an already-built response frame to the source fd, for a source that encodes its own.
---

```c
ssize_t rsi_write_response(int fd, const void *frame, size_t len);
```

Writes one already-built response frame to the source fd — a thin `write(2)` wrapper returning the bytes written, or `-1` with `errno`. Most callers never need this; the `rsi_respond_*` helpers build *and* send. It exists for callers assembling frames by other means.
