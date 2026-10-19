---
title: Status codes
description: The statuses a response may carry and the errno each becomes for the registry client, so the right one matters.
---

Every response carries exactly one of these statuses. The kernel translates a non-OK status into the errno the registry client sees, so send the code that matches what actually happened:

| Code | When to send it |
|---|---|
| `RSI_OK` | The operation succeeded. Status-only ops report it via `rsi_respond_status`; the five payload-bearing ops must use their own helper. |
| `RSI_NOT_FOUND` | The requested key, entry, value, or layer does not exist in your store (client sees `ENOENT`). |
| `RSI_ALREADY_EXISTS` | A create collided with something that already exists (client sees `EEXIST`). |
| `RSI_STORAGE_ERROR` | Your backing store failed — I/O error, corruption, anything the client can't fix (client sees `EIO`). |
| `RSI_NOT_EMPTY` | The operation needs the key to have no children, and it has some (client sees `ENOTEMPTY`). |
| `RSI_TOO_LARGE` | The data exceeds what the source is willing or able to store (client sees `ENOSPC`). |
| `RSI_TXN_BUSY` | A transaction can't proceed right now — e.g. write-lock contention; the operation may be retried (client sees `EBUSY`). |
| `RSI_INVALID` | The request is well-formed RSI but violates the source's rules or refers to something malformed (client sees `EINVAL`). |
| `RSI_CAS_FAILED` | A sequence-guarded write's `expected_sequence` did not match the current entry — the compare-and-swap lost (client sees `EAGAIN` and retries). |
| `RSI_TXN_NOT_SUPPORTED` | Reply to `BEGIN_TRANSACTION` from a source that does not implement transactions (client sees `ENOTSUP`). |
