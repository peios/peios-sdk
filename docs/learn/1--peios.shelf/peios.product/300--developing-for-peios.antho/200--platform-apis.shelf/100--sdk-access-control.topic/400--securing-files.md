---
title: Securing Files
type: how-to
description: Opening files with explicit access and reading or changing their security descriptors.
---

Use libpeios to open a file with explicit access or to read and change its
security descriptor. Select the descriptor components needed by the task
so that an update preserves unrelated security settings.

## Before you start

Prepare the required authority and inputs:

- A Peios kernel with the Kernel Access Control Subsystem (KACS).
- A process token with the rights needed for the requested operation.
- The libpeios headers and matching library.
- A validated security descriptor for operations that supply one.

## Opening a file

A native file open requests a specific access mask. The resulting handle
carries the access granted at open. Choose that mask deliberately before
passing the handle to another process.

1. Initialize `struct peios_open_params` to zero.
2. Set `desired_access` to the file rights the caller needs.
3. Choose the disposition that matches the operation. Use
   `KACS_DISPOSITION_CREATE` when an existing path must be an error, or
   `KACS_DISPOSITION_OPEN` when the file must already exist.
4. For creation, supply the creator descriptor when required by the
   application's security policy. For an existing file, leave `sd` as
   `NULL` and `sd_len` as zero.
5. Call `peios_file_open` and check its return value before using the
   descriptor or the status output.
6. Close the returned descriptor when its ownership ends.

If a create-only operation reports `EEXIST`, decide whether the application
can accept the existing object before opening it separately. A preceding
path-existence check cannot make creation and opening atomic.

See [opening a file](~peios/sdk-files/opening-a-file) for the parameter
contract and the kernel's supported create options.

## Reading a security descriptor

The `KACS_SECINFO_*` selection identifies which components to read. A
request for the owner and discretionary access control list (DACL) does
not also request the system access control list (SACL).

1. Choose `peios_fd_get_sd` when the application already owns a suitable
   handle. Otherwise, use `peios_file_get_sd` with the intended path policy.
2. Probe the required length with a zero-capacity buffer and check for
   failure before converting the result to `size_t`.
3. Allocate the buffer and check that allocation succeeded.
4. Repeat the query. If the descriptor grows and the call reports `ERANGE`,
   probe again or stop according to the application's retry policy.
5. Parse only the bytes returned by a successful query.
6. Keep the buffer unchanged while using parsed views, then release it.

A size probe does not reserve the descriptor or prevent another process
from changing it. Do not use the probe's length as though it were the
length returned by the second call.

## Changing selected components

Use `peios_sd_builder` to construct the descriptor that carries the
replacement components:

1. Create a builder and check that it is not `NULL`.
2. Add the replacement components, such as the DACL.
3. Obtain the encoded bytes and check for a retained builder error.
4. Call `peios_fd_set_sd` or `peios_file_set_sd` with the corresponding
   `KACS_SECINFO_*` selection.
5. Check the result before reporting success, then free the builder.

Selecting only `KACS_SECINFO_DACL` preserves the owner, group, and SACL.
Changing an owner or integrity label has additional permission checks;
possessing a syntactically valid descriptor does not satisfy those checks.

## Checking the result

Read the selected components again through the intended object handle and
compare them with the requested values. Distinguish a failed update from a
failed verification query, and preserve the relevant `errno` before cleanup.

The [file-security reference](~peios/sdk-files/reading-and-writing-a-file-s-security-descriptor)
describes each library entry point. The
[kernel file-security guide](~kernel/file-security/the-set-security-interface)
owns the kernel's access checks and descriptor merge rules.
