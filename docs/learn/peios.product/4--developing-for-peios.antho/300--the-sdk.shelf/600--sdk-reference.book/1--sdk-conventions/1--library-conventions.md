---
title: Library Conventions
description: Understanding results, ownership, descriptors, and allocation across both C libraries.
---

libpeios and librsi share an error model and explicit ownership rules.
Check each result before using output parameters, and follow the installed
header for the function's particular buffer contract.

## Results and errors

Most `int` functions return zero or a new file descriptor on success and
`-1` with `errno` on failure. Length-returning functions use `ssize_t`.
Predicate functions and void builder setters have different contracts;
read their declarations before choosing an error check.

Read `errno` only after a function reports failure. Save it before cleanup
calls if the caller needs to report the original error.

## Buffers and builders

Many libpeios encoders support a zero-capacity size probe. Allocate the
reported space and repeat the call. String lengths exclude the terminating
NUL, so string buffers need one additional byte.

Registry reads use lengths stored in parameter structures and report a
short buffer through `ERANGE`. They do not use the same return convention
as security encoders. Recheck sizes when retrying reads of mutable data.

Builders retain their first error until it is inspected or the result is
requested. Free each builder with its matching library destructor. Do not
pass a library-owned builder to `free()`.

## Borrowed data

Parsed views borrow the caller's input bytes. Keep those bytes alive and
unchanged until every view and derived pointer is unused. Copy
borrowed data before retaining it beyond the input buffer's lifetime.

Registry source request decoders follow the same rule. Reading the next
request into a frame buffer invalidates views into the previous frame.

## File descriptors

Close owned descriptors when their use ends. Passing a descriptor to an
operation does not transfer ownership unless the header states otherwise.
A successful operation does not grant ownership of a descriptor merely
mentioned in an output structure; use that function's documented contract.

Start with [result
conventions](~peios/sdk-conventions/how-results-are-returned)
and [memory ownership](~peios/sdk-conventions/memory-ownership) for examples.
