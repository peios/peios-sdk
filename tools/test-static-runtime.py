#!/usr/bin/env python3
"""Link both SDK archives into Rust and exercise independent runtime behavior."""

import os
import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> None:
    """Check debug/unwind and optimized/abort consumer configurations."""
    peios, rsi = (Path(value).resolve() for value in sys.argv[1:])
    source = Path(__file__).with_name("static-rust-smoke.rs")
    with tempfile.TemporaryDirectory(prefix="peios-rust-static-") as directory:
        for panic, optimization in (("unwind", "0"), ("abort", "2")):
            binary = Path(directory) / panic
            subprocess.run(
                [
                    os.environ["RUSTC"],
                    "--edition=2021",
                    str(source),
                    "-C",
                    "panic=" + panic,
                    "-C",
                    "opt-level=" + optimization,
                    "-C",
                    "link-arg=" + str(peios),
                    "-C",
                    "link-arg=" + str(rsi),
                    "-o",
                    str(binary),
                ],
                check=True,
            )
            subprocess.run([str(binary)], check=True)


if __name__ == "__main__":
    main()
