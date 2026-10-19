#!/usr/bin/python3
"""Stage the SDK's declared libraries and debug sources for canonical PPK commits."""

import hashlib
import json
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import TypedDict, cast

sys.dont_write_bytecode = True
from licences import collect  # noqa: E402
from validate import validate  # noqa: E402

LIBDIR = Path("usr/lib/x86_64-linux-peios")
LIBRARIES = ("libpeios", "librsi")
DEBUG_ROOT = Path("usr/src/debug/dev.peios.peios-sdk")


class ComponentInput(TypedDict):
    """Artifact and transitive source inputs supplied by Bazel."""

    binary: str
    inputs: list[str]


class StageSpec(TypedDict):
    """The staging rule's declared inputs and target policy."""

    components: dict[str, ComponentInput]
    sources: list[str]
    version: str
    rustc: str
    toolchain_inputs: list[str]
    native: bool


def copy(source: Path, target: Path) -> None:
    """Copy a declared file with a stable installed mode."""
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    target.chmod(0o644)


def split_debug(component: str, elf: Path, output: Path) -> None:
    """Retain matching debug information under its GNU build ID."""
    notes = subprocess.check_output(["readelf", "-n", str(elf)], text=True)
    match = re.search(r"Build ID: ([0-9a-f]+)", notes)
    if not match:
        raise ValueError(f"missing build ID: {elf}")
    identity = match.group(1)
    debug = (
        output / f"debug/{component}/usr/lib/debug/.build-id/{identity[:2]}/{identity[2:]}.debug"
    )
    debug.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["objcopy", "--only-keep-debug", elf, debug], check=True)
    subprocess.run(["strip", "--strip-unneeded", elf], check=True)
    subprocess.run(["objcopy", f"--add-gnu-debuglink={debug}", elf], check=True)


def source_files(names: list[str]) -> set[Path]:
    """Resolve declared source products without traversing an ambient checkout."""
    files = set()
    for name in names:
        path = Path(name)
        if path.is_dir():
            files.update(
                p for p in path.rglob("*") if p.is_file() and p.suffix in {".rs", ".h", ".c"}
            )
        elif path.is_file():
            files.add(path)
        else:
            raise ValueError(f"missing declared input: {path}")
    return files


def stage_headers(lib: str, sources: set[Path], output: Path, version: str) -> None:
    """Install public headers, the development symlink, and pkg-config metadata."""
    include = Path(lib) / "include"
    for path in sorted(sources):
        if path.is_relative_to(include):
            copy(path, output / "headers" / lib / path.relative_to(include))
    if not (output / "headers" / lib / f"{lib[3:]}.h").is_file():
        raise ValueError(f"missing public umbrella header for {lib}")
    (output / LIBDIR / f"{lib}.so").symlink_to(f"{lib}.so.0")
    metadata = Path(lib, f"{lib[3:]}.pc.in").read_text(encoding="utf-8")
    metadata = metadata.replace("@prefix@", "/usr").replace("@libdir@", "/" + str(LIBDIR))
    metadata = metadata.replace("@version@", version)
    target = output / LIBDIR / "pkgconfig" / f"{lib[3:]}.pc"
    target.parent.mkdir(exist_ok=True)
    target.write_text(metadata, encoding="utf-8")


