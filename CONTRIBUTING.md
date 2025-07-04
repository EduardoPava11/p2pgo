# Contributing to P2P Go

Thank you for your interest in contributing to P2P Go! We welcome contributions of all kinds - code, documentation, bug reports, feature requests, and more.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and welcoming to all contributors.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](https://github.com/EduardoPava11/p2pgo/issues)
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - System information (OS, version)
   - Relevant logs or screenshots

### Suggesting Features

1. Check existing feature requests
2. Create a new issue with the "enhancement" label
3. Describe the feature and its benefits
4. Consider how it fits with P2P Go's philosophy

### Code Contributions

#### Setup

```bash
# Fork and clone the repository
git clone https://github.com/EduardoPava11/p2pgo.git
cd p2pgo

# Add upstream remote
git remote add upstream https://github.com/EduardoPava11/p2pgo.git

# Create a feature branch
git checkout -b feature/your-feature-name
```

#### Development Process

1. **Make your changes**
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed

2. **Test thoroughly**
   ```bash
   # Run all tests
   cargo test --all-features
   
   # Run clippy
   cargo clippy -- -D warnings
   
   # Check formatting
   cargo fmt -- --check
   ```

3. **Commit your changes**
   ```bash
   # Use conventional commits
   git commit -m "feat: add new feature"
   git commit -m "fix: resolve issue with X"
   git commit -m "docs: update README"
   ```

4. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   Then create a Pull Request on GitHub

#### Pull Request Guidelines

- Clear title describing the change
- Reference any related issues
- Include screenshots for UI changes
- Ensure all CI checks pass
- Be responsive to review feedback

### Code Style

#### Rust Style

- Follow standard Rust naming conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write documentation for public APIs
- Keep functions focused and small

#### Architecture Guidelines

- Maintain separation between crates
- P2P code stays in `network/`
- Game logic stays in `core/`
- UI code stays in `ui-egui/`
- No circular dependencies

#### Testing

- Unit tests go in the same file
- Integration tests in `tests/`
- Test edge cases and error conditions
- Mock external dependencies

### Documentation

#### Code Documentation

```rust
/// Brief description of what this does.
///
/// Longer explanation if needed, including:
/// - Important details
/// - Usage examples
/// - Edge cases
///
/// # Arguments
/// * `foo` - Description of foo
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When this function returns errors
pub fn example(foo: &str) -> Result<String, Error> {
    // Implementation
}
```

#### User Documentation

- Update README.md for user-facing changes
- Update architecture docs for design changes
- Include examples where helpful
- Keep language clear and concise

### Project Structure

Understand the codebase:

```
p2pgo/
├── core/        # Game logic (no external deps)
├── network/     # P2P networking
├── neural/      # Neural network AI
├── ui-egui/     # Desktop UI
└── docs/        # Documentation and website
```

### Getting Help

- Check existing documentation
- Ask in GitHub Discussions
- Review similar PRs/issues
- Tag maintainers if truly stuck

## Recognition

Contributors are recognized in:
- The git history (use your real name/handle)
- Release notes for significant contributions
- A future CONTRIBUTORS.md file

## Release Process

Maintainers handle releases:
1. Version bump in Cargo.toml
2. Update CHANGELOG.md
3. Create git tag
4. GitHub Actions builds release

## Philosophy

Remember P2P Go's core values:
- **Privacy First** - No tracking or data collection
- **True P2P** - No servers or intermediaries
- **Local First** - Everything runs on user's device
- **Open Source** - Transparent and community-driven

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT).

Thank you for helping make P2P Go better! 🎯