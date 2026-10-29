# Building and Testing

Use pekit to build the SDK's two C libraries, libpeios and librsi, and to
run their checks. Pekit builds each crate on its own, so a change rebuilds
only the crates it reaches. The supported development host is x86-64
Linux. Cargo remains useful for focused Rust development.

## Before you start

You need pekit and Git. Everything else is pinned and fetched by pekit on
the first build: the Bootlin C toolchain, Rust 1.98.1, Python 3.14,
cbindgen 0.29.2, and the crates `Cargo.lock` selects. The first build needs
network access; later builds find what they need in pekit's store, and
every step runs without network access in a sandbox that holds only its
declared inputs.

The build files use the gcslib libraries named in `workspace.gcsl` and the
build files. `pekit lock` pins their commits in `pekit.lock`.

## Build

```sh
pekit build libpeios libpeios_static libpeios_static_musl libpeios_include librsi librsi_static librsi_include
```

| Target | Output |
|---|---|
| `libpeios` | libpeios, `libpeios.so.0` by its shared object name (SONAME) |
| `libpeios_static` | libpeios's static archive, its private symbols renamed |
| `libpeios_static_musl` | libpeios's static archive for `x86_64-unknown-linux-musl`, its private symbols renamed |
| `libpeios_include` | libpeios's headers, with the kernel's `pkm` headers they include |
| `librsi` | librsi, `librsi.so.0` by its SONAME |
| `librsi_static` | librsi's static archive, its private symbols renamed |
| `librsi_include` | librsi's headers, with the kernel's `pkm` headers they include |

Pekit prints where each output is, under `pekit-out/<target>/`. Each
library is linked with full RELRO and needs only the C library. One compile
makes both a library's shared object and its archive, as Cargo's does.

## Test

```sh
pekit test
```

| Test | Check |
|---|---|
| `libpeios/unit`, `librsi/unit`, `peios-cabi/unit` | Each member's unit tests |
| `libpeios/abi`, `librsi/abi` | The library's ABI, as `tools/verify-abi.py` checks it |
| `tools/static_runtime` | Both archives linked into Rust programs, with unwinding and aborting |
| `tools/kernel_inputs` | The files under `vendor/kernel` are exactly those `ORIGIN.json` lists |
| `tools/verify_abi_tool` | `verify-abi.py`'s own unit tests |
| `tools/update_kernel_inputs_tool` | `update-kernel-inputs.py`'s own unit tests |

A test whose inputs did not change is found in the store and not run
again. `tools/update_kernel_inputs_tool` makes Git repositories, so it runs
on the host with the host's `git` and Python, every time.

The ABI checks build cbindgen 0.29.2 from its crates.io source and run with
the Bootlin toolchain's GCC, G++, `nm`, and `readelf`. See [ABI
maintenance](abi.md) for what they check.

## Style and repository checks

The repository's complete check entry point is:

```sh
tools/check
```

It checks formatting with the pinned tools prepared by
`tools/prepare-style-tools.py`, checks the vendored kernel files, builds
the libraries and runs the tests with pekit, and runs Cargo's lints and
documentation tests. The PSC baseline lives in Panorama; the SDK's
checked-in tool configuration makes its mechanical rules reproducible.

For a focused Rust iteration, the workspace includes libpeios, librsi, and
peios-cabi. Keep `Cargo.lock` in sync with deliberate dependency changes.
Do not modify the kernel-owned code under `vendor/kernel/` to satisfy an
SDK style check.

## Building against the SDK

The SDK is a GCSL library as well as a workspace. Another workspace builds
the SDK's libraries from the SDK's own files by importing its definitions:

```text
import build * from "github.com/peios/peios-sdk"

def build program() { ... build libpeios() ... build libpeios_include() ... }
```

`pekit lock` pins the SDK's commit in that workspace's `pekit.lock`, as it
pins any library. A program built with Rust binds libpeios through
peios-rs, whose library gives `peios-sys` the SDK's library and headers.

## Static consumers

The static targets namespace private implementation and Rust runtime
symbols, with `tools/isolate-static.py`. This lets a Rust executable link
the C ABI archive without conflicting with its own allocator or panic
runtime; it also lets both SDK archives coexist. Public `peios_*` and
`rsi_*` names and undefined system-library imports retain their names. The
shared libraries are unchanged.

Use the `libpeios_static` and `librsi_static` outputs for consumers, and
`libpeios_static_musl` for a static musl program.
Cargo's compiler output is an intermediate archive; if using that
development path, finish it explicitly before linking it into a Rust
application:

```sh
python3 tools/isolate-static.py target/release/libpeios.a /tmp/libpeios.a --prefix peios
```

The `tools/static_runtime` test links both archives into Rust executables
with unwind and abort panic strategies and exercises userspace parsing. It
does not invoke Peios kernel operations.

## Not built with pekit yet

Packages, builds with the native Peios toolchain, which hardens the
libraries with control-flow enforcement technology (CET), and musl archives
for early-boot consumers are not yet built with pekit. The package
definitions, the packaging pipeline, and the native and musl build settings
from the Bazel build are kept outside this repository until they are.

## Documentation checks

Install the pinned Trail version used by CI or set `TRAIL` to its executable.
Run the component check:

```sh
tools/check-docs
```

This runs the PSC lint rules and a strict build. The component tree refers
to kernel documentation that is supplied by Panorama. The exact expected
missing-product diagnostics are recorded in `docs/external-links.json`.
Any added, removed, or changed diagnostic fails the component check.

Run the publication-context check from the standard workspace layout:

```sh
tools/check-docs --context ../learn
```

The checker copies the context without generated output or caches,
resolves merge paths against the original root, and builds with strict
link checking. This is the gate that proves the external links resolve.
Updating the diagnostic baseline alone does not satisfy that gate.

## Qualification boundaries

Portable tests cover parsers, encoders, builders, ownership rules, and
mocked or in-memory transport behavior. ABI smoke programs check that C and
C++ can link and run against shared and static outputs.

These checks do not exercise successful Peios system calls on a development
host. Validate kernel-facing behavior on the intended Peios kernel and
record its identity separately. A package build, native userspace test,
reproducibility comparison, and kernel qualification establish different
properties; retain their separate results when preparing a release.
