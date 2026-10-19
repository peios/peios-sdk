#!/usr/bin/env python3
"""Verify SDK source, shipped headers and explicit shared/static build outputs.

Host requirements: Python 3.11+, cbindgen 0.29.2, GCC, a C++ compiler, nm and readelf.
CXX selects an explicit C++ compiler executable; the default is g++.
No check is optional. Kernel headers come only from the committed source pin.
"""

from __future__ import annotations

import argparse
import difflib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

CBINDGEN_VERSION = "cbindgen 0.29.2"
CXX = os.environ.get("CXX", "g++")
ROOT = Path(os.path.abspath(__file__)).parent.parent


@dataclass(frozen=True)
class Library:
    member: str
    prefix: str
    shared: Path
    static: Path

    @property
    def root(self) -> Path:
        return ROOT / self.member

    @property
    def snapshot(self) -> Path:
        return self.root / "abi" / f"{self.prefix}-abi.h"

    @property
    def headers(self) -> list[Path]:
        return sorted((self.root / "include").rglob("*.h"))


class VerificationError(Exception):
    """A required ABI property failed."""


def run(command: list[str], *, env: dict[str, str] | None = None) -> str:
    result = subprocess.run(command, text=True, capture_output=True, check=False, env=env)
    if result.returncode:
        raise VerificationError(
            f"command failed ({result.returncode}): {command!r}\n{result.stdout}{result.stderr}"
        )
    return result.stdout


def equal(actual: str, expected: str, description: str) -> None:
    if actual != expected:
        difference = "".join(
            difflib.unified_diff(
                actual.splitlines(keepends=True),
                expected.splitlines(keepends=True),
                fromfile="shipping",
                tofile="source snapshot",
            )
        )
        raise VerificationError(f"{description}:\n{difference}")


def lines(values: list[str] | set[str]) -> str:
    return "".join(f"{value}\n" for value in sorted(set(values)))


def check_tools() -> None:
    for tool in ("cbindgen", "gcc", CXX, "nm", "readelf"):
        if shutil.which(tool) is None:
            raise VerificationError(f"required host tool missing: {tool}")
    cpp_version = run([CXX, "--version"]).splitlines()
    if not cpp_version:
        raise VerificationError(f"selected C++ compiler reported no version: {CXX}")
    print(f"C++ compiler: {CXX}: {cpp_version[0]}")
    found = run(["cbindgen", "--version"]).strip()
    if found != CBINDGEN_VERSION:
        raise VerificationError(f"required {CBINDGEN_VERSION}; found {found}")


def check_snapshot(library: Library, temporary: Path) -> None:
    generated = temporary / "generated.h"
    run(
        [
            "cbindgen",
            "--config",
            str(library.root / "cbindgen.toml"),
            "--lang",
            "c",
            "-o",
            str(generated),
            str(library.root / "src/lib.rs"),
        ]
    )
    equal(library.snapshot.read_text(), generated.read_text(), "stale Rust ABI snapshot")
    print("ok: exact cbindgen source snapshot")


def translation_unit(header: Path) -> str:
    return f'#include "{header}"\n'


def check_headers(library: Library, includes: list[str]) -> None:
    for compiler, language in (("gcc", "c"), (CXX, "c++")):
        for header in [library.snapshot, *library.headers]:
            run([compiler, *includes, "-Werror", "-fsyntax-only", "-x", language, str(header)])
    print(f"ok: {len(library.headers)} shipping headers and snapshot compile in C and C++")


def normalize_function(prototype: str) -> str:
    prototype = re.sub(r"^/\*.*?\*/\s*", "", prototype)
    prototype = re.sub(r"^extern\s+", "", prototype)
    prototype = re.sub(r"\bstruct\s+", "", prototype)
    prototype = re.sub(r"\benum\s+\w+", "int", prototype)
    prototype = re.sub(r"\bptrdiff_t\b", "ssize_t", prototype)
    prototype = re.sub(r"\buintptr_t\b", "size_t", prototype)
    return re.sub(r"\s+", " ", prototype).replace(" ;", ";").strip()


