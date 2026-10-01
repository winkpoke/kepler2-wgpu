"""Tests for machine-readable native line coverage and fail-closed gates."""

import argparse
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "coverage_summary.py"
SPEC = importlib.util.spec_from_file_location("coverage_summary", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
coverage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(coverage)


class CoverageSummaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        for name in ("src/critical.rs", "src/other.rs"):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("// synthetic coverage fixture\n", encoding="utf-8")

    def report(self, critical=(9, 10), other=(1, 10)):
        return {"data": [{"files": [
            {"filename": str(self.root / "src/critical.rs"), "summary": {"lines": {"covered": critical[0], "count": critical[1]}}},
            {"filename": "src/other.rs", "summary": {"lines": {"covered": other[0], "count": other[1]}}},
            {"filename": "tests/helper.rs", "summary": {"lines": {"covered": 100, "count": 100}}},
        ]}]}

    def summarize(self, report=None):
        return coverage.summarize(report or self.report(), self.root, ["src/critical.rs"])

    def test_medical_and_overall_have_distinct_denominators(self):
        result = self.summarize()
        self.assertEqual(result["medical"]["percent"], 90.0)
        self.assertEqual(result["overall"]["percent"], 50.0)
        self.assertEqual(result["overall"]["count"], 20)

    def test_percentages_are_weighted_by_lines_not_averaged(self):
        result = self.summarize(self.report(critical=(1, 1), other=(0, 9)))
        self.assertEqual(result["overall"]["percent"], 10.0)

    def test_missing_medical_file_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "missing medical"):
            coverage.summarize(self.report(), self.root, ["src/missing.rs"])

    def test_empty_medical_scope_is_an_error(self):
        with self.assertRaises(ValueError):
            coverage.summarize(self.report(), self.root, [])

    def test_missing_line_data_and_invalid_counts_are_errors(self):
        for invalid in ({}, {"covered": 11, "count": 10}, {"covered": -1, "count": 10}, {"covered": 1.5, "count": 10}, {"covered": True, "count": 10}):
            with self.subTest(invalid=invalid):
                report = self.report()
                report["data"][0]["files"][0]["summary"]["lines"] = invalid
                with self.assertRaises(ValueError):
                    self.summarize(report)

    def test_empty_or_duplicate_measurements_are_errors(self):
        with self.assertRaises(ValueError):
            self.summarize({"data": []})
        report = self.report()
        report["data"][0]["files"].append(report["data"][0]["files"][0])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.summarize(report)

    def test_external_sources_do_not_inflate_coverage(self):
        report = self.report()
        report["data"][0]["files"].append({"filename": str(self.root.parent / "external.rs")})
        self.assertEqual(self.summarize(report)["overall"]["percent"], 50.0)

    def test_thresholds_fail_independently_and_boundary_passes(self):
        result = self.summarize()
        self.assertEqual(coverage.gate(result, 50, 90), [])
        self.assertIn("overall", coverage.gate(result, 51, 90)[0])
        self.assertIn("medical", coverage.gate(result, 50, 91)[0])

    def test_cli_exports_summary_and_returns_threshold_failure(self):
        report = self.root / "report.json"
        scope = self.root / "scope.json"
        output = self.root / "summary.json"
        report.write_text(json.dumps(self.report()), encoding="utf-8")
        scope.write_text(json.dumps({"medical_files": ["src/critical.rs"]}), encoding="utf-8")
        result = subprocess.run([sys.executable, str(SCRIPT), str(report), "--root", str(self.root), "--scope", str(scope), "--output", str(output), "--overall-min", "51"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(output.read_text())["metric"], "native-source-line")
        self.assertIn("overall", result.stdout)

    def test_cli_rejects_nonfinite_threshold(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "missing.json", "--overall-min", "nan"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)

    def test_main_valid_export_and_threshold_failure(self):
        report = self.root / "report.json"
        scope = self.root / "scope.json"
        output = self.root / "nested/summary.json"
        report.write_text(json.dumps(self.report()), encoding="utf-8")
        scope.write_text(json.dumps({"medical_files": ["src/critical.rs"]}), encoding="utf-8")
        args = [str(report), "--root", str(self.root), "--scope", str(scope), "--output", str(output)]
        with redirect_stdout(io.StringIO()):
            self.assertEqual(coverage.main(args), 0)
            self.assertEqual(coverage.main(args + ["--overall-min", "51"]), 1)
        self.assertTrue(output.exists())

    def test_main_missing_invalid_json_and_invalid_scope_fail_closed(self):
        report = self.root / "report.json"
        args = [str(report), "--root", str(self.root)]
        with redirect_stderr(io.StringIO()):
            self.assertEqual(coverage.main(args), 2)
            report.write_text("not JSON", encoding="utf-8")
            self.assertEqual(coverage.main(args), 2)
            report.write_text("[]", encoding="utf-8")
            scope = self.root / "doc/testing/coverage-scope.json"
            scope.parent.mkdir(parents=True)
            scope.write_text("{}", encoding="utf-8")
            self.assertEqual(coverage.main(args), 2)

    def test_report_structure_zero_denominator_and_scope_errors(self):
        reports = [{"data": [None]}, {"data": [{"files": [None]}]}, self.report(critical=(0, 0))]
        for report in reports:
            with self.subTest(report=report), self.assertRaises(ValueError):
                self.summarize(report)
        for scope in (["tests/helper.rs"], ["src/critical.rs", "src/critical.rs"]):
            with self.subTest(scope=scope), self.assertRaises(ValueError):
                coverage.summarize(self.report(), self.root, scope)

    def test_percentage_rejects_invalid_thresholds(self):
        for value in ("-1", "101", "inf", "nan", "not-a-number"):
            with self.subTest(value=value), self.assertRaises((ValueError, argparse.ArgumentTypeError)):
                coverage.percentage(value)


if __name__ == "__main__":
    unittest.main()
