# Contributing to the Anthropic Rust SDK

Thanks for helping improve the project. Contributions should keep the public API,
documentation, tests, and release metadata consistent.

This is an unofficial community SDK. Do not present a change as reviewed or
endorsed by Anthropic, and do not copy credentials or customer data into an
issue, test, commit, or pull request.

## Before you start

- Search [existing issues](https://github.com/ThreatFlux/anthropic_rust_sdk/issues)
  and pull requests for related work.
- Open an issue before a large API redesign or breaking change so maintainers
  can align on scope.
- Report vulnerabilities privately through [SECURITY.md](SECURITY.md).
- Base normal work on `main` and keep unrelated changes in separate pull
  requests.

## Development setup

1. Fork and clone the repository:

   ```bash
   git clone https://github.com/YOUR-USER/anthropic_rust_sdk.git
   cd anthropic_rust_sdk
   git remote add upstream https://github.com/ThreatFlux/anthropic_rust_sdk.git
   ```

2. Install the declared MSRV and useful components:

   ```bash
   rustup toolchain install 1.95.0 --component rustfmt,clippy
   ```

3. Create a branch:

   ```bash
   git switch -c feature/short-description
   ```

No API key is needed for the unit and mock-server test paths. Set live
credentials only when deliberately running opt-in real API tests.

## Validation

Run the checks relevant to the change. The standard credential-free set is:

```bash
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy --all-targets --all-features -- -D warnings
cargo +1.95.0 test --test unit_suite
cargo +1.95.0 test --test integration_suite
cargo +1.95.0 test --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo +1.95.0 doc --no-deps --all-features
python3 scripts/check_docs.py
```

When changing TLS features, validate both supported configurations:

```bash
cargo +1.95.0 check --no-default-features --features native-tls
cargo +1.95.0 check --no-default-features --features rustls-tls
```

Run `cargo audit` and `cargo deny check` for dependency or release changes when
those tools are available. CI is the final authority for the supported operating
system and toolchain matrix.

## Live API tests

Live tests are opt-in because they can consume account quota, create remote
resources, and incur charges:

```bash
export ANTHROPIC_API_KEY="dedicated-test-key"
cargo +1.95.0 test --features real_api_tests --test real_api_suite
```

Some end-to-end tests are also marked `#[ignore]`. Read the test before adding
`--ignored`; confirm its cleanup behavior and selected model first. Use a
dedicated least-privilege credential and never use production prompts, files, or
responses as fixtures.

Administration tests require a separate `ANTHROPIC_ADMIN_KEY` and appropriate
organization permissions.

## Making a code change

- Preserve source compatibility unless the issue and pull request explicitly
  justify a breaking change.
- Use typed request and response models where the service schema is stable
  enough; retain forward compatibility for evolving enum/event surfaces.
- Return `AnthropicError` variants with actionable context and without secrets.
- Add unit tests for serialization, validation, and error behavior.
- Add mock-server tests for paths, methods, headers, request bodies, and response
  handling.
- Keep live tests narrowly scoped and feature-gated.
- Avoid logging credentials or complete sensitive request/response bodies.
- Consider retries, timeouts, pagination, streaming termination, and cleanup for
  every new endpoint.

## Documentation changes

Public API changes normally require:

- rustdoc on public items;
- an update to [API coverage](docs/api-coverage.md);
- an example or compile-checked doctest for a new workflow;
- README changes when installation, configuration, features, or primary usage
  changes; and
- changelog context when the change is user-visible.

The README quickstart is mirrored in `examples/quickstart.rs`. Update both and
run `python3 scripts/check_docs.py`; CI rejects drift between them. The checker
also verifies the documented package version, MSRV, Cargo features, and local
documentation links.

Model IDs, beta header versions, limits, and prices are time-sensitive. Verify
them against Anthropic's current official documentation and include the source
link and verification date in the pull request.

## Pull requests

A focused pull request should include:

- a clear problem statement and scope;
- user-visible behavior and compatibility impact;
- linked issue or official API reference when relevant;
- tests and documentation that cover the change;
- exact validation commands and results; and
- remaining risks, live-test gaps, or follow-up work.

Use Conventional Commit-style subjects, for example:

```text
feat(messages): support a new content block
fix(streaming): retain partial SSE frames
docs: clarify retry behavior
```

Keep the subject concise and explain the reason and tradeoffs in the commit or
pull-request body. Review the staged diff before committing so generated files,
credentials, `.env`, and unrelated edits are not included.

## Reporting bugs

A useful bug report contains:

1. The crate version or Git commit.
2. Rust version, target, operating system, and enabled Cargo features.
3. A minimal reproduction.
4. Expected and observed behavior.
5. Sanitized status, error variant, and response details.
6. Whether the issue reproduces against the current `main` branch.

Do not include API keys, bearer tokens, admin credentials, private files, or
unredacted customer content.

## Release process

Releases are managed by Release Please and the workflows under
`.github/workflows/`:

1. Conventional commits merged to `main` feed the release pull request.
2. The release pull request updates the crate version and changelog.
3. Merging it creates the tag and GitHub release.
4. The release workflow validates and publishes the crate and associated
   artifacts when repository credentials and environments permit.

Maintainers should verify the package with `cargo package` and its generated
file list before publishing. Do not manually edit a release tag after it has
been published.

## Getting help

Use [GitHub Issues](https://github.com/ThreatFlux/anthropic_rust_sdk/issues) for
reproducible SDK questions and proposals. Account, billing, service status, and
model-access questions belong with Anthropic support.