def functions(source: Path, prefix: str, includes: list[str]) -> set[str]:
    auxiliary = source.with_suffix(".aux")
    run(["gcc", *includes, "-aux-info", str(auxiliary), "-c", "-o", os.devnull, str(source)])
    pattern = rf"\b{prefix}_[a-z_]+\s*\("
    return {
        normalize_function(line)
        for line in auxiliary.read_text().splitlines()
        if re.search(pattern, line)
    }


def without_comments(source: str) -> str:
    return re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.DOTALL)


def structures(source: str, prefix: str) -> dict[str, list[str]]:
    result = {}
    for name, body in re.findall(
        rf"\bstruct\s+({prefix}_\w+)\s*\{{([^}}]*)\}}", without_comments(source), re.DOTALL
    ):
        fields = []
        for declaration in body.split(";"):
            declaration = declaration.strip()
            if not declaration:
                continue
            pointer = re.search(r"\(\s*\*\s*(\w+)\s*\)", declaration)
            field = (
                pointer.group(1)
                if pointer
                else re.findall(r"\b\w+\b", re.sub(r"\[[^]]*\]", "", declaration))[-1]
            )
            fields.append("type" if field == "type_" else field)
        result[name] = fields
    return result


def layout_source(header: Path, structs: dict[str, list[str]], *, snapshot: bool) -> str:
    source = "#include <stdio.h>\n#include <stddef.h>\n" + translation_unit(header)
    source += "int main(void) {\n"
    for name, fields in sorted(structs.items()):
        source += f'printf("{name} %zu %zu\\n", sizeof(struct {name}), _Alignof(struct {name}));\n'
        for field in fields:
            spelling = "type_" if snapshot and field == "type" else field
            source += (
                f'printf("{name}.{field} %zu %zu\\n", offsetof(struct {name}, {spelling}), '
                f"sizeof(((struct {name} *)0)->{spelling}));\n"
            )
    return source + "return 0;\n}\n"


def layout(
    header: Path,
    structs: dict[str, list[str]],
    source: Path,
    includes: list[str],
    *,
    snapshot: bool,
) -> str:
    source.write_text(layout_source(header, structs, snapshot=snapshot))
    binary = source.with_suffix("")
    run(["gcc", *includes, "-Werror", str(source), "-o", str(binary)])
    return lines(run([str(binary)]).splitlines())


def check_structures(library: Library, temporary: Path, includes: list[str]) -> None:
    shipping = structures(
        "\n".join(header.read_text() for header in library.headers), library.prefix
    )
    snapshot = structures(library.snapshot.read_text(), library.prefix)
    equal(lines(set(shipping)), lines(set(snapshot)), "public struct names differ")
    for name in shipping:
        equal(lines(shipping[name]), lines(snapshot[name]), f"{name}: field names differ")
    header = library.root / "include" / f"{library.prefix}.h"
    equal(
        layout(header, shipping, temporary / "layout-hand.c", includes, snapshot=False),
        layout(library.snapshot, snapshot, temporary / "layout-snap.c", includes, snapshot=True),
        "struct sizes, alignment, field offsets or field sizes differ",
    )
    print(f"ok: {len(shipping)} complete struct and field layouts")


def data_symbols(source: str, prefix: str) -> set[str]:
    return {
        re.sub(r"\s+", " ", declaration).strip()
        for declaration in re.findall(
            rf'\bextern\s+[^;(){{}}"#]*\b{prefix}_[a-z_]+\s*;', without_comments(source)
        )
    }


