"""Regression tests for the documentation checks (stdlib only)."""

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check_docs.py"
SPEC = importlib.util.spec_from_file_location("check_docs", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
check_docs = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check_docs)


class DocumentationChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def put(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        return target

    def test_relative_links_and_directory_links_exist(self):
        self.put("doc/target.md", "# Target\n")
        source = self.put("AGENTS.md", "[doc](doc/target.md#target) [dir](doc/)\n")
        self.assertEqual(check_docs.check_links(self.root, [source]), [])

    def test_missing_link_reports_source_and_line(self):
        source = self.put("doc/guide.md", "# Guide\n[bad](missing.md)\n")
        self.assertIn("doc/guide.md:2", check_docs.check_links(self.root, [source])[0])

    def test_external_links_and_fenced_examples_are_not_local_dependencies(self):
        source = self.put("AGENTS.md", "[web](https://example.com/a) [mail](mailto:a@example.com)\n```md\n[example](planned.md)\n```\n~~~md\n[example](other.md)\n~~~\n")
        self.assertEqual(check_docs.check_links(self.root, [source]), [])

    def test_encoded_spaces_and_repo_absolute_links(self):
        self.put("doc/a b.md", "# Target\n")
        source = self.put("doc/guide.md", "[one](a%20b.md) [two](/doc/a%20b.md)\n")
        self.assertEqual(check_docs.check_links(self.root, [source]), [])

    def test_link_cannot_escape_repository(self):
        source = self.put("AGENTS.md", "[outside](../outside.md)\n")
        self.assertIn("outside repository", check_docs.check_links(self.root, [source])[0])

    def test_nonexistent_root_does_not_pass_with_an_empty_scan(self):
        self.assertIn("root", check_docs.check(self.root / "missing")[0])

    def test_empty_directory_is_not_a_valid_documentation_root(self):
        self.assertIn("AGENTS.md", check_docs.check(self.root)[0])

    def test_external_source_is_rejected_before_reading(self):
        source = self.root.parent / "outside.md"
        self.assertIn("outside repository", check_docs.check_links(self.root, [source])[0])

    def delta(self, content, baseline="### Requirement: Existing\n"):
        self.put("openspec/specs/demo/spec.md", baseline)
        self.put("openspec/changes/update-demo/specs/demo/spec.md", content)
        return check_docs.check_deltas(self.root)

    def test_modified_and_removed_targets_must_exist(self):
        for operation in ("MODIFIED", "REMOVED"):
            with self.subTest(operation=operation):
                issues = self.delta(f"## {operation} Requirements\n### Requirement: Missing\n")
                self.assertIn("Missing", issues[0])

    def test_existing_modified_target_and_new_added_target_pass(self):
        self.assertEqual(self.delta("## MODIFIED Requirements\n### Requirement: Existing\n## ADDED Requirements\n### Requirement: New\n"), [])

    def test_added_target_cannot_duplicate_baseline(self):
        self.assertIn("already exists", self.delta("## ADDED Requirements\n### Requirement: Existing\n")[0])

    def test_renamed_target_can_be_modified(self):
        delta = "## RENAMED Requirements\n- FROM: `### Requirement: Existing`\n- TO: `### Requirement: New`\n## MODIFIED Requirements\n### Requirement: New\n"
        self.assertEqual(self.delta(delta), [])

    def test_rename_requires_existing_source(self):
        delta = "## RENAMED Requirements\n- FROM: `### Requirement: Missing`\n- TO: `### Requirement: New`\n"
        self.assertIn("Missing", self.delta(delta)[0])

    def test_archives_are_not_treated_as_pending_deltas(self):
        self.put("openspec/changes/archive/old/specs/demo/spec.md", "## MODIFIED Requirements\n### Requirement: Missing\n")
        self.assertEqual(check_docs.check_deltas(self.root), [])

    def test_cli_exit_status_is_nonzero_on_drift(self):
        self.put("AGENTS.md", "[bad](missing.md)\n")
        result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(self.root)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("missing.md", result.stdout)


if __name__ == "__main__":
    unittest.main()
