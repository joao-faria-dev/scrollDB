# Contributing to ScrollDB

Thank you for your interest in contributing to ScrollDB! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- **Rust**: Version 1.70 or later ([rustup](https://rustup.rs/))
- **Python**: Version 3.8 or later (for Python bindings)
- **Maturin**: For building Python bindings (`pip install maturin`)
- **Git**: For version control

### Development Setup

1. **Fork the repository** on GitHub by clicking the "Fork" button at the top of the repository page.


2. **Build the project**:
   ```bash
   # Build the Rust library
   cargo build

   # Build Python bindings (optional)
   cd scrolldb-py
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   pip install maturin
   maturin develop
   ```

3. **Run tests**:
   ```bash
   # Rust tests
   cargo test

   # Python tests
   cd scrolldb-py
   python -m pytest tests/
   ```

## Development Workflow

### 1. Keep Your Fork Updated

Before starting work, ensure your fork is up to date with the upstream repository:

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main
```

### 2. Create a Branch

Create a feature branch from `main`:

```bash
git checkout -b feature/your-feature-name
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or improvements

### 3. Make Changes

- Write clear, readable code
- Follow Rust conventions (use `rustfmt`)
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

### 4. Code Style

#### Rust

- Use `rustfmt` to format code:
  ```bash
  cargo fmt
  ```

- Run `clippy` for linting:
  ```bash
  cargo clippy -- -D warnings
  ```

- Follow Rust naming conventions:
  - Functions: `snake_case`
  - Types: `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE`

#### Python

- Follow PEP 8 style guide
- Use type hints where appropriate
- Format with `black` (if configured)

### 5. Testing

#### Writing Tests

- **Unit tests**: Place in the same file with `#[cfg(test)]`
- **Integration tests**: Place in `tests/` directory
- **Python tests**: Place in `scrolldb-py/tests/`

Example Rust test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

#### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_feature

# With output
cargo test -- --nocapture
```

### 6. Documentation

- Document public APIs with doc comments:
  ```rust
  /// Opens or creates a database at the given path.
  ///
  /// # Arguments
  /// * `path` - The file path to the database
  ///
  /// # Returns
  /// A `Result` containing the `Database` or an error
  pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
      // ...
  }
  ```

- Update README.md for user-facing changes
- Add examples for new features

### 7. Commit Messages

Write clear, descriptive commit messages:

```
feat: Add support for $regex query operator

- Implement regex matching in query parser
- Add tests for regex queries
- Update documentation

Fixes #123
```

Commit message format:
- **Type**: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`
- **Scope** (optional): Component affected
- **Description**: Clear summary
- **Body** (optional): Detailed explanation
- **Footer** (optional): Issue references

### 8. Push to Your Fork

Push your branch to your fork:

```bash
git push origin feature/your-feature-name
```

### 9. Submit a Pull Request

1. **Go to GitHub** and navigate to your fork of the repository.

2. **Create a Pull Request**:
   - Click "New Pull Request"
   - Select your branch from your fork
   - Set the base repository and branch (usually `main` of the upstream repository)
   - Click "Create Pull Request"

3. **Fill out the PR template**:
   - Use a clear, descriptive title
   - Describe what changes you made and why
   - Reference any related issues
   - Include screenshots/examples if applicable

4. **PR Checklist**:
   - [ ] Code follows style guidelines
   - [ ] Tests pass (`cargo test`)
   - [ ] No warnings (`cargo clippy`)
   - [ ] Code is formatted (`cargo fmt`)
   - [ ] Documentation is updated
   - [ ] Commit messages are clear

## Reporting Bugs

When reporting bugs, please include:

1. **Description**: Clear description of the bug
2. **Reproduction**: Steps to reproduce
3. **Expected behavior**: What should happen
4. **Actual behavior**: What actually happens
5. **Environment**:
   - OS and version
   - Rust version (`rustc --version`)
   - Python version (if applicable)
   - ScrollDB version
6. **Error messages**: Full error output
7. **Minimal example**: Small code snippet that reproduces the issue

## Suggesting Features

Feature suggestions are welcome! Please:

1. Check if the feature already exists or is planned
2. Open an issue with the `enhancement` label
3. Describe:
   - Use case and motivation
   - Proposed API/interface
   - Potential implementation approach
   - Alternatives considered

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [PyO3 Documentation](https://pyo3.rs/)
- [Maturin Documentation](https://maturin.rs/)

## Questions?

- Open an issue for questions
- Check existing issues and discussions
- Reach out to maintainers

Thank you for contributing to ScrollDB!
