# Pull Request Guidelines

**Last Updated**: 2025-01-15

## Commit Message Format

Follow conventional commits: `type(scope): description`

### Types

- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code restructuring without behavior change
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `docs`: Documentation changes
- `style`: Code formatting (no logic change)
- `chore`: Maintenance tasks

### Examples

```
feat(rendering): add isosurface extraction for mesh views
fix(mpr): resolve slice position calculation error
refactor(view): extract common view logic into trait
perf(volume): optimize texture upload for large datasets
test(dicom): add validation tests for DICOM parsing
docs(agents): restructure agent documentation
style(rustfmt): apply cargo fmt to entire codebase
chore(deps): update wgpu to 23.0
```

## PR Requirements

### Scope

- ✅ Minimal, well-scoped changes
- ✅ Single purpose per PR (no multiple unrelated features)
- ✅ Clear description of what and why

### Code Quality

- ✅ Update `src/lib.rs` re-exports if public API changed
- ✅ Add tests for new functionality
- ✅ Ensure both native (`cargo build`) and WASM (`wasm-pack build`) pass
- ✅ Run `cargo test` and ensure all tests pass
- ✅ Reference related issues in PR description
- ✅ Check platform-gated code compiles on both targets

### Documentation

- ✅ Update inline documentation for new/changed APIs
- ✅ Update relevant doc files if architecture/behavior changes
- ✅ Document any breaking changes

## Pre-Submit Checklist

Before creating a PR, verify:

- [ ] Code formatted with `cargo fmt`
- [ ] No warnings from `cargo clippy`
- [ ] All tests pass (`cargo test`)
- [ ] Native build succeeds (`cargo build --release`)
- [ ] WASM build succeeds (`wasm-pack build --target web`)
- [ ] Public API changes reflected in `src/lib.rs`
- [ ] Documentation updated (if applicable)
- [ ] Breaking changes documented in PR description

## PR Description Template

```markdown
## Summary
[Brief description of changes]

## Changes
- [ ] Add/change/fix
- [ ] Another change

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Refactor
- [ ] Documentation

## Testing
[Describe how you tested this change]

## Checklist
- [ ] Tests pass locally
- [ ] Code formatted
- [ ] No clippy warnings
- [ ] Native build succeeds
- [ ] WASM build succeeds
- [ ] Updated re-exports (if public API changed)

## Related Issues
Closes #(issue number)
Related to #(issue number)
```

## Breaking Changes

If your PR introduces breaking changes:

1. Clearly mark in title: `[BREAKING]` or in description
2. Document migration path
3. Update all affected code in the same PR
4. Update version in `Cargo.toml` if publishing crate
5. Add deprecation warnings for removed APIs (when possible)

## Examples

### Bug Fix PR

```
feat(mpr): fix slice position calculation error

## Summary
Fixed incorrect slice position calculation when volume spacing is non-uniform.

## Changes
- Fixed transform matrix in `MprView::calculate_slice_position`
- Added regression test for non-uniform spacing

## Type
- [x] Bug fix

## Testing
- Added test with non-uniform spacing volumes
- Manually verified slice positioning in UI

## Checklist
- [x] Tests pass locally
- [x] Code formatted
- [x] No clippy warnings
- [x] Native build succeeds
- [x] WASM build succeeds

## Related Issues
Closes #123
```

### Feature PR

```
feat(rendering): add isosurface extraction for mesh views

## Summary
Added isosurface extraction capability for 3D mesh rendering of CT volumes.

## Changes
- Implemented Marching Cubes algorithm in `src/rendering/mesh/isosurface.rs`
- Added `MeshView::set_isovalue()` method
- Added UI controls for isovalue adjustment
- Performance: optimized with lookup tables

## Type
- [x] New feature

## Testing
- Unit tests for isosurface extraction
- Visual verification with sample DICOM datasets
- Performance profiling: < 100ms for 512^3 volumes

## Checklist
- [x] Tests pass locally
- [x] Code formatted
- [x] No clippy warnings
- [x] Native build succeeds
- [x] WASM build succeeds
- [x] Updated re-exports in `src/lib.rs`
- [x] Added documentation

## Related Issues
Closes #45
Related to #12
```

### Refactor PR

```
refactor(view): extract common view logic into trait

## Summary
Extracted common rendering logic into `View` trait to reduce code duplication.

## Changes
- Created `View` trait with `render()` and `resize()` methods
- Refactored `MprView`, `MipView`, `MeshView` to implement trait
- Removed duplicate code from view implementations
- No behavior changes

## Type
- [x] Refactor

## Testing
- All existing tests pass
- Verified rendering unchanged in UI
- Added trait implementation tests

## Checklist
- [x] Tests pass locally
- [x] Code formatted
- [x] No clippy warnings
- [x] Native build succeeds
- [x] WASM build succeeds

## Related Issues
Related to #67
```

## Review Process

### For Reviewers

When reviewing a PR:

1. **Scope Check**: Is the change minimal and focused?
2. **Tests**: Are tests added/updated?
3. **Documentation**: Is inline documentation updated?
4. **Breaking Changes**: Are they documented and necessary?
5. **Code Quality**: Does it follow project conventions?
6. **Performance**: Any performance implications?
7. **Platform**: Does it work on both native and WASM?

### For Authors

When responding to review feedback:

1. Address each comment individually
2. Explain your rationale if disagreeing
3. Update tests/ documentation if needed
4. Mark resolved comments as done
5. Request re-review after significant changes

## Common Review Feedback

### Testing Concerns

> "Please add tests for this new functionality"

Action: Add unit tests in `tests/` or alongside module. Follow test patterns in `CONVENTIONS.md`.

### Documentation Concerns

> "Please add documentation for this public API"

Action: Add `///` documentation comments explaining:
- What the function does
- Parameters and their types
- Return value and possible errors
- Usage examples if helpful

### Performance Concerns

> "This looks expensive for large datasets"

Action:
- Profile the code
- Consider optimizing or caching
- Document performance characteristics
- Add performance tests if critical

### Breaking Change Concerns

> "This changes the public API"

Action:
- Document breaking change in PR description
- Add migration guide if needed
- Consider deprecation warnings
- Update all internal uses

## Landing Process

### After Approval

1. Rebase if requested (squash/rebase as instructed)
2. Ensure all CI checks pass
3. Update PR description with final summary
4. Request final review if changes were made
5. Wait for maintainer to merge

### Post-Merge

1. Delete your branch after merge
2. Verify in next release or environment
3. Monitor for reported issues

## Related Documentation

- **Quick Reference**: `QUICK_REFERENCE.md` - Common git/PR commands
- **Conventions**: `CONVENTIONS.md` - Coding standards
- **Build**: `BUILD.md` - Testing and quality checks
- **Pitfalls**: `PITFALLS.md` - Common mistakes to avoid
