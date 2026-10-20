#!/usr/bin/env python3
"""Regression coverage for source-package input exclusions."""

import importlib.util
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_source_inputs", Path(__file__).with_name("check-source-inputs.py")
)
assert SPEC is not None and SPEC.loader is not None
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class SourceInputsTests(unittest.TestCase):
    def test_configuration_and_source_are_retained(self) -> None:
        for source in (
            ".gitignore",
            ".bazelrc",
            ".bazelversion",
            ".github/workflows/check.yml",
            "tools/bazel-vendor.py",
            "build/native-rust.bzl",
            "vendor/kernel/security/core.rs",
            "docs/learn/1--peios.shelf/peios.product/trail.toml",
        ):
            with self.subTest(source=source):
                self.assertFalse(CHECK.forbidden_source(source))

    def test_checkout_metadata_and_caches_are_rejected(self) -> None:
        for source in (
            ".git/config",
            ".git",
            "nested/.git/objects/example",
            "bazel-bin/program",
            "bazel-sdk/README.md",
            "libpeios/target/debug/libpeios.so",
            "docs/learn/dist/index.html",
            "docs/learn/.trail-cache/search-index",
            ".ci-trail/src/main.rs",
            "tools/__pycache__/check.pyc",
            "node_modules/package/index.js",
            "tools/.venv/bin/python",
            "out/store/manifest",
            "packages/.ruff_cache/check",
            "orphan.pyc",
        ):
            with self.subTest(source=source):
                self.assertTrue(CHECK.forbidden_source(source))


if __name__ == "__main__":
    unittest.main()
