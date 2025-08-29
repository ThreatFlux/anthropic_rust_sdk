.PHONY: all build test clean fmt lint audit doc release check install dev-setup

# Default target
all: clean fmt lint build test doc

# Build the project
build:
	@echo "Building project..."
	@cargo build --all-features
	@cargo build --release --all-features

# Run all tests
test:
	@echo "Running tests..."
	@cargo test --all-features --lib
	@cargo test --all-features --doc
	@cargo test --all-features --examples

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf target/
	@rm -f Cargo.lock
	@rm -rf tarpaulin-report.html cobertura.xml

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt --all

# Run clippy linter
lint:
	@echo "Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

# Security audit
audit:
	@echo "Running security audit..."
	@cargo audit || true

# Generate documentation
doc:
	@echo "Generating documentation..."
	@cargo doc --no-deps --all-features --document-private-items

# Prepare for release
release: all
	@echo "Checking for release readiness..."
	@cargo publish --dry-run

# Quick check
check:
	@echo "Running quick check..."
	@cargo check --all-features
	@cargo fmt --all -- --check
	@cargo clippy --all-features -- -D warnings

# Install development dependencies
install:
	@echo "Installing development tools..."
	@cargo install cargo-audit || true
	@cargo install cargo-tarpaulin || true
	@cargo install cargo-outdated || true
	@cargo install cargo-deny || true
	@cargo install cargo-criterion || true

# Full development setup
dev-setup: install
	@echo "Setting up development environment..."
	@rustup component add rustfmt clippy
	@rustup toolchain install nightly
	@cp .env.example .env 2>/dev/null || true
	@echo "Development environment ready!"

# Run examples
examples:
	@echo "Building examples..."
	@for example in examples/*.rs; do \
		name=$$(basename $$example .rs); \
		echo "Building example: $$name"; \
		cargo build --example $$name; \
	done

# Check dependencies
deps:
	@echo "Checking dependencies..."
	@cargo tree
	@cargo outdated || true

# Run benchmarks
bench:
	@echo "Running benchmarks..."
	@cargo bench --all-features

# Coverage report
coverage:
	@echo "Generating coverage report..."
	@cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out html

# Watch for changes and run tests
watch:
	@echo "Watching for changes..."
	@cargo watch -x test

# Pre-commit checks
pre-commit: fmt lint test
	@echo "Pre-commit checks passed!"

# Help
help:
	@echo "Available targets:"
	@echo "  all        - Run clean, fmt, lint, build, test, and doc"
	@echo "  build      - Build debug and release versions"
	@echo "  test       - Run all tests"
	@echo "  clean      - Remove build artifacts"
	@echo "  fmt        - Format code with rustfmt"
	@echo "  lint       - Run clippy linter"
	@echo "  audit      - Run security audit"
	@echo "  doc        - Generate documentation"
	@echo "  release    - Prepare for crates.io release"
	@echo "  check      - Quick check (format, lint)"
	@echo "  install    - Install development tools"
	@echo "  dev-setup  - Complete development setup"
	@echo "  examples   - Build all examples"
	@echo "  deps       - Check dependencies"
	@echo "  bench      - Run benchmarks"
	@echo "  coverage   - Generate coverage report"
	@echo "  watch      - Watch and test on changes"
	@echo "  pre-commit - Run pre-commit checks"
	@echo "  help       - Show this help message"