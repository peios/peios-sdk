#!/usr/bin/env python3
"""Namespace private archive definitions so Rust and C runtimes can coexist."""

import argparse
import subprocess
import tempfile
from pathlib import Path


def defined_symbols(archive: Path) -> set[str]:
    """Read global definitions without demangling their linker names."""
    result = subprocess.run(
        ["nm", "-g", "--defined-only", "--format=posix", str(archive)],
        check=True,
        capture_output=True,
        text=True,
    )
    return {fields[0] for line in result.stdout.splitlines() if len(fields := line.split()) >= 3}


def isolate(source: Path, output: Path, prefix: str) -> None:
    """Rename every private definition and its references across archive members.

    Merely hiding or localizing definitions cannot resolve references between
    members. A namespace preserves those references, keeps the SDK's abort
    runtime independent, and leaves undefined libc imports unchanged.
    """
    symbols = defined_symbols(source)
    public = {symbol for symbol in symbols if symbol.startswith(prefix + "_")}
    if not public:
        raise ValueError(f"archive has no {prefix}_ API definitions")
    namespace = f"__lib{prefix}_private_"
    if any(symbol.startswith(namespace) for symbol in symbols):
        raise ValueError("archive is already isolated")
    with tempfile.TemporaryDirectory(prefix="peios-static-") as directory:
        mapping = Path(directory) / "symbols.txt"
        mapping.write_text(
            "".join(f"{symbol} {namespace}{symbol}\n" for symbol in sorted(symbols - public))
        )
        subprocess.run(
            ["objcopy", "--redefine-syms=" + str(mapping), str(source), str(output)], check=True
        )
    expected = public | {namespace + symbol for symbol in symbols - public}
    if defined_symbols(output) != expected:
        raise ValueError("isolated archive definitions differ from the expected public/private set")


def main() -> None:
    """Transform one compiler-produced archive into a public SDK artifact."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--prefix", required=True, choices=["peios", "rsi"])
    arguments = parser.parse_args()
    isolate(arguments.source, arguments.output, arguments.prefix)


if __name__ == "__main__":
    main()
