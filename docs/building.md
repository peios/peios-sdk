# Building and Testing

Run repository commands from the Peios SDK root. Bazel builds the two C
libraries, runs tests, and assembles their packages. Cargo remains useful
for focused Rust development; it does not assemble the package store.

## Development builds

Use Linux on x86-64 with Python 3.11 or later, GCC, G++, binutils, and a
working certificate store. The `bazel` launcher selects the version in
`.bazelversion`; Bazel pins the Rust toolchain and external dependencies.
Initial dependency acquisition needs network access.

Build the shared libraries and static archives:

```sh
./bazel build //:artifacts
```

Build one member with `//libpeios:shared`, `//libpeios:static`,
`//librsi:shared`, or `//librsi:static`. Use `bazel cquery` when resolving
configured output paths instead of assuming Cargo's directory layout.

Run the development test suite:

```sh
./bazel test //...
```

The member test targets are `//libpeios:tests` and `//librsi:tests`.
The `//libpeios:abi-test` and `//librsi:abi-test` targets check the exported
interfaces. ABI verification requires cbindgen 0.29.2, GCC, G++, `nm`, and
`readelf`; an absent generator is an error, not a skipped freshness check.
GCC remains the C prototype checker. To use another C++ compiler explicitly,
pass `--test_env=CXX=/path/to/clang++` to Bazel; the ABI checks report its identity.

## Style and repository checks

The repository's complete check entry point is:

```sh
tools/check
```

Use the pinned tools prepared by `tools/prepare-style-tools.py` for source
formatting. The PSC baseline lives in Panorama; the SDK's checked-in tool
configuration makes its mechanical rules reproducible.

For a focused Rust iteration, the workspace includes libpeios, librsi, and
peios-cabi. Keep `Cargo.lock` in sync with deliberate dependency changes.
Do not modify the kernel-owned code under `vendor/kernel/` to satisfy an
SDK style check.

## Package assembly

Build the content-addressed package store:

```sh
./bazel build //:packages
```

Each public library retains its runtime, development, static, debugging,
and debug-source package identities. `dev.peios.peios-sdk-source` contains
the consolidated source. `peios-cabi` is internal code linked into the
public libraries; it is not a third public shared library.

Package manifests and blobs are assembled by the pinned canonical writer.
Package validation and native hardening checks are separate from ordinary
source tests. Development artifacts are not signed release artifacts.

## Static consumers

The Bazel static targets namespace private implementation and Rust runtime
symbols. This lets a Rust executable link the C ABI archive without conflicting
with its own allocator or panic runtime; it also lets both SDK archives coexist.
Public `peios_*` and `rsi_*` names and undefined system-library imports retain
their names. The shared libraries are unchanged.

Use the completed `//libpeios:static` and `//librsi:static` outputs for consumers.
Cargo's compiler output is an intermediate archive; if using that development
path, finish it explicitly before linking it into a Rust application:

```sh
python3 tools/isolate-static.py target/release/libpeios.a /tmp/libpeios.a --prefix peios
```

The `//:static-runtime-test` check links both archives into Rust executables
with unwind and abort panic strategies and exercises userspace parsing.
It does not invoke Peios kernel operations.

## Static debugging

Packaged static archives have DWARF information removed while retaining
linker symbols. Per-library debug packages describe the shared objects and
cannot restore SDK debug information in a statically linked executable.
Use the unstripped `//libpeios:static` or `//librsi:static` Bazel output for
static debugging, or rebuild the matching source and compiler inputs.

## Native builds and offline inputs

The `peios` Bazel configuration selects the installed Peios Rust compiler
and hardened standard library. Set `PEIOS_RUSTC` when that compiler is not
`/usr/bin/rustc`. Use a compatible Peios userspace, compiler runtime, and
linker; setting `--config=peios` on an unrelated host is not a substitute.

`tools/offline-build` separates acquisition from use of a frozen dependency
snapshot. Its operations are:

| Operation | Inputs | Effect |
|---|---|---|
| `acquire` | New absolute snapshot directory | Acquires host and native-selected dependencies, freezes repositories, and records the module lock |
| `build` | Snapshot and private absolute work directory | Builds native packages and targets with downloads forbidden |
| `test` | Snapshot and private absolute work directory | Runs native package and test targets with downloads forbidden |

Acquire outside the isolated native environment. Run `build` or `test`
inside the prepared native environment with the snapshot available. The
helper rejects a module lock that differs from the recorded snapshot.
It does not create a virtual machine or install packages into a live system.

For reproducibility checks, build the same frozen source in separate
output roots. Compare the unstripped shared libraries and archives as well
as package payloads. Preserve compiler, dependency, and source identities
with the results.

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
tools/check-docs --context ../panorama/docs
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
