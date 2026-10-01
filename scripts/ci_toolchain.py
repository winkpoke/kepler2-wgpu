#!/usr/bin/env python3
"""Resolve repository Rust setup without floating defaults (Python >=3.11)."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

RELEASE = re.compile(r"[1-9][0-9]*\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
COV_VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
TOKEN = re.compile(r"[a-z0-9][a-z0-9_-]*")


def read_toml(path: Path) -> dict:
    try:
        with path.open("rb") as file:
            return tomllib.load(file)
    except (OSError, ValueError) as error:
        raise ValueError(f"Cannot read {path.name}: {error}") from error


def token_list(value: object, field: str, required: set[str]) -> str:
    if (
        not isinstance(value, list)
        or not value
        or any(not isinstance(item, str) or not TOKEN.fullmatch(item) for item in value)
        or len(set(value)) != len(value)
        or not required.issubset(value)
    ):
        raise ValueError(
            f"toolchain.{field} must be a unique token list including {sorted(required)}"
        )
    return ",".join(value)


def resolve(root: Path) -> dict[str, str]:
    toolchain = read_toml(root / "rust-toolchain.toml").get("toolchain")
    if not isinstance(toolchain, dict):
        raise ValueError("Missing [toolchain] table")
    channel = toolchain.get("channel")
    if not isinstance(channel, str) or not RELEASE.fullmatch(channel):
        raise ValueError(
            "toolchain.channel must be an exact stable release (major.minor.patch)"
        )
    if toolchain.get("profile") != "minimal":
        raise ValueError("toolchain.profile must be minimal")
    return {
        "toolchain": channel,
        "components": token_list(
            toolchain.get("components"),
            "components",
            {"rustfmt", "clippy", "llvm-tools-preview"},
        ),
        "targets": token_list(
            toolchain.get("targets"), "targets", {"wasm32-unknown-unknown"}
        ),
    }


def resolve_coverage_version(root: Path) -> str:
    versions = read_toml(root / "scripts/ci-tool-versions.toml")
    coverage = versions.get("coverage")
    if not isinstance(coverage, dict):
        raise ValueError("Missing [coverage] table in scripts/ci-tool-versions.toml")
    version = coverage.get("cargo-llvm-cov")
    if not isinstance(version, str) or not COV_VERSION.fullmatch(version):
        raise ValueError(
            "coverage.cargo-llvm-cov must be an exact CLI release version (major.minor.patch)"
        )
    return version


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[1]
    )
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--coverage-output", type=Path)
    parser.add_argument("--print-coverage-version", action="store_true")
    args = parser.parse_args(argv)
    try:
        values = resolve(args.root)
        coverage_version = (
            resolve_coverage_version(args.root)
            if args.coverage_output or args.print_coverage_version
            else None
        )
        if args.github_output:
            with args.github_output.open("a", encoding="utf-8") as file:
                for key, value in values.items():
                    file.write(f"{key}={value}\n")
        if args.coverage_output:
            with args.coverage_output.open("a", encoding="utf-8") as file:
                file.write(f"coverage-cli-version={coverage_version}\n")
    except (ValueError, OSError) as error:
        print(f"Rust setup configuration error: {error}", file=sys.stderr)
        return 2
    if args.print_coverage_version:
        print(coverage_version)
    else:
        print(json.dumps(values, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
