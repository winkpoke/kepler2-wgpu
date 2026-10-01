# Pull request checklist

Keep one requested purpose per change and preserve unrelated edits.
Use Conventional Commits (`type(scope): description`); do not commit, push, or
merge unless the user requests it.

## Before review

- Describe changed behavior and scope, including public API/format compatibility.
- Link the approved OpenSpec change for significant contract/architecture changes.
- Add the relevant regression and run checks from [BUILD.md](BUILD.md).
- Record commands actually run, failures, and unavailable/manual checks separately.
  Existing failures do not justify deleting tests or weakening quality gates.
- Update current docs/specs when contracts change and
  [the changelog](../CHANGELOG.md) for user-visible changes.
- Review public re-exports, native/WASM guards, signed-data precision, uniform
  layout, and GPU device/attachment-format ownership when those areas are touched.
- Identify incomplete deployment/browser/device work. Leave its tasks unchecked.

## Suggested PR description

```markdown
## Summary and scope
## Compatibility / affected requirements
## Verification (commands and results)
## Limitations / outstanding manual work
## Related issue or OpenSpec change
```

Reviewers check behavior against requirements and executable evidence, not merely
style, test counts, or a claimed coverage threshold. Breaking changes need an
explicit migration/rollback decision. Archive a change only after its completion
criteria are met; see [the OpenSpec workflow](OPENSPEC.md).
