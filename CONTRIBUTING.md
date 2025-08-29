# Contributing to Anthropic Rust SDK

First off, thank you for considering contributing to the Anthropic Rust SDK! It's people like you that make this SDK a great tool for the Rust community.

## Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* Use a clear and descriptive title
* Describe the exact steps which reproduce the problem
* Provide specific examples to demonstrate the steps
* Describe the behavior you observed after following the steps
* Explain which behavior you expected to see instead and why
* Include Rust version, OS, and any relevant environment details

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

* Use a clear and descriptive title
* Provide a step-by-step description of the suggested enhancement
* Provide specific examples to demonstrate the steps
* Describe the current behavior and explain which behavior you expected to see instead
* Explain why this enhancement would be useful

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes (`cargo test`)
5. Make sure your code lints (`cargo clippy -- -D warnings`)
6. Format your code (`cargo fmt`)
7. Issue that pull request!

## Development Setup

1. Clone your fork:
```bash
git clone git@github.com:your-username/anthropic_rust_sdk.git
cd anthropic_rust_sdk
```

2. Install Rust (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Install development tools:
```bash
cargo install cargo-watch cargo-tarpaulin cargo-audit cargo-outdated
```

4. Set up your environment:
```bash
cp .env.example .env
# Edit .env with your Anthropic API key for testing
```

5. Run tests:
```bash
cargo test
```

6. Run with watching:
```bash
cargo watch -x test
```

## Development Workflow

### Before Committing

1. **Format your code:**
```bash
cargo fmt
```

2. **Run clippy:**
```bash
cargo clippy -- -D warnings
```

3. **Run tests:**
```bash
cargo test
```

4. **Check documentation:**
```bash
cargo doc --no-deps --open
```

5. **Run security audit:**
```bash
cargo audit
```

### Testing

* Write unit tests for new functionality
* Ensure all tests pass before submitting PR
* Add integration tests for API interactions where appropriate
* Mock external API calls in tests

### Documentation

* Add documentation comments to all public APIs
* Include examples in doc comments
* Update README.md if adding new features
* Update CHANGELOG.md following Keep a Changelog format

## Style Guidelines

### Rust Code Style

* Follow standard Rust naming conventions
* Use `rustfmt` for formatting
* Keep functions small and focused
* Prefer composition over inheritance
* Use descriptive variable names
* Add comments for complex logic

### Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line

Format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
* `feat`: New feature
* `fix`: Bug fix
* `docs`: Documentation only changes
* `style`: Formatting, missing semi colons, etc
* `refactor`: Code change that neither fixes a bug nor adds a feature
* `perf`: Code change that improves performance
* `test`: Adding missing tests
* `chore`: Changes to the build process or auxiliary tools

### Example Workflow

1. Create a feature branch:
```bash
git checkout -b feature/my-new-feature
```

2. Make your changes and commit:
```bash
git add .
git commit -m "feat(messages): add support for new message format"
```

3. Push to your fork:
```bash
git push origin feature/my-new-feature
```

4. Open a Pull Request

## Release Process

Releases are automated through GitHub Actions when a new tag is pushed:

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Commit changes
4. Create and push tag:
```bash
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin v0.1.0
```

## Questions?

Feel free to open an issue with your question or reach out to the maintainers.

## Recognition

Contributors will be recognized in:
* The CHANGELOG.md file
* The project README
* GitHub's contributor graph

Thank you for contributing!