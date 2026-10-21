"""Regression tests for the separate build-toolchain and consumer MSRV contract."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_docs


class ToolchainContractTests(unittest.TestCase):
    def validate(self, channel, builder, msrv="1.99.0"):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "rust-toolchain.toml").write_text(
                f'[toolchain]\nchannel = "{channel}"\n', encoding="utf-8"
            )
            (root / "Dockerfile").write_text(
                f"FROM rust:{builder}-trixie@sha256:{'a' * 64} AS builder\n",
                encoding="utf-8",
            )
            errors = []
            with patch.object(check_docs, "ROOT", root):
                check_docs.check_toolchains({"rust-version": msrv}, errors)
            return errors

    def test_build_can_be_newer_than_msrv(self):
        self.assertEqual(self.validate("1.99.1", "1.99.1"), [])

    def test_build_cannot_be_older_than_msrv(self):
        self.assertTrue(self.validate("1.98.1", "1.98.1"))

    def test_builder_must_match_selected_build_toolchain(self):
        self.assertTrue(self.validate("1.99.1", "1.99.0"))


if __name__ == "__main__":
    unittest.main()