def checked_symbols(library: Library, temporary: Path, includes: list[str]) -> set[str]:
    hand = temporary / "hand.c"
    hand.write_text(translation_unit(library.root / "include" / f"{library.prefix}.h"))
    snap = temporary / "snap.c"
    snap.write_text(translation_unit(library.snapshot))
    prototypes = functions(hand, library.prefix, includes)
    equal(
        lines(prototypes),
        lines(functions(snap, library.prefix, includes)),
        "function signatures differ",
    )
    data = data_symbols("\n".join(h.read_text() for h in library.headers), library.prefix)
    equal(
        lines(data),
        lines(data_symbols(library.snapshot.read_text(), library.prefix)),
        "data symbols differ",
    )
    names = set()
    for declaration in prototypes | data:
        names.update(re.findall(rf"\b({library.prefix}_[a-z_]+)\s*(?:\(|;)", declaration))
    if not names:
        raise VerificationError("no public symbols found")
    print(f"ok: {len(prototypes)} canonical function prototypes and {len(data)} data declarations")
    return names


def check_binary(library: Library, expected: set[str]) -> str:
    dynamic = run(["readelf", "-dW", str(library.shared)])
    needed = set(re.findall(r"\(NEEDED\).*?\[(.*?)\]", dynamic))
    if library.member == "librsi":
        equal(lines(needed), "libc.so.6\n", "librsi must depend only on libc")
    exported = {
        line.split()[0]
        for line in run(
            ["nm", "-D", "--defined-only", "--format=posix", str(library.shared)]
        ).splitlines()
        if line.strip()
    }
    equal(lines(exported), lines(expected), "shared-library exports differ")
    sonames = re.findall(r"\(SONAME\).*?\[(.*?)\]", dynamic)
    if len(sonames) > 1 or any(Path(name).name != name for name in sonames):
        raise VerificationError("invalid shared-library SONAME")
    print(f"ok: exact {len(exported)} DSO exports")
    return sonames[0] if sonames else library.shared.name


def smoke(library: Library, temporary: Path, includes: list[str], soname: str) -> None:
    # Resolve Bazel's SONAME without assuming installation symlinks already exist.
    runtime = temporary / "runtime"
    runtime.mkdir()
    (runtime / soname).symlink_to(library.shared)
    environment = dict(os.environ, LD_LIBRARY_PATH=str(runtime))
    source = library.root / "tools/package-smoke.c"
    for compiler, language in (("gcc", "c"), (CXX, "c++")):
        for linkage, archive in (("shared", library.shared), ("static", library.static)):
            binary = temporary / f"smoke-{language}-{linkage}"
            run(
                [
                    compiler,
                    *includes,
                    "-Werror",
                    "-x",
                    language,
                    str(source),
                    "-x",
                    "none",
                    str(archive),
                    "-ldl",
                    "-lpthread",
                    "-lm",
                    "-o",
                    str(binary),
                ]
            )
            run([str(binary)], env=environment)
    print("ok: C and C++ link and run with shared and static libraries")


def verify(library: Library) -> None:
    check_tools()
    for artifact in (library.shared, library.static):
        if not artifact.is_file():
            raise VerificationError(f"missing built library: {artifact}; build //:artifacts first")
    with tempfile.TemporaryDirectory(prefix=f"{library.member}-abi-") as directory:
        temporary = Path(directory)
        (temporary / "include").mkdir()
        (temporary / "include/pkm").symlink_to(ROOT / "vendor/kernel/include/uapi/pkm")
        includes = ["-I", str(library.root / "include"), "-I", str(temporary / "include")]
        check_snapshot(library, temporary)
        check_headers(library, includes)
        symbols = checked_symbols(library, temporary, includes)
        check_structures(library, temporary, includes)
        soname = check_binary(library, symbols)
        smoke(library, temporary, includes, soname)
    print(f"ABI VERIFIED: {library.member}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("member", choices=("libpeios", "librsi"))
    parser.add_argument("--shared", type=Path, required=True)
    parser.add_argument("--static", type=Path, required=True)
    args = parser.parse_args()
    library = Library(
        args.member,
        "peios" if args.member == "libpeios" else "rsi",
        args.shared.resolve(),
        args.static.resolve(),
    )
    try:
        verify(library)
    except (VerificationError, OSError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
