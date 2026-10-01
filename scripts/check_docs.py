#!/usr/bin/env python3
"""Check current guide links and pending OpenSpec delta targets; no dependencies.

Historical reports and archives are deliberately outside the current-guide scope.
Checks inline Markdown link targets (not anchors or remote URLs), ignoring fenced
examples. OpenSpec remains responsible for requirement/scenario syntax validation.
"""

from __future__ import annotations

import argparse
import re
from collections.abc import Iterable
from pathlib import Path
from urllib.parse import unquote, urlsplit

CURRENT_GUIDES = (
    "AGENTS.md", "CLINE.md", ".github/copilot-instructions.md",
    ".trae/rules/project_rules.md", "doc/README.md", "doc/agents/*.md",
    "doc/architecture/ARCHITECTURE.md", "doc/rendering/volume-encoding.md",
    "doc/testing/*.md", "openspec/specs/*/spec.md",
)
LINK = re.compile(r"!?\[[^\]\n]*\]\(([^)\n]+)\)")
REQUIREMENT = re.compile(r"^### Requirement:\s*(.+?)\s*$", re.MULTILINE)
OPERATION = re.compile(r"^## (ADDED|MODIFIED|REMOVED|RENAMED) Requirements\s*$", re.MULTILINE)
RENAME = re.compile(r"- FROM: `### Requirement: (.+?)`\s*\n- TO: `### Requirement: (.+?)`")


def prose_lines(text: str):
    """Yield numbered non-fenced lines without interpreting illustrative links."""
    fence = ""
    for number, line in enumerate(text.splitlines(), 1):
        match = re.match(r"^\s*(`{3,}|~{3,})", line)
        if match:
            delimiter = match.group(1)
            if not fence:
                fence = delimiter
            elif delimiter[0] == fence[0] and len(delimiter) >= len(fence):
                fence = ""
            continue
        if not fence:
            yield number, line


def check_links(root: Path, files: Iterable[Path]) -> list[str]:
    root = root.resolve()
    issues = []
    for source in files:
        source = source.resolve()
        if root not in source.parents:
            issues.append(f"source outside repository: {source}")
            continue
        for number, line in prose_lines(source.read_text(encoding="utf-8")):
            for match in LINK.finditer(line):
                destination = match.group(1).strip().split(' "', 1)[0].strip("<>")
                url = urlsplit(destination)
                if url.scheme or url.netloc or not url.path:
                    continue
                path = unquote(url.path)
                target = (root / path.lstrip("/") if path.startswith("/") else source.parent / path).resolve()
                location = f"{source.relative_to(root)}:{number}"
                if root not in (target, *target.parents):
                    issues.append(f"{location}: link outside repository: {destination}")
                elif not target.exists():
                    issues.append(f"{location}: missing link target: {destination}")
    return issues


def check_deltas(root: Path) -> list[str]:
    issues = []
    changes = root / "openspec/changes"
    for delta in sorted(changes.glob("*/specs/*/spec.md")):
        if delta.relative_to(changes).parts[0] == "archive":
            continue
        baseline = root / "openspec/specs" / delta.parent.name / "spec.md"
        names = set(REQUIREMENT.findall(baseline.read_text(encoding="utf-8"))) if baseline.exists() else set()
        text = delta.read_text(encoding="utf-8")
        sections = list(OPERATION.finditer(text))
        blocks = [(section.group(1), text[section.end():sections[index + 1].start() if index + 1 < len(sections) else len(text)]) for index, section in enumerate(sections)]
        location = str(delta.relative_to(root))
        for operation, block in blocks:
            if operation != "RENAMED":
                continue
            for old, new in RENAME.findall(block):
                if old not in names:
                    issues.append(f"{location}: RENAMED source does not exist: {old}")
                elif new in names and new != old:
                    issues.append(f"{location}: RENAMED target already exists: {new}")
                else:
                    names.remove(old)
                    names.add(new)
        for operation, block in blocks:
            for name in REQUIREMENT.findall(block):
                if operation in ("MODIFIED", "REMOVED") and name not in names:
                    issues.append(f"{location}: {operation} target does not exist: {name}")
                elif operation == "ADDED" and name in names:
                    issues.append(f"{location}: ADDED target already exists: {name}")
    return issues


def check(root: Path) -> list[str]:
    root = root.resolve()
    if not root.is_dir():
        return [f"Documentation root does not exist or is not a directory: {root}"]
    if not (root / "AGENTS.md").is_file():
        return [f"Documentation root is missing required AGENTS.md: {root}"]
    files = sorted({path for pattern in CURRENT_GUIDES for path in root.glob(pattern) if path.is_file()})
    return check_links(root, files) + check_deltas(root)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args(argv)
    issues = check(args.root)
    for issue in issues:
        print(issue)
    print(f"Documentation checks: {len(issues)} issue(s)")
    return 1 if issues else 0


if __name__ == "__main__":
    raise SystemExit(main())
