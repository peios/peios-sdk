"""Validate the staged shared and static SDK artifacts without invoking kernel APIs."""

import os
import re
import subprocess
from pathlib import Path

LIBDIR = Path("usr/lib/x86_64-linux-peios")


def require(condition: bool, message: str) -> None:
    """Keep package gates effective when Python assertions are disabled."""
    if not condition:
        raise ValueError(message)


def readelf(path: Path, option: str) -> str:
    """Read one ELF view, tolerating binary CRC bytes in the debug-link section."""
    return subprocess.check_output(
        ["readelf", option, str(path)],
        text=True,
        errors="replace",
        env=os.environ | {"LC_ALL": "C"},
    )


def build_id(notes: str, path: Path) -> str:
    """Extract the single GNU build identity required by split debugging."""
    matches = re.findall(r"Build ID: ([0-9a-f]+)", notes)
    require(
        len(matches) == 1 and len(matches[0]) >= 3,
        f"{path}: missing or ambiguous build ID",
    )
    return matches[0]


def validate_elf(stage: Path, binary: str, *, native: bool) -> None:
    """Require target-specific hardening and matching split debugging information."""
    elf = stage / LIBDIR / (binary + ".so.0")
    require(elf.is_file(), f"{elf}: missing executable")
    header, program, dynamic = (readelf(elf, option) for option in ("-h", "-lW", "-dW"))
    notes, sections = readelf(elf, "-n"), readelf(elf, "-SW")
    require(bool(re.search(r"Class:\s+ELF64\b", header)), f"{elf}: not a 64-bit ELF")
    require(
        bool(re.search(r"Machine:\s+Advanced Micro Devices X86-64\s*$", header, re.MULTILINE)),
        f"{elf}: not x86-64",
    )
    require(bool(re.search(r"Type:\s+DYN\b", header)), f"{elf}: not position independent")
    require("GNU_RELRO" in program and "BIND_NOW" in dynamic, f"{elf}: missing full RELRO")
    stack = [line for line in program.splitlines() if "GNU_STACK" in line]
    require(
        len(stack) == 1 and bool(re.search(r"\sRW\s", stack[0])),
        f"{elf}: executable stack",
    )
    require(f"Library soname: [{binary}.so.0]" in dynamic, f"{elf}: incorrect SONAME")
    dependencies = re.findall(r"\(NEEDED\).*Shared library: \[([^\]]+)\]", dynamic)
    require(
        dependencies == ["libc.so.6"],
        f"{elf}: unexpected shared dependencies {dependencies}",
    )
    require("INTERP" not in program, f"{elf}: shared library has an interpreter")
    require(
        not re.search(r"\((RPATH|RUNPATH|TEXTREL)\)", dynamic),
        f"{elf}: unsafe dynamic tag",
    )
    require(".debug_info" not in sections, f"{elf}: unsplit debug information")
    if native:
        require("IBT" in notes and "SHSTK" in notes, f"{elf}: missing native CET properties")
        require(
            bool(re.search(r"\(RELR\)", dynamic)),
            f"{elf}: missing native packed relocations",
        )
    identity = build_id(notes, elf)
    component = binary
    debug = stage / f"debug/{component}/usr/lib/debug/.build-id/{identity[:2]}/{identity[2:]}.debug"
    require(debug.is_file() and debug.stat().st_size > 0, f"{elf}: missing split debug file")
    require(".debug_info" in readelf(debug, "-SW"), f"{debug}: missing debug information")
    require(build_id(readelf(debug, "-n"), debug) == identity, f"{debug}: build ID mismatch")
    require(
        debug.name in readelf(elf, "--string-dump=.gnu_debuglink"),
        f"{elf}: wrong debug link",
    )
    forbidden = {
        str(Path.cwd().resolve()),
        str(Path.cwd().resolve()).split("/execroot/")[0],
    }
    for path in (elf, debug):
        contents = path.read_bytes()
        for prefix in forbidden:
            require(
                len(prefix) <= 1 or prefix.encode() not in contents,
                f"{path}: leaked {prefix}",
            )


def validate(stage: Path, *, native: bool) -> None:
    """Check both libraries, their development metadata and static archives."""
    for name in ("libpeios", "librsi"):
        validate_elf(stage, name, native=native)
        archive = stage / LIBDIR / (name + ".a")
        members = subprocess.check_output(["ar", "t", archive], text=True)
        require(bool(members.strip()), f"{archive}: empty archive")
        require(
            ".debug_info" not in readelf(archive, "-SW"),
            f"{archive}: unsplit archive debug data",
        )
        link = stage / LIBDIR / (name + ".so")
        require(
            link.is_symlink() and os.readlink(link) == name + ".so.0",
            f"{link}: incorrect target",
        )
        metadata = stage / LIBDIR / "pkgconfig" / (name[3:] + ".pc")
        require(
            "@" not in metadata.read_text(encoding="utf-8"),
            f"{metadata}: unexpanded template",
        )
