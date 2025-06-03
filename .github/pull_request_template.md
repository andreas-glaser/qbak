## Description
Brief description of what this PR does.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring (no functional changes)

## Related Issue
Fixes #(issue number) or closes #(issue number)

## Changes Made
- [ ] List the main changes
- [ ] Include any new files created
- [ ] Note any removed functionality
- [ ] Highlight any API changes

## Testing
- [ ] All existing tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Manual testing performed
- [ ] Cross-platform testing (if relevant)

### Test Commands
```bash
# Commands used to test this change
cargo test
cargo build --release
./target/release/qbak --help
```

## Documentation
- [ ] Code is self-documenting with clear comments
- [ ] README updated (if needed)
- [ ] CHANGELOG updated (if needed)
- [ ] Help text updated (if needed)

## Security
- [ ] No new security vulnerabilities introduced
- [ ] Input validation maintained/improved
- [ ] No sensitive information exposed
- [ ] Path traversal protection preserved

## Performance
- [ ] No significant performance regressions
- [ ] Memory usage remains reasonable
- [ ] Binary size impact minimal (if any)

## Cross-Platform Compatibility
- [ ] Works on Linux
- [ ] Works on macOS (if testable)
- [ ] Works on Windows/WSL (if testable)
- [ ] No platform-specific code without guards

## Checklist
- [ ] I have read the contributing guidelines
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes