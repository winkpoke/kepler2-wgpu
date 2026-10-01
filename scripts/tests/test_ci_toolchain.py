"""Fail-closed repository compiler resolution (Python >=3.11)."""

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "ci_toolchain.py"
SPEC = importlib.util.spec_from_file_location("ci_toolchain", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
resolver = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(resolver)

VALID = """[toolchain]
channel = "1.98.0"
profile = "minimal"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
targets = ["wasm32-unknown-unknown"]
"""


class ToolchainResolutionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.pin = self.root / "rust-toolchain.toml"
        self.pin.write_text(VALID, encoding="utf-8")
        self.versions = self.root / "scripts/ci-tool-versions.toml"
        self.versions.parent.mkdir(parents=True)
        self.versions.write_text(
            '[coverage]\ncargo-llvm-cov = "0.6.23"\n', encoding="utf-8"
        )

    def test_exact_release_components_and_targets(self):
        self.assertEqual(
            resolver.resolve(self.root),
            {
                "toolchain": "1.98.0",
                "components": "rustfmt,clippy,llvm-tools-preview",
                "targets": "wasm32-unknown-unknown",
            },
        )

    def test_exact_coverage_cli_version(self):
        self.assertEqual(resolver.resolve_coverage_version(self.root), "0.6.23")

    def test_coverage_cli_missing_malformed_and_floating_versions_fail(self):
        self.versions.unlink()
        with self.assertRaises(ValueError):
            resolver.resolve_coverage_version(self.root)
        for text in [
            "[coverage",
            "[coverage]\ncargo-llvm-cov = true",
            '[coverage]\ncargo-llvm-cov = "0.6"',
            '[coverage]\ncargo-llvm-cov = "stable"',
            '[coverage]\ncargo-llvm-cov = "0.6.23-beta.1"',
        ]:
            self.versions.write_text(text, encoding="utf-8")
            with self.subTest(text=text), self.assertRaises(ValueError):
                resolver.resolve_coverage_version(self.root)

    def test_coverage_cli_channel_is_read_from_file(self):
        self.versions.write_text(
            '[coverage]\ncargo-llvm-cov = "0.5.39"\n', encoding="utf-8"
        )
        self.assertEqual(resolver.resolve_coverage_version(self.root), "0.5.39")

    def test_channel_is_read_from_file_not_duplicated(self):
        self.pin.write_text(VALID.replace("1.98.0", "1.97.1"), encoding="utf-8")
        self.assertEqual(resolver.resolve(self.root)["toolchain"], "1.97.1")

    def test_floating_partial_and_prerelease_channels_fail(self):
        for channel in [
            "stable",
            "nightly",
            "beta",
            "1.98",
            "1.98.0-nightly",
            "nightly-2026-08-24",
            "1.98.0\nanything=oops",
        ]:
            with self.subTest(channel=channel):
                self.pin.write_text(VALID.replace("1.98.0", channel), encoding="utf-8")
                with self.assertRaises(ValueError):
                    resolver.resolve(self.root)

    def test_missing_and_malformed_files_fail(self):
        self.pin.unlink()
        with self.assertRaises(ValueError):
            resolver.resolve(self.root)
        for text in ["[toolchain", "channel = 123", "[toolchain]\nchannel = 123"]:
            self.pin.write_text(text, encoding="utf-8")
            with self.assertRaises(ValueError):
                resolver.resolve(self.root)

    def test_required_components_target_and_profile(self):
        for text in [
            VALID.replace('"clippy", ', ""),
            VALID.replace('"llvm-tools-preview"', '"other"'),
            VALID.replace('"wasm32-unknown-unknown"', '"other"'),
            VALID.replace('"minimal"', '"default"'),
        ]:
            with self.subTest(text=text):
                self.pin.write_text(text, encoding="utf-8")
                with self.assertRaises(ValueError):
                    resolver.resolve(self.root)

    def test_invalid_lists_and_output_injection_fail(self):
        for value in [
            '"rustfmt,clippy"',
            "[]",
            "[123]",
            '["rustfmt", "clippy", "llvm-tools-preview", "--other"]',
            '["rustfmt", "clippy", "llvm-tools-preview", "rustfmt"]',
        ]:
            text = VALID.replace('["rustfmt", "clippy", "llvm-tools-preview"]', value)
            self.pin.write_text(text, encoding="utf-8")
            with self.subTest(value=value), self.assertRaises(ValueError):
                resolver.resolve(self.root)

    def test_cli_json_and_github_outputs(self):
        output = self.root / "github-output"
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--github-output",
                str(output),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), resolver.resolve(self.root))
        self.assertEqual(
            output.read_text(),
            "toolchain=1.98.0\ncomponents=rustfmt,clippy,llvm-tools-preview\ntargets=wasm32-unknown-unknown\n",
        )

    def test_print_coverage_version_for_local_install(self):
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--print-coverage-version",
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "0.6.23\n")

    def test_coverage_output_uses_same_version_declaration(self):
        output = self.root / "github-output"
        coverage = self.root / "coverage-output"
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--github-output",
                str(output),
                "--coverage-output",
                str(coverage),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(coverage.read_text(), "coverage-cli-version=0.6.23\n")

    def test_missing_coverage_pin_writes_neither_output(self):
        output = self.root / "github-output"
        coverage = self.root / "coverage-output"
        self.versions.unlink()
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--github-output",
                str(output),
                "--coverage-output",
                str(coverage),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertFalse(output.exists())
        self.assertFalse(coverage.exists())

    def test_failed_cli_does_not_write_partial_outputs(self):
        output = self.root / "github-output"
        self.pin.write_text(VALID.replace("1.98.0", "stable"), encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--github-output",
                str(output),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertFalse(output.exists())


class WorkflowCompilerTests(unittest.TestCase):
    def test_coverage_jobs_use_one_pinned_run_and_retain_provenance(self):
        root = SCRIPT.parents[1]
        for name in ["coverage.yml", "coverage-gate.yml"]:
            with self.subTest(workflow=name):
                text = (root / ".github/workflows" / name).read_text(encoding="utf-8")
                self.assertEqual(text.count("cargo llvm-cov test --locked"), 1)
                self.assertIn("coverage/test-run.log", text)
                self.assertIn("coverage/test-outcome.txt", text)
                self.assertIn("coverage/adapter.txt", text)
                self.assertIn("coverage/coverage-scope.json", text)
                self.assertIn("coverage/toolchain.txt", text)
                self.assertIn("coverage/tool.txt", text)
                self.assertIn("coverage/revision.txt", text)
                self.assertIn("if: always()", text)
                self.assertIn("cargo llvm-cov report", text)
                self.assertNotIn("cargo llvm-cov --locked --lcov", text)
                self.assertNotIn("cargo llvm-cov --locked --json", text)

    def test_release_builds_are_locked(self):
        text = (SCRIPT.parents[1] / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("cargo build --release --locked", text)
        self.assertIn("wasm-pack build --release --target web -- --locked", text)
        self.assertIn("cargo install wasm-pack --locked", text)
        self.assertIn("libudev-dev", text)

    def test_independent_platform_gates_preserve_strict_failures(self):
        root = SCRIPT.parents[1]
        text = (root / ".github/workflows/rust-checks.yml").read_text(encoding="utf-8")
        for command in [
            "cargo check --locked --all-targets",
            "cargo test --locked --lib",
            "cargo check --locked --lib --target wasm32-unknown-unknown",
            "cargo fmt -- --check",
            "cargo clippy --locked --all-targets -- -D warnings",
        ]:
            self.assertIn(command, text)
        self.assertIn("fail-fast: false", text)
        self.assertIn("uses: ./.github/actions/setup-rust", text)
        self.assertIn('KEPLER_REQUIRE_GPU_TESTS: "1"', text)
        self.assertIn("mesa-vulkan-drivers", text)
        self.assertNotIn("continue-on-error", text)
        self.assertNotIn("--allow", text)
        self.assertNotIn("cargo check --locked --all-targets --target wasm32", text)

    def test_all_rust_workflows_resolve_repository_compiler_before_tools(self):
        root = SCRIPT.parents[1]
        for name in [
            "gpu-tests.yml",
            "coverage.yml",
            "coverage-gate.yml",
            "release.yml",
        ]:
            with self.subTest(workflow=name):
                text = (root / ".github/workflows" / name).read_text(encoding="utf-8")
                setup = text.find("uses: ./.github/actions/setup-rust")
                self.assertGreaterEqual(setup, 0)
                self.assertNotIn("dtolnay/rust-toolchain@", text)
                self.assertNotIn("actions-rs/toolchain@", text)
                self.assertNotIn("toolchain: nightly", text)
                self.assertNotIn("toolchain: stable", text)
                self.assertNotIn("1.98.0", text)
                if name in {"coverage.yml", "coverage-gate.yml"}:
                    self.assertIn(
                        "CARGO_LLVM_COV_VERSION: ${{ steps.rust.outputs.coverage-cli-version }}",
                        text,
                    )
                    normalized = " ".join(text.replace("\\\n", " ").split())
                    self.assertIn(
                        'cargo install cargo-llvm-cov --version "$CARGO_LLVM_COV_VERSION" --locked',
                        normalized,
                    )
                for marker in ["cargo install", "Swatinem/rust-cache@"]:
                    if marker in text:
                        self.assertLess(setup, text.index(marker))


if __name__ == "__main__":
    unittest.main()
