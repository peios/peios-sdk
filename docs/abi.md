# Maintaining the C ABI

The handwritten headers in `libpeios/include/` and `librsi/include/` are the
installed interfaces. cbindgen snapshots in each member's `abi/` directory
check that those headers agree with the Rust exports. Applications include
`<peios.h>` or `<rsi.h>`; the generated snapshots are not installed headers.

## Compatibility

The libpeios compatibility baseline starts at version 0.5.0. Compatible
additions retain `libpeios.so.0`. Incompatible changes require a deliberate
SONAME change. librsi retains its own `librsi.so.0` interface boundary.

The consolidated SDK release version is shared by all owned components.
A shared version does not make the two libraries interchangeable and does
not merge their symbol sets. Do not infer a new ABI baseline from a source
layout change or a packaging change.

## Reviewing a change

Review the public interface as part of every exported Rust change:

1. Update the corresponding handwritten header, including ownership,
   return values, error conditions, and output-buffer behavior.
2. Regenerate the member's snapshot with pinned cbindgen 0.29.2 and its
   `cbindgen.toml` configuration.
3. Review the snapshot diff for signatures, data symbols, struct sizes,
   alignment, field offsets, and symbol additions or removals.
4. Run the ABI checks against both the headers and the built shared object.
5. Update the application guidance and compatibility tests for any changed
   behavior.

The verifier compares function signatures and data symbols, compiles public
and snapshot headers as C and C++, checks struct layouts, and compares
exports against the built library. Generator freshness is a separate gate:
checking two committed headers alone cannot prove they reflect Rust source.

## Running the checks

Use `//libpeios:abi-test` and `//librsi:abi-test` through Bazel. For a focused
check against explicit outputs, `tools/verify-abi.py` takes a member name
and required `--shared` and `--static` paths. It never builds an unrequested
Cargo artifact as a fallback.

The smoke programs compile and run as C and C++ against each linkage form.
Keep this executable check alongside header comparison: matching source
signatures alone cannot prove that the produced library links correctly.

## Kernel inputs

`vendor/kernel/ORIGIN.json` identifies the kernel snapshot supplying the
userspace API and shared semantic crates. Changes to that input need both
ABI checks and behavior tests. A source pin does not prove runtime
compatibility with an installed kernel.

Portable codec tests exercise userspace behavior. Native execution of a
binary is not proof of successful Peios system calls on an unrelated host.
Qualify kernel-facing operations on the intended kernel and retain that
kernel's identity with the results.
