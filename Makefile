.PHONY: all build test clean fmt lint audit doc release check install dev-setup \
        ci ci-fmt ci-clippy ci-build ci-test ci-doctest ci-doc ci-audit \
        ci-examples ci-msrv ci-license ci-coverage pre-commit examples deps \
        bench coverage watch help

# Default target
all: clean fmt lint build test doc

# Build the project
build:
	@echo "Building project..."
	@cargo build --all-features
	@cargo build --release --all-features

# Run all tests (full suite, matching CI: lib + integration crates + doctests)
test:
	@echo "Running tests..."
	@cargo test --all-features
	@cargo test --all-features --doc

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

# Security audit (hard-fails, matching CI)
audit:
	@echo "Running security audit..."
	@cargo audit

# Generate documentation (RUSTDOCFLAGS=-D warnings, matching CI's doc check)
doc:
	@echo "Generating documentation..."
	@RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --document-private-items

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

# ---------------------------------------------------------------------------
# Local CI gate — mirrors .github/workflows/ci.yml step-for-step so that a
# green `make ci` means green GitHub CI. RUN THIS BEFORE EVERY COMMIT/PUSH.
#
# Reproduced locally (deterministic, blocking in CI): formatting, clippy,
# build, tests, doc tests, the documentation build, security audit, examples,
# the MSRV check, and license check.
# NOT reproducible locally (hosted services; informational): Codecov patch
# coverage, Codacy, CodeQL/Analyze. Use `make ci-coverage` for local coverage.
# ---------------------------------------------------------------------------
ci: ci-fmt ci-clippy ci-build ci-test ci-doctest ci-doc ci-audit ci-examples ci-msrv ci-license
	@echo ""
	@echo "✅ make ci: all local CI checks passed — safe to commit/push"

ci-fmt:
	@echo "[ci] cargo fmt --all -- --check"
	@cargo fmt --all -- --check

ci-clippy:
	@echo "[ci] cargo clippy --all-targets --all-features -- -D warnings"
	@cargo clippy --all-targets --all-features -- -D warnings

ci-build:
	@echo "[ci] RUSTFLAGS=-D warnings cargo build --all-features"
	@RUSTFLAGS="-D warnings" cargo build --all-features

ci-test:
	@echo "[ci] RUSTFLAGS=-D warnings cargo test --all-features"
	@RUSTFLAGS="-D warnings" cargo test --all-features

ci-doctest:
	@echo "[ci] RUSTDOCFLAGS=-D warnings cargo test --doc --all-features"
	@RUSTDOCFLAGS="-D warnings" cargo test --doc --all-features

ci-doc:
	@echo "[ci] RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features --document-private-items"
	@RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --document-private-items

ci-audit:
	@echo "[ci] cargo audit"
	@cargo audit

ci-examples:
	@echo "[ci] cargo build --examples --all-features"
	@cargo build --examples --all-features

ci-msrv:
	@echo "[ci] MSRV (rustc 1.95.0): cargo check --all-features"
	@cargo check --all-features

ci-license:
	@echo "[ci] cargo deny check licenses"
	@cargo deny check licenses

# Local coverage (codecov is non-blocking in CI: fail_ci_if_error: false).
ci-coverage:
	@echo "[ci] cargo tarpaulin --all-features --workspace --out xml"
	@cargo tarpaulin --all-features --workspace --timeout 120 --out xml \
		|| echo "NOTE: cargo-tarpaulin missing or failed (codecov is non-blocking in CI; 'cargo install cargo-tarpaulin' to enable)"

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

# Pre-commit checks — run the full local CI gate before every commit/push
pre-commit: ci
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
	@echo "  ci         - Full local CI gate mirroring GitHub CI (run before commit)"
	@echo "  ci-coverage- Local coverage via tarpaulin (codecov is non-blocking)"
	@echo "  install    - Install development tools"
	@echo "  dev-setup  - Complete development setup"
	@echo "  examples   - Build all examples"
	@echo "  deps       - Check dependencies"
	@echo "  bench      - Run benchmarks"
	@echo "  coverage   - Generate coverage report"
	@echo "  watch      - Watch and test on changes"
	@echo "  pre-commit - Run pre-commit checks"
	@echo "  help       - Show this help message"