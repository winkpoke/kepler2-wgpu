# Coverage measurement and evidence

**Status:** Current measurement procedure. Targets are intended quality gates,
not proof of clinical safety, regulatory conformity, or achieved coverage.
The old 24% assessment and proposed branch/exclusion examples are
[historical](../archive/agent-guidance-2026-10-01/coverage_methodology.md).

## Metrics and scope

The executable CI gate reports **native source-line coverage**, not branch coverage.
It consumes LLVM JSON with explicit covered/count denominators through
[coverage_summary.py](../../scripts/coverage_summary.py). It does not grep a human
summary, average file percentages, rerun tests for every threshold, or call the
same overall number medical-path coverage.

- Overall scope: measured Rust files under the checkout's `src/`; external crates
  and integration-test/helper files are excluded from this aggregate.
- Medical scope: the explicit files in [coverage-scope.json](coverage-scope.json),
  covering DICOM identity/data, volume/geometry, window/level, and MPR. Every named
  file must have line-count data; missing/malformed inputs fail closed.
- Percent = sum(covered lines) / sum(measured executable lines) × 100. The summary
  retains file lists and per-file counts so the aggregate can be audited.
- Inline `#[cfg(test)]` code can still be counted within source files; this is not
  a production-only branch or function measurement. Uncompiled browser branches
  are not in a native denominator. Record these limitations with results.
- A missing instrumented module is not automatically “fully covered.” The medical
  subset is checked for completeness, but an overall report still reflects the
  compiled/measured native configuration rather than every source/platform path.

Existing initial thresholds remain **45% overall** and **80% medical**. The baseline
spec's staged targets remain intended policy; the active coverage change proposes
further milestones, including 65% for phase three. This tooling change does not
claim any milestone is reached or change a threshold to fit current results.

## Reproducible export and gate

Install the compatible `cargo-llvm-cov` CLI release declared in
[`scripts/ci-tool-versions.toml`](../../scripts/ci-tool-versions.toml); the Cargo
dev-dependency is not CLI installation or a version pin. The resolver and native
CI scripts require Python >=3.11. From the repository root:

```bash
cargo install cargo-llvm-cov \
  --version "$(python3.11 scripts/ci_toolchain.py --print-coverage-version)" --locked
cargo llvm-cov --version
rustc -Vv
rustup show active-toolchain
git rev-parse HEAD
KEPLER_REQUIRE_GPU_TESTS=1 cargo llvm-cov test --locked \
  --json --output-path coverage/coverage.json
python3.11 scripts/coverage_summary.py coverage/coverage.json \
  --output coverage/summary.json --overall-min 45 --medical-min 80
```

Record compiler/CLI versions, features, adapter/backend, scope, exact test command,
pass/fail/ignored results, and Git revision alongside the artifact. The CI workflows
run one instrumented test command and derive JSON/LCOV/HTML reports from its saved
coverage data; on test or threshold failure they retain logs, outcome, raw report
when available, and provenance for diagnosis. A missing artifact is not a successful
measurement. Their adapter record is from configured software Vulkan (lavapipe),
not a claim of browser or clinical validation.

Exit status: 0 = gates passed; 1 = valid measurement below a threshold;
2 = missing/invalid measurement. The JSON summary is emitted even when thresholds
fail. The coverage CI job also exports LCOV from the same instrumented run for its
existing PR comment. CI is configured evidence, not a claim it ran locally.

Inspect suspicious mapping/toolchain failures: a report showing zero unit-code
coverage despite executed tests needs investigation, not fabricated metrics.
[The prior render verification](../../openspec/changes/optimize-render-resource-lifetimes/verification.md)
records one such mapping limitation and explicitly scoped results.

## Branch coverage, trends, and limitations

Branch coverage is a separate desired metric. Do not relabel default line/region
output as branch coverage. A branch run must explicitly enable compatible branch
instrumentation/export and report its own branch covered/count denominator; the
current gate does not enforce a branch target. Native/WASM or different feature
sets require separate measurements, not an average of incompatible reports.

Trend enforcement requires a previous **comparable JSON artifact** with the same
metric, file scope, features, and toolchain. The old scalar `coverage_report.txt`
does not contain that provenance and is not a trustworthy baseline. Until a
comparable artifact is supplied, CI explicitly reports trend detection as pending
rather than making a misleading comparison with `HEAD~1` or zero. This remains
outstanding coverage-plan work, not a passed regression gate.

Test execution, line/branch coverage, algorithm correctness, browser behavior,
performance, and clinical validation are separate evidence types. Scoped results
and synthetic parser fixtures cannot establish all of them.
