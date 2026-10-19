"""Exercise snapshot provenance and failure recovery using disposable repositories."""

import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

MODULE_PATH = Path(__file__).with_name("update-kernel-inputs.py")
SPEC = importlib.util.spec_from_file_location("update_kernel_inputs", MODULE_PATH)
assert SPEC and SPEC.loader
updater = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(updater)


class SnapshotTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="sdk-kernel-inputs-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.kernel = self.root / "kernel"
        self.kernel.mkdir()
        self.snapshot = self.root / "snapshot"
        self.run_git("init", "-q")
        self.run_git("config", "user.name", "Snapshot test")
        self.run_git("config", "user.email", "snapshot-test@example.invalid")
        for name in updater.INPUT_PATHS:
            path = self.input_path(name)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"committed content for {name}\n")
        self.executable = self.kernel / "peios/rust/kacs-core/tool"
        self.executable.write_text("#!/bin/sh\nexit 0\n")
        self.executable.chmod(0o755)
        self.commit = self.commit_files()
        self.metadata = updater.update_snapshot(self.kernel, self.commit, self.snapshot)

    def run_git(self, *arguments: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(self.kernel), *arguments],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        )
        return result.stdout.strip()

    def input_path(self, name: str) -> Path:
        if name.endswith(".rs") or name.startswith("LICENSES/"):
            return self.kernel / name
        return self.kernel / name / "fixture.rs"

    def commit_files(self) -> str:
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "test: create kernel fixture")
        return self.run_git("rev-parse", "HEAD")

    def test_updates_from_exact_commit_not_dirty_worktree(self) -> None:
        source = self.input_path("peios/rust/kacs-core")
        expected = source.read_bytes()
        source.write_bytes(b"uncommitted changes\n")
        updater.update_snapshot(self.kernel, self.commit, self.snapshot)
        self.assertEqual((self.snapshot / source.relative_to(self.kernel)).read_bytes(), expected)
        self.assertEqual(updater.check_snapshot(self.snapshot)["commit"], self.commit)
        self.assertEqual(
            (self.snapshot / self.executable.relative_to(self.kernel)).stat().st_mode & 0o777, 0o755
        )
        for name in updater.LICENSE_PATHS:
            self.assertEqual((self.snapshot / name).read_bytes(), (self.kernel / name).read_bytes())

    def test_ignores_local_git_replacement_objects(self) -> None:
        source = self.input_path("peios/rust/kacs-core")
        original = source.read_bytes()
        source.write_text("replacement commit content\n")
        replacement = self.commit_files()
        self.run_git("replace", self.commit, replacement)
        updater.update_snapshot(self.kernel, self.commit, self.snapshot)
        self.assertEqual((self.snapshot / source.relative_to(self.kernel)).read_bytes(), original)

    def test_rejects_modified_snapshot_without_overwriting_it(self) -> None:
        source = self.snapshot / "peios/rust/kacs-core/fixture.rs"
        source.write_bytes(b"local work\n")
        with self.assertRaisesRegex(updater.SnapshotError, "was changed"):
            updater.update_snapshot(self.kernel, self.commit, self.snapshot)
        self.assertEqual(source.read_bytes(), b"local work\n")

    def test_detects_extra_and_missing_files(self) -> None:
        extra = self.snapshot / "peios/rust/kacs-core/untracked.rs"
        extra.write_text("unexpected\n")
        with self.assertRaisesRegex(updater.SnapshotError, "extra="):
            updater.check_snapshot(self.snapshot)
        extra.unlink()
        (self.snapshot / "peios/rust/kacs-core/fixture.rs").unlink()
        with self.assertRaisesRegex(updater.SnapshotError, "missing="):
            updater.check_snapshot(self.snapshot)

    def test_rejects_symlinks_in_snapshot(self) -> None:
        link = self.snapshot / "peios/rust/kacs-core/link.rs"
        link.symlink_to(self.input_path("peios/rust/kacs-core"))
        with self.assertRaisesRegex(updater.SnapshotError, "nonregular"):
            updater.check_snapshot(self.snapshot)

    def test_bazel_file_transport_requires_explicit_mode(self) -> None:
        runfiles = self.root / "runfiles"
        for source in self.snapshot.rglob("*"):
            target = runfiles / source.relative_to(self.snapshot)
            if source.is_dir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.symlink_to(source)
        with self.assertRaisesRegex(updater.SnapshotError, "real directories/files"):
            updater.check_snapshot(runfiles)
        self.assertEqual(updater.check_snapshot(runfiles, bazel_inputs=True), self.metadata)
        (self.snapshot / "peios/rust/kacs-core/fixture.rs").write_text("changed input\n")
        with self.assertRaisesRegex(updater.SnapshotError, "was changed"):
            updater.check_snapshot(runfiles, bazel_inputs=True)

    def test_bazel_mode_rejects_nested_directory_links(self) -> None:
        (self.snapshot / "hidden").symlink_to(self.kernel, target_is_directory=True)
        with self.assertRaisesRegex(updater.SnapshotError, "nonregular"):
            updater.check_snapshot(self.snapshot, bazel_inputs=True)

    def test_bazel_root_link_and_cache_modes_are_transport(self) -> None:
        runfiles = self.root / "runfiles"
        runfiles.symlink_to(self.snapshot, target_is_directory=True)
        (self.snapshot / "peios/rust/kacs-core/fixture.rs").chmod(0o555)
        with self.assertRaises(updater.SnapshotError):
            updater.check_snapshot(runfiles)
        self.assertEqual(updater.check_snapshot(runfiles, bazel_inputs=True), self.metadata)

    def test_rejects_symlinked_git_inputs(self) -> None:
        link = self.kernel / "peios/rust/kacs-core/link.rs"
        link.symlink_to("fixture.rs")
        commit = self.commit_files()
        with self.assertRaisesRegex(updater.SnapshotError, "not a regular file"):
            updater.update_snapshot(self.kernel, commit, self.snapshot)
        self.assertEqual(updater.check_snapshot(self.snapshot)["commit"], self.commit)

    def test_accepts_group_writable_checkout_permissions(self) -> None:
        (self.snapshot / "peios/rust/kacs-core/fixture.rs").chmod(0o664)
        (self.snapshot / self.executable.relative_to(self.kernel)).chmod(0o775)
        self.assertEqual(updater.check_snapshot(self.snapshot), self.metadata)

    def test_detects_added_executable_mode(self) -> None:
        (self.snapshot / "peios/rust/kacs-core/fixture.rs").chmod(0o755)
        with self.assertRaisesRegex(updater.SnapshotError, "mode was changed"):
            updater.check_snapshot(self.snapshot)

    def test_detects_changed_executable_mode(self) -> None:
        (self.snapshot / self.executable.relative_to(self.kernel)).chmod(0o644)
        with self.assertRaisesRegex(updater.SnapshotError, "mode was changed"):
            updater.check_snapshot(self.snapshot)

    def test_rejects_moving_or_abbreviated_revision(self) -> None:
        for revision in ("HEAD", self.commit[:12]):
            with (
                self.subTest(revision=revision),
                self.assertRaisesRegex(updater.SnapshotError, "complete lowercase"),
            ):
                updater.update_snapshot(self.kernel, revision, self.snapshot)
        self.assertEqual(updater.check_snapshot(self.snapshot)["commit"], self.commit)

    def test_rejects_path_escape_in_metadata(self) -> None:
        self.metadata["files"]["../outside"] = "0" * 64
        (self.snapshot / "ORIGIN.json").write_text(json.dumps(self.metadata))
        with self.assertRaisesRegex(updater.SnapshotError, "noncanonical"):
            updater.check_snapshot(self.snapshot)

    def test_rejects_missing_required_license_from_commit(self) -> None:
        self.input_path(updater.LICENSE_PATHS[0]).unlink()
        commit = self.commit_files()
        with self.assertRaisesRegex(updater.SnapshotError, "missing required input"):
            updater.update_snapshot(self.kernel, commit, self.snapshot)
        self.assertEqual(updater.check_snapshot(self.snapshot)["commit"], self.commit)

    def test_ignores_export_ignore_attributes(self) -> None:
        (self.kernel / ".gitattributes").write_text(
            "peios/rust/kacs-core/fixture.rs export-ignore\n"
        )
        commit = self.commit_files()
        updater.update_snapshot(self.kernel, commit, self.snapshot)
        self.assertTrue((self.snapshot / "peios/rust/kacs-core/fixture.rs").is_file())

    def test_restores_previous_snapshot_if_install_rename_fails(self) -> None:
        replace = os.replace
        calls = 0

        def fail_install(source: Path, destination: Path) -> None:
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("injected install failure")
            replace(source, destination)

        with patch.object(updater.os, "replace", side_effect=fail_install):
            with self.assertRaisesRegex(OSError, "injected install failure"):
                updater.update_snapshot(self.kernel, self.commit, self.snapshot)
        self.assertEqual(updater.check_snapshot(self.snapshot), self.metadata)
        self.assertEqual(list(self.root.glob(".kernel-update-*")), [])

    def test_preserves_backup_if_rollback_also_fails(self) -> None:
        replace = os.replace
        calls = 0

        def fail_after_backup(source: Path, destination: Path) -> None:
            nonlocal calls
            calls += 1
            if calls > 1:
                raise OSError("injected install and rollback failure")
            replace(source, destination)

        with patch.object(updater.os, "replace", side_effect=fail_after_backup):
            with self.assertRaisesRegex(OSError, "rollback failure"):
                updater.update_snapshot(self.kernel, self.commit, self.snapshot)
        backups = list(self.root.glob(".kernel-update-*/previous"))
        self.assertEqual(len(backups), 1)
        self.assertEqual(updater.check_snapshot(backups[0]), self.metadata)


if __name__ == "__main__":
    unittest.main()
