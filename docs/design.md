# SDK Implementation and Ownership

The SDK has two public C interfaces and one internal Rust support crate.
Repository consolidation coordinates changes to shared code without
combining the public libraries' ABI or package identities.

## Public interfaces

libpeios exposes the `peios_*` functions and mapping objects declared by
`<peios.h>` and its concept headers. It provides security data operations,
tokens, access checks, file and process security, registry clients, and
events. Its shared-library identity is `libpeios.so.0`.

librsi exposes the `rsi_*` functions declared by `<rsi.h>`. It registers a
registry source, decodes requests, and constructs responses. The backend
supplies storage and transaction semantics. Its shared-library identity is
`librsi.so.0`.

Both libraries also build static archives. Their supported application
boundary is the C ABI, including for callers implemented in Rust. Internal
Rust types are not an additional public SDK interface.

## Shared implementation

peios-cabi owns the common allocator bridge, errno handling, system-call
wrappers, buffer helpers, and builder support. It has no installed shared
object and no independent application API.

Kernel-owned semantic crates provide shared parsing and validation logic.
The SDK owns userspace construction and encoding where the kernel only
needs parsing, including security builders and registry source responses.
Sharing parser code reduces divergence; independent expected-result tests
are still needed because two users of the same parser can share a bug.

## Kernel source pin

`vendor/kernel/ORIGIN.json` records the source repository, exact commit,
selected paths, and file digests. The snapshot includes the userspace API
headers and the kernel's Rust semantic crates. Update those inputs from
the kernel repository as one reviewed change.

Do not edit semantic code in the snapshot as an SDK fix. Make the change
in its owning kernel project, then update the SDK pin and rerun interface
and behavior checks. Keep the kernel version and SDK release version
separate.

## Allocation and lifetime

The public interfaces use explicit buffers, opaque builders, and borrowed
views. Callers retain input buffers for the lifetime of borrowed data.
Library-created builders are destroyed through the matching library
function. Registry source decoders borrow their incoming frame; reusing
that frame invalidates the decoded views.

The no-standard-library production crates still allocate through the C
allocator where their interfaces need owned buffers. That arrangement does
not make unchecked pointers safe. Every exported function's pointer,
length, aliasing, and ownership requirements remain part of its contract.

## Licensing

Each owned member carries its license. Kernel-derived files retain their
source notices and the applicable kernel license texts. Packaging must
preserve those notices alongside the source and debugging payloads; the
repository's ownership boundary does not change upstream licensing.