def stage_library(
    lib: str, spec: StageSpec, sources: set[Path], output: Path
) -> tuple[str, set[Path]]:
    """Stage both link modes and retain the actual source and notice closure."""
    shared, static = (spec["components"][name] for name in (lib, lib + "-static"))
    elf = output / LIBDIR / f"{lib}.so.0"
    copy(Path(shared["binary"]), elf)
    split_debug(lib, elf, output)
    archive = output / LIBDIR / f"{lib}.a"
    copy(Path(static["binary"]), archive)
    subprocess.run(["objcopy", "--strip-debug", archive], check=True)
    # Include only this member's source closure, plus declared workspace metadata/notices.
    inputs = source_files(shared["inputs"] + static["inputs"])
    metadata = {
        path
        for path in sources
        if path
        in {
            Path("Cargo.toml"),
            Path("Cargo.lock"),
            Path("LICENSE"),
            Path("vendor/kernel/ORIGIN.json"),
        }
        or path.is_relative_to(Path(lib) / "third-party")
        or path.is_relative_to("vendor/kernel/LICENSES")
    }
    inputs.update(metadata)
    expression = collect(sorted(inputs), output / "licenses" / lib)
    for path in sorted(inputs):
        if path.is_absolute() or ".." in path.parts:
            raise ValueError(f"non-relative debug source: {path}")
        if path.suffix not in {".so", ".a", ".rlib", ".o"}:
            copy(path, output / "debugsource" / lib / DEBUG_ROOT / path)
    stage_headers(lib, sources, output, spec["version"])
    return expression, inputs


def check_versions(sources: set[Path], version: str) -> None:
    """Require one SDK version, leaving kernel-owned crate versions untouched."""
    if Path("Cargo.toml") not in sources:
        raise ValueError("missing workspace manifest")
    workspace = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]
    if workspace["package"]["version"] != version:
        raise ValueError("workspace version differs from Bazel")
    for member in (*LIBRARIES, "peios-cabi"):
        manifest = Path(member) / "Cargo.toml"
        if manifest not in sources:
            raise ValueError(f"missing member manifest: {member}")
        package = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]
        if package["version"] != {"workspace": True}:
            raise ValueError(f"member does not inherit the SDK version: {member}")


def input_identity(names: list[str]) -> str:
    """Hash each declared input before hashing its deterministic identity map."""
    fingerprints = {}
    for name in sorted(set(names)):
        with Path(name).open("rb") as stream:
            fingerprints[name] = hashlib.file_digest(stream, "sha256").hexdigest()
    return hashlib.sha256(json.dumps(fingerprints, sort_keys=True).encode()).hexdigest()


def write_build_record(
    spec: StageSpec, output: Path, inputs: set[Path], licenses: dict[str, str]
) -> None:
    """Record input/compiler identity and distinguish development from native builds."""
    record = {
        "version": spec["version"],
        "userspace_abi": "peios-native" if spec["native"] else "debian-development",
        "release_ready": False,
        "signed": False,
        "checks": "native-hardening-and-payload" if spec["native"] else "development-payload",
        "input_digest": input_identity([str(path) for path in inputs]),
        "licences": licenses,
        "kernel": json.loads(Path("vendor/kernel/ORIGIN.json").read_text(encoding="utf-8")),
        "compiler": {
            "version": subprocess.check_output(
                [spec["rustc"], "--version", "--verbose"], text=True
            ).strip(),
            "input_digest": input_identity(spec["toolchain_inputs"]),
        },
    }
    (output / "build.json").write_text(
        json.dumps(record, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )


def main() -> None:
    """Stage and validate the whole family before any package is committed."""
    spec = cast(StageSpec, json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")))
    output = Path(sys.argv[2])
    output.mkdir(parents=True, exist_ok=True)
    sources = source_files(spec["sources"])
    check_versions(sources, spec["version"])
    expected = {name for lib in LIBRARIES for name in (lib, lib + "-static")}
    if set(spec["components"]) != expected:
        raise ValueError("incomplete or unexpected library set")
    inputs, licenses = set(sources), {}
    for lib in LIBRARIES:
        licenses[lib], closure = stage_library(lib, spec, sources, output)
        inputs.update(closure)
    for path in sorted(Path("packages/definitions").glob("*.json")):
        definition = json.loads(path.read_text(encoding="utf-8"))["package"]
        name = definition["name"].removeprefix("dev.peios.")
        lib = name.split("-", 1)[0]
        expected_license = "MIT" if name.endswith("-devel") else licenses[lib]
        if definition["license"] != expected_license:
            raise ValueError(f"{path}: expected licence expression {expected_license}")
    validate(output, native=spec["native"])
    write_build_record(spec, output, inputs, licenses)


if __name__ == "__main__":
    main()
