#!/usr/bin/env python3
"""Regression tests for the ABI verifier's source and compiler-based gates."""

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "verify_abi", Path(__file__).with_name("verify-abi.py")
)
assert SPEC is not None and SPEC.loader is not None
VERIFY = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = VERIFY
SPEC.loader.exec_module(VERIFY)


class AbiVerifierTests(unittest.TestCase):
    def test_cpp_linkage_is_not_a_data_declaration(self) -> None:
        source = """extern "C" {
extern const struct mapping peios_mapping;
typedef struct peios_writer peios_writer;
}"""
        self.assertEqual(
            VERIFY.data_symbols(source, "peios"),
            {"extern const struct mapping peios_mapping;"},
        )

    def test_typedef_storage_arrays_and_callback_fields_are_recognized(self) -> None:
        source = """typedef struct peios_view { unsigned long _opaque[4]; } peios_view;
struct peios_handler { int (*handle)(int, void *); unsigned type_; };"""
        self.assertEqual(
            VERIFY.structures(source, "peios"),
            {"peios_view": ["_opaque"], "peios_handler": ["handle", "type"]},
        )

    def test_function_normalization_keeps_const_and_pointer_depth(self) -> None:
        canonical = VERIFY.normalize_function("extern int peios_call (const struct item **);")
        changed = VERIFY.normalize_function("extern int peios_call (struct item *);")
        self.assertNotEqual(canonical, changed)
        self.assertEqual(canonical, "int peios_call (const item **);")

    def test_layout_probe_detects_field_size_even_when_struct_size_matches(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            header = root / "abi.h"
            fields = {"peios_item": ["value", "tail"]}
            header.write_text("struct peios_item { unsigned value; unsigned long tail; };\n")
            original = VERIFY.layout(header, fields, root / "one.c", [], snapshot=False)
            header.write_text("struct peios_item { unsigned short value; unsigned long tail; };\n")
            changed = VERIFY.layout(header, fields, root / "two.c", [], snapshot=False)
        self.assertNotEqual(original, changed)
        self.assertEqual(original.splitlines()[0], changed.splitlines()[0])

    def test_snapshot_generator_version_cannot_be_skipped(self) -> None:
        with patch.object(VERIFY.shutil, "which", return_value="/usr/bin/tool"):
            with patch.object(VERIFY, "run", return_value="cbindgen 0.28.0\n"):
                with self.assertRaisesRegex(VERIFY.VerificationError, "required cbindgen 0.29.2"):
                    VERIFY.check_tools()

    def test_changed_public_symbols_fail_comparison(self) -> None:
        with self.assertRaisesRegex(VERIFY.VerificationError, "exports"):
            VERIFY.equal("peios_old\n", "peios_new\n", "exports")

    def test_failed_tools_cannot_pass_with_empty_output(self) -> None:
        with self.assertRaises(VERIFY.VerificationError):
            VERIFY.run([sys.executable, "-c", "raise SystemExit(2)"])


if __name__ == "__main__":
    unittest.main()
