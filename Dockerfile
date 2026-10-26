# ThreatFlux Anthropic Rust SDK helper image.
#
# Follows ThreatFlux/rust-cicd-template's canonical layout: a Debian 13
# (trixie) Rust builder and a minimal Wolfi GNU runtime with no shell,
# package manager or coreutils. The helper binaries link OpenSSL through the
# default `native-tls` feature; the helper image vendors OpenSSL while the
# runtime supplies patched glibc, libgcc and CA certificates.
#
# Base images are pinned by digest for reproducibility (Scorecard
# Pinned-Dependencies). Dependabot refreshes both digests; refresh one by hand
# with:
#   docker buildx imagetools inspect <image> | awk '/^Digest:/{print $2}'
# The builder tag must match the build toolchain in rust-toolchain.toml
# (scripts/check_docs.py enforces this).

# rust 1.99.1 on Debian 13 (trixie); multi-arch index digest
FROM rust:1.99.1-trixie@sha256:931c2c0531a8b143e044f0f3914dfe802f15bb3d1955f0b657e9359327ac2ffc AS builder

# tini is installed here so the runtime stage can copy it out: distroless ships
# no init, and PID 1 must reap zombies and forward signals. Package revisions
# follow the pinned base image's Debian 13 repositories.
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl-dev pkg-config tini \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 builder
USER builder
WORKDIR /build

# The image's CARGO_HOME (/usr/local/cargo) is root-owned; give the
# unprivileged builder its own registry cache.
ENV CARGO_HOME=/home/builder/.cargo

COPY --chown=builder:builder . .

# Cargo.lock is not committed for this library crate. The release workflow
# resolves it once and passes it in the build context, so the image is built
# from the same dependency graph as the release assets; a local or CI build
# without one resolves it here.
RUN rustc --version --verbose && cargo --version \
    && if [ ! -f Cargo.lock ]; then cargo generate-lockfile; fi \
    && cargo build --release --locked --bins --features vendored-native-tls \
    && mkdir -p /home/builder/out/bin \
    && cp target/release/check_my_usage target/release/test_api /home/builder/out/bin/

# Wolfi GNU runtime with glibc 2.44-r8; multi-arch index digest.
# GNU ABI is retained; native TLS uses statically linked vendored OpenSSL.
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:2827063c9efb4ba971d8621eb75f4e61b8d19850a6293a797df37358cc6f687b AS runtime

ARG VERSION=dev
ARG VCS_REF=unknown

LABEL org.opencontainers.image.title="ThreatFlux Anthropic Rust SDK" \
      org.opencontainers.image.description="Rust SDK and helper binaries for the Anthropic API" \
      org.opencontainers.image.vendor="ThreatFlux" \
      org.opencontainers.image.source="https://github.com/ThreatFlux/anthropic_rust_sdk" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.licenses="MIT"

COPY --from=builder /usr/bin/tini /usr/bin/tini

# The binaries stay root-owned so the runtime user cannot modify them.
COPY --from=builder --chown=0:0 /home/builder/out/bin/ /usr/local/bin/

USER 65532:65532

# Exec form (there is no shell in distroless). The default command is kept from
# the previous image; run the other helper with `docker run <image> test_api`.
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/check_my_usage"]
