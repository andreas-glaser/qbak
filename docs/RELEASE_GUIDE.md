# Release Guide (qbak)

Simple, repeatable steps to cut a qbak release. The Release GitHub Action builds artifacts for Linux/macOS/Windows and can publish to crates.io.

## Pre‑Release

1) On `dev` branch
```bash
git checkout dev
git pull
```

2) Run checks (use Docker tools or local toolchain)
```bash
# Docker (no local Rust needed)
make -C tools/docker pre-commit

# Or locally
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -- --test-threads=1
```

3) Update version and changelog
- Edit `Cargo.toml`: `version = "X.Y.Z"`
- Update `CHANGELOG.md`: add `## [X.Y.Z] - YYYY-MM-DD` and summarize changes

4) Commit
```bash
git add -A
git commit -m "chore: prepare release vX.Y.Z"
```

## Release

5) Merge to main
```bash
git checkout main
git pull
git merge dev --no-ff -m "Merge dev for vX.Y.Z release"
```

6) Tag and push
```bash
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main --tags
```

7) GitHub Actions
- Tag push triggers the Release workflow:
  - Creates GitHub release with notes from `CHANGELOG.md`
  - Builds and uploads binaries for: Linux (glibc/musl, x86_64/arm64/armv7), macOS (x86_64/arm64), Windows (x86_64)
  - Optionally publishes to crates.io when `CARGO_REGISTRY_TOKEN` is configured

8) Verify
```bash
gh run list --workflow "Release" --limit 3
```
Or check Actions UI: https://github.com/andreas-glaser/qbak/actions

## Post‑Release

9) Sync branches
```bash
git checkout dev
git merge main
git push origin dev
```

10) (Optional) Start next version
- Bump `Cargo.toml` to `X.Y.(Z+1)-dev` and commit

## Versioning
- Patch: X.Y.(Z+1) — bug fixes
- Minor: X.(Y+1).0 — backward compatible features
- Major: (X+1).0.0 — breaking changes

## Files to Update
- `Cargo.toml` (version)
- `CHANGELOG.md`
