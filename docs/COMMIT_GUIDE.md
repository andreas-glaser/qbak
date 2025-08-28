# Commit Guide (qbak)

Follow Conventional Commits with short, focused changes. Target the `dev` branch for PRs.

## Pre‑Commit Checks

1) Review changes
```bash
git status
git diff
git diff --staged
```

2) Run code quality checks (choose one)
```bash
# Using Docker tools (no local Rust needed)
make -C tools/docker pre-commit

# Or locally
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -- --test-threads=1
```

## Commit Process

3) Stage files
```bash
git add <files>
# or interactively
git add -p
```

4) Create commit
```bash
git commit -m "<type>(<scope>): <short description>"
```

5) Verify CI
```bash
gh run list --limit 5
```
Or check: https://github.com/andreas-glaser/qbak/actions

## Commit Message Format

Types:
- feat: new feature
- fix: bug fix
- docs: documentation only
- style: formatting only (no logic changes)
- refactor: code change that neither fixes a bug nor adds a feature
- test: add or update tests
- build: build system or dependencies
- ci: CI configuration
- chore: misc maintenance

Scopes (examples):
- core, backup, config, naming, progress, signal, utils
- docker, readme, docs, ci, release

Examples:
- feat(core): add dry-run flag
- fix(utils): reject path traversal in validation
- docs(readme): add Docker quick start
- build(deps): bump clap to <4.6
- ci(release): add arm64 musl build

Rules:
- Imperative mood (“add”, not “added”)
- No trailing period
- Keep subject ≤ 72 chars; add details in body if needed

Footers:
- Use `BREAKING CHANGE:` for breaking changes in the commit body
- Reference issues with `Fixes #123` or `Refs #123`

Workflow:
- Target PRs at `dev` branch (not `main`)
- Keep commits focused; prefer multiple small commits over one large one
- Do not include AI/assistant references in commit messages

Multi‑line example:
```bash
git commit -m "fix(utils): handle symlink loop" -m "
- Detect loops via visited set
- Add tests for nested links
Fixes #123"
```
