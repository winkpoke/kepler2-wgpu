#!/usr/bin/env python3
"""Summarize LLVM JSON native source-line coverage and enforce explicit scopes.

This is not branch coverage or proof of clinical safety. Missing measurements
fail closed; tests/dependencies cannot inflate the production-source denominator.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any


def aggregate(files: dict[str, dict[str, int]]) -> dict[str, Any]:
    count = sum(value["count"] for value in files.values())
    covered = sum(value["covered"] for value in files.values())
    if count == 0:
        raise ValueError("coverage scope has no measured executable lines")
    return {"covered": covered, "count": count, "percent": covered * 100.0 / count,
            "files": sorted(files)}


def summarize(report: dict[str, Any], root: Path, medical_files: list[str]) -> dict[str, Any]:
    root = root.resolve()
    if not medical_files or any(not isinstance(name, str) or not name.startswith("src/") for name in medical_files):
        raise ValueError("medical scope must list production src/ files")
    if len(set(medical_files)) != len(medical_files):
        raise ValueError("duplicate medical scope file")
    data = report.get("data")
    if not isinstance(data, list) or not data:
        raise ValueError("LLVM report has no data")
    measured = {}
    for entry in data:
        if not isinstance(entry, dict) or not isinstance(entry.get("files"), list):
            raise ValueError("LLVM report has invalid files data")
        for file in entry["files"]:
            if not isinstance(file, dict) or not isinstance(file.get("filename"), str):
                raise ValueError("LLVM report has invalid filename")
            path = Path(file["filename"])
            path = (path if path.is_absolute() else root / path).resolve()
            try:
                relative = path.relative_to(root).as_posix()
            except ValueError:
                continue
            if not relative.startswith("src/") or not relative.endswith(".rs"):
                continue
            if relative in measured:
                raise ValueError(f"duplicate coverage file: {relative}")
            summary = file.get("summary")
            lines = summary.get("lines") if isinstance(summary, dict) else None
            if not isinstance(lines, dict):
                raise ValueError(f"missing line data: {relative}")
            count, covered = lines.get("count"), lines.get("covered")
            if type(count) is not int or type(covered) is not int or not 0 <= covered <= count:
                raise ValueError(f"invalid line counts: {relative}")
            measured[relative] = {"covered": covered, "count": count}
    missing = sorted(set(medical_files) - measured.keys())
    if missing:
        raise ValueError(f"missing medical coverage files: {', '.join(missing)}")
    medical = {name: measured[name] for name in medical_files}
    return {"metric": "native-source-line", "overall": aggregate(measured),
            "medical": aggregate(medical), "per_file": dict(sorted(measured.items()))}


def gate(result: dict[str, Any], overall_min: float, medical_min: float) -> list[str]:
    issues = []
    for name, threshold in (("overall", overall_min), ("medical", medical_min)):
        value = result[name]["percent"]
        if value < threshold:
            issues.append(f"{name} line coverage {value:.2f}% is below {threshold:g}%")
    return issues


def percentage(value: str) -> float:
    try:
        number = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("threshold must be numeric") from error
    if not math.isfinite(number) or not 0 <= number <= 100:
        raise argparse.ArgumentTypeError("threshold must be finite and between 0 and 100")
    return number


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--scope", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--overall-min", type=percentage, default=45.0)
    parser.add_argument("--medical-min", type=percentage, default=80.0)
    args = parser.parse_args(argv)
    scope_path = args.scope or args.root / "doc/testing/coverage-scope.json"
    try:
        report = json.loads(args.report.read_text(encoding="utf-8"))
        scope = json.loads(scope_path.read_text(encoding="utf-8"))
        if not isinstance(report, dict) or not isinstance(scope, dict) or not isinstance(scope.get("medical_files"), list):
            raise ValueError("report/scope must be objects with a medical_files list")
        result = summarize(report, args.root, scope["medical_files"])
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError) as error:
        print(f"Coverage measurement error: {error}", file=sys.stderr)
        return 2
    for name in ("overall", "medical"):
        value = result[name]
        print(f"{name}: {value['covered']}/{value['count']} lines ({value['percent']:.2f}%)")
    issues = gate(result, args.overall_min, args.medical_min)
    for issue in issues:
        print(issue)
    return 1 if issues else 0


if __name__ == "__main__":
    raise SystemExit(main())
