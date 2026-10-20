---
title: Installing and Linking
type: how-to
description: Selecting SDK packages and linking a C application to libpeios or librsi.
---

Link your application to the library whose services it uses. Install the
matching development package when compiling, and the runtime package on
machines that run the resulting executable.

## Before you start

Prepare these build inputs:

- A C or C++ compiler for the target system.
- The development package for libpeios, librsi, or both.
- The Peios kernel userspace API headers required by those packages.
- `pkg-config` configured to find the target system's package metadata.

## Choosing packages

Each library has the same package split. Replace `libpeios` with `librsi`
for registry source development:

| Package | Contents | Needed for |
|---|---|---|
| `dev.peios.libpeios` | Versioned shared library | Running dynamically linked programs |
| `dev.peios.libpeios-devel` | Headers, linker symlink, and `peios.pc` | Compiling programs |
| `dev.peios.libpeios-static` | Static archive | Static linking |
| `dev.peios.libpeios-debuginfo` | Separate debugging information | Debugging library code |
| `dev.peios.libpeios-debugsource` | Source files for debugging | Stepping through library code |

Use headers and libraries from the same SDK package set. Do not combine
headers from an arbitrary kernel checkout with an unrelated library build.
The library headers include `<pkm/*.h>` definitions; the directory name is
part of the installed include interface.

## Linking an application

Use the complete `first.c` program from
[compiling a first program](~peios/sdk-basics/your-first-program):

1. Include `<peios.h>` or the individual `<peios/*.h>` headers that the
   application uses.
2. Obtain the compiler and linker options from the `peios` pkg-config
   module. Put the library options after the source files or object files.
3. Build the program for the same target as the installed library.

For librsi, use `<rsi.h>` and the `rsi` pkg-config module. Applications that
serve registry data and also access other Peios services can link both.

## Checking the result

A successful compile confirms that the headers are available. A successful
link confirms that the required symbols are present in the selected
library. Run the first-program example to check portable data handling.

Calls that access tokens, registry keys, or kernel events need a Peios
kernel with the required services. Test those calls on the target system
with the privileges and objects described by their guides.

## Static linking

The static package contains `libpeios.a` or `librsi.a`. Select that archive
explicitly when constructing the linker command. Static linking includes
the SDK code in the executable; the application's other dependencies still
follow their own linking rules.

Rebuild statically linked applications when adopting an SDK fix. Updating
the shared-library package alone does not replace code in those programs.

## Debugging static builds

The packaged static archives retain linker symbols but omit DWARF debugging
information. Each library's debug package describes its shared object; it
does not supply the missing SDK internals for a statically linked program.

To debug statically linked SDK code, use the unstripped `//libpeios:static`
or `//librsi:static` Bazel output, or rebuild the matching source with its
recorded compiler and dependency inputs. Keep those inputs with the binary
so that debugger source paths and layouts match the executable.
