#!/usr/bin/env python3
"""Reject repository metadata and generated caches in the declared source closure."""

import argparse
import json
from pathlib import Path, PurePosixPath

FORBIDDEN_COMPONENTS = {
    ".git",
    ".trail-cache",
    "__pycache__",
    ".ruff_cache",
    ".pytest_cache",
    ".mypy_cache",
    ".cache",
    ".venv",
    "node_modules",
    "target",
    "out",
}


def forbidden_source(source: str) -> bool:
    """Permit owned configuration while rejecting checkout-specific build inputs."""
    parts = PurePosixPath(source).parts
    if any(part in FORBIDDEN_COMPONENTS for part in parts):
        return True
    if any(part.startswith(("bazel-", ".ci-")) for part in parts[:-1]):
        return True
    if "docs" in parts and "dist" in parts:
        return True
    return source.endswith((".pyc", ".pyo"))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    sources = json.loads(args.manifest.read_text())
    invalid = [source for source in sources if forbidden_source(source)]
    if invalid:
        raise SystemExit("Non-source inputs declared for packaging:\n" + "\n".join(invalid))
    print(f"Checked {len(sources)} declared inputs: no repository metadata or build caches")


if __name__ == "__main__":
    main()
