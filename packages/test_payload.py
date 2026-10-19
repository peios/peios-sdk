"""Exercise staged SDK ELF gates and component-specific source closure."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import stage
import validate


class PayloadTests(unittest.TestCase):
    """Build small real ELF fixtures without loading SDK code or kernel APIs."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture_directory = tempfile.TemporaryDirectory()
        cls.fixture = Path(cls.fixture_directory.name)
        source = cls.fixture / "library.c"
        source.write_text("int sdk_fixture(void) { return 42; }\n", encoding="utf-8")
        for name in ("libpeios", "librsi"):
            elf = cls.fixture / stage.LIBDIR / (name + ".so.0")
            elf.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(
                [
                    "cc",
                    "-shared",
                    "-fPIC",
                    "-g",
                    "library.c",
                    "-o",
                    str(elf),
                    f"-Wl,--build-id=sha1,-soname,{name}.so.0,-z,relro,-z,now,-z,noexecstack",
                    "-Wl,--no-as-needed",
                    "-lc",
                ],
                cwd=cls.fixture,
                check=True,
            )
            stage.split_debug(name, elf, cls.fixture)
            subprocess.run(
                ["cc", "-c", "library.c", "-o", "library.o"],
                cwd=cls.fixture,
                check=True,
            )
            subprocess.run(
                ["ar", "rcs", str(elf.with_name(name + ".a")), "library.o"],
                cwd=cls.fixture,
                check=True,
            )
            elf.with_name(name + ".so").symlink_to(elf.name)
            metadata = elf.parent / "pkgconfig" / (name[3:] + ".pc")
            metadata.parent.mkdir(exist_ok=True)
            metadata.write_text("Version: 0.5.11\n", encoding="utf-8")

    @classmethod
    def tearDownClass(cls) -> None:
        cls.fixture_directory.cleanup()

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.output = Path(self.directory.name) / "stage"
        shutil.copytree(self.fixture, self.output, symlinks=True)

    def test_development_payload_passes(self) -> None:
        validate.validate(self.output, native=False)

    def test_development_payload_does_not_claim_native_hardening(self) -> None:
        with self.assertRaisesRegex(ValueError, "missing native (CET|packed relocations)"):
            validate.validate(self.output, native=True)

    def test_wrong_development_link_fails(self) -> None:
        link = self.output / stage.LIBDIR / "libpeios.so"
        link.unlink()
        link.symlink_to("librsi.so.0")
        with self.assertRaisesRegex(ValueError, "incorrect target"):
            validate.validate(self.output, native=False)

    def test_unexpanded_metadata_fails(self) -> None:
        metadata = self.output / stage.LIBDIR / "pkgconfig/peios.pc"
        metadata.write_text("Version: @version@\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "unexpanded template"):
            validate.validate(self.output, native=False)

    def test_missing_split_debug_fails(self) -> None:
        shutil.rmtree(self.output / "debug/libpeios")
        with self.assertRaisesRegex(ValueError, "missing split debug"):
            validate.validate(self.output, native=False)

    def test_static_archive_with_debug_fails(self) -> None:
        subprocess.run(
            ["cc", "-g", "-c", "library.c", "-o", "library.o"],
            cwd=self.output,
            check=True,
        )
        archive = self.output / stage.LIBDIR / "libpeios.a"
        archive.unlink()
        subprocess.run(["ar", "rcs", str(archive), "library.o"], cwd=self.output, check=True)
        with self.assertRaisesRegex(ValueError, "unsplit archive debug"):
            validate.validate(self.output, native=False)

    def test_elf_contract_rejects_invalid_properties(self) -> None:
        # Mutate one tool view at a time; all other views come from the real fixture.
        original = validate.readelf
        mutations = [
            ("-h", "ELF64", "ELF32", "not a 64-bit"),
            ("-h", "Advanced Micro Devices X86-64", "AArch64", "not x86-64"),
            (
                "-dW",
                "libc.so.6",
                "libunexpected.so.1",
                "unexpected shared dependencies",
            ),
            ("-dW", "libpeios.so.0", "libwrong.so.0", "incorrect SONAME"),
            ("-lW", "GNU_RELRO", "NOT_RELRO", "missing full RELRO"),
            ("-dW", "BIND_NOW", "NO_BIND", "missing full RELRO"),
        ]
        for option, before, after, diagnostic in mutations:
            with self.subTest(diagnostic=diagnostic):

                def changed(
                    path: Path,
                    view: str,
                    option: str = option,
                    before: str = before,
                    after: str = after,
                ) -> str:
                    result = original(path, view)
                    if view == option:
                        self.assertIn(before, result)
                        return result.replace(before, after)
                    return result

                with (
                    patch.object(validate, "readelf", side_effect=changed),
                    self.assertRaisesRegex(ValueError, diagnostic),
                ):
                    validate.validate_elf(self.output, "libpeios", native=False)

    def test_additional_shared_dependency_fails(self) -> None:
        original = validate.readelf

        def changed(path: Path, view: str) -> str:
            result = original(path, view)
            if view == "-dW":
                result += "\n0x1 (NEEDED) Shared library: [libunexpected.so.1]\n"
            return result

        with (
            patch.object(validate, "readelf", side_effect=changed),
            self.assertRaisesRegex(ValueError, "unexpected shared dependencies"),
        ):
            validate.validate_elf(self.output, "libpeios", native=False)


class SourceClosureTests(unittest.TestCase):
    """Keep package-tool manifests out of an SDK library's debug-source closure."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        previous = Path.cwd()
        os.chdir(self.directory.name)
        self.addCleanup(os.chdir, previous)

    def write(self, name: str, contents: str = "fixture") -> Path:
        path = Path(name)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")
        return path

    def test_component_closure_excludes_unrelated_package_tools(self) -> None:
        members = {
            self.write("libpeios/Cargo.toml"),
            self.write("libpeios/src/lib.rs"),
            self.write("peios-cabi/Cargo.toml"),
            self.write("peios-cabi/src/lib.rs"),
        }
        metadata = {
            self.write("Cargo.toml"),
            self.write("Cargo.lock"),
            self.write("LICENSE"),
            self.write("vendor/kernel/ORIGIN.json"),
            self.write("vendor/kernel/LICENSES/preferred/MIT"),
            self.write("libpeios/third-party/NOTICE"),
        }
        unrelated = {
            self.write("vendor/ppk/Cargo.toml"),
            self.write("librsi/Cargo.toml"),
            self.write("librsi/third-party/NOTICE"),
            self.write("libpeios/build.rs"),
        }
        binary = self.write("artifacts/libpeios.so")
        spec = {
            "components": {
                "libpeios": {
                    "binary": str(binary),
                    "inputs": [str(p) for p in members],
                },
                "libpeios-static": {"binary": str(binary), "inputs": []},
            },
            "version": "0.5.11",
        }
        with (
            patch.object(stage, "split_debug"),
            patch.object(stage.subprocess, "run"),
            patch.object(stage, "collect", return_value="MIT") as collect,
            patch.object(stage, "stage_headers"),
        ):
            expression, inputs = stage.stage_library(
                "libpeios", spec, members | metadata | unrelated, Path("output")
            )
        self.assertEqual(expression, "MIT")
        self.assertEqual(inputs, members | metadata)
        self.assertEqual(set(collect.call_args.args[0]), inputs)
        debug_sources = Path("output/debugsource/libpeios") / stage.DEBUG_ROOT
        self.assertEqual(
            {p.relative_to(debug_sources) for p in debug_sources.rglob("*") if p.is_file()},
            inputs,
        )

    def test_members_must_inherit_one_sdk_version(self) -> None:
        sources = {self.write("Cargo.toml", '[workspace.package]\nversion = "0.5.11"\n')}
        for member in ("libpeios", "librsi", "peios-cabi"):
            sources.add(self.write(f"{member}/Cargo.toml", "[package]\nversion.workspace = true\n"))
        stage.check_versions(sources, "0.5.11")
        self.write("librsi/Cargo.toml", '[package]\nversion = "0.5.10"\n')
        with self.assertRaisesRegex(ValueError, "does not inherit"):
            stage.check_versions(sources, "0.5.11")

    def test_source_input_must_exist(self) -> None:
        with self.assertRaisesRegex(ValueError, "missing declared input"):
            stage.source_files(["missing.rs"])


if __name__ == "__main__":
    unittest.main()
