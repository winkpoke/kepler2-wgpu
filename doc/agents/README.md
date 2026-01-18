# AI Agent Documentation

Welcome! This is the central hub for AI agents working on Kepler2-WGPU.

## Quick Start

**New to the project?** Start here:
1. Read `QUICK_REFERENCE.md` for common commands and patterns
2. Read `ARCHITECTURE.md` to understand the project structure
3. Read `CONVENTIONS.md` for coding standards

**Need to implement a feature?** Use OpenSpec:
- See `OPENSPEC.md` for spec-driven development workflow

**Need to fix a bug?** Skip to:
- `CONVENTIONS.md` - Coding patterns
- `PITFALLS.md` - Common mistakes
- `RENDERING.md` - If it's GPU-related

## Documentation Guide

### Core Documentation

| File | Purpose | When to Read |
|------|---------|--------------|
| `QUICK_REFERENCE.md` | One-page cheat sheet | Quick lookup, daily tasks |
| `ARCHITECTURE.md` | Project structure and modules | Understanding codebase layout |
| `CONVENTIONS.md` | Coding standards and patterns | Writing code |
| `BUILD.md` | Build and test commands | Development workflow |
| `RENDERING.md` | GPU and rendering patterns | Graphics work |
| `PITFALLS.md` | Common anti-patterns | Avoiding mistakes |
| `PR_GUIDELINES.md` | Contribution workflow | Creating PRs |
| `OPENSPEC.md` | Spec-driven development | Proposing changes |

### When to Use Each Guide

#### Getting Started
- **First time**: Read `QUICK_REFERENCE.md` → `ARCHITECTURE.md` → `CONVENTIONS.md`
- **Daily reference**: `QUICK_REFERENCE.md`
- **Onboarding new agent**: Share `ARCHITECTURE.md` and `CONVENTIONS.md`

#### Implementing Features
- **Small changes**: `CONVENTIONS.md` → write code → test
- **Medium features**: `OPENSPEC.md` (create proposal) → implement → `PR_GUIDELINES.md`
- **Major features**: `OPENSPEC.md` → `ARCHITECTURE.md` → `CONVENTIONS.md` → implement

#### Fixing Bugs
- **Quick fix**: `PITFALLS.md` (check for common issues) → fix → test
- **Deep dive**: `ARCHITECTURE.md` → relevant module docs → fix
- **GPU-related**: `RENDERING.md` → fix

#### Code Review
- **Reviewing PRs**: `PR_GUIDELINES.md` → `CONVENTIONS.md` → `PITFALLS.md`
- **Checking compliance**: `CONVENTIONS.md` → `PITFALLS.md`

#### Performance Work
- **GPU optimization**: `RENDERING.md`
- **General optimization**: `ARCHITECTURE.md` → `CONVENTIONS.md`

## Common Workflows

### Workflow 1: Fix a Bug

```bash
1. Identify bug location (use ARCHITECTURE.md)
2. Check PITFALLS.md for common issues
3. Review CONVENTIONS.md for patterns
4. Fix the bug
5. Run tests (see BUILD.md)
6. Create PR (see PR_GUIDELINES.md)
```

### Workflow 2: Add a Feature

```bash
1. Check if feature exists (openspec list --specs)
2. Create proposal (OPENSPEC.md)
3. Wait for approval
4. Implement following tasks.md
5. Test (BUILD.md)
6. Update tests (CONVENTIONS.md)
7. Create PR (PR_GUIDELINES.md)
8. Archive after deployment (OPENSPEC.md)
```

### Workflow 3: Debug GPU Issues

```bash
1. Read RENDERING.md for patterns
2. Check PITFALLS.md for common GPU issues
3. Enable validation: KEPLER_WGPU_VALIDATION=true
4. Test with different backends: KEPLER_WGPU_BACKEND=vulkan
5. Fix and test
```

### Workflow 4: Onboard New Team Member

```bash
1. Share QUICK_REFERENCE.md
2. Walk through ARCHITECTURE.md
3. Review CONVENTIONS.md together
4. Practice with a small bug fix
5. Gradually introduce OPENSPEC.md workflow
```

## Navigation Tips

### By Topic

- **Architecture**: `ARCHITECTURE.md`
- **Coding**: `CONVENTIONS.md`, `PITFALLS.md`
- **Rendering**: `RENDERING.md`
- **Build/Test**: `BUILD.md`, `QUICK_REFERENCE.md`
- **Workflow**: `PR_GUIDELINES.md`, `OPENSPEC.md`

### By Experience Level

- **New agent**: QUICK_REFERENCE.md → ARCHITECTURE.md → CONVENTIONS.md
- **Experienced agent**: Jump to relevant topic, use QUICK_REFERENCE.md for lookup
- **Advanced agent**: OPENSPEC.md for architecture changes, ARCHITECTURE.md for deep dives

### By Task Type

- **Quick fix**: PITFALLS.md → QUICK_REFERENCE.md
- **Implementation**: CONVENTIONS.md → BUILD.md
- **Design**: ARCHITECTURE.md → OPENSPEC.md
- **Review**: PR_GUIDELINES.md → CONVENTIONS.md

## Keeping Documentation Updated

When you learn something new:
1. Update relevant documentation files
2. Add examples to `QUICK_REFERENCE.md`
3. Add anti-patterns to `PITFALLS.md`
4. Update patterns in `CONVENTIONS.md`

## Related Resources

### Project Documentation

- `README.md` - Project overview
- `doc/architecture/` - Detailed architecture docs
- `doc/rendering/` - Rendering deep dives
- `.github/copilot-instructions.md` - External agent instructions

### External Documentation

- [WGPU Documentation](https://docs.rs/wgpu)
- [Glam Documentation](https://docs.rs/glam)
- [Rust Book](https://doc.rust-lang.org/book/)

## Getting Help

If you can't find what you need:
1. Check QUICK_REFERENCE.md first
2. Search across all files: `rg "keyword" doc/agents/`
3. Ask in the issue tracker
4. Check external documentation links

## Feedback

Documentation is living. If something is unclear or missing:
- File an issue with tag `documentation`
- Propose improvements via PR
- Update relevant files directly for small fixes

---

**Remember**: Good documentation saves time. Keep it accurate, keep it current.
