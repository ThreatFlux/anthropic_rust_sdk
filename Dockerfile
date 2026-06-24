FROM rust:1.95-bookworm AS builder

WORKDIR /app
COPY . .

ARG VERSION=dev

RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo generate-lockfile \
    && cargo build --release --bins --locked

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

ARG VERSION=dev

LABEL org.opencontainers.image.title="ThreatFlux Anthropic Rust SDK" \
      org.opencontainers.image.description="Rust SDK and helper binaries for the Anthropic API" \
      org.opencontainers.image.vendor="ThreatFlux" \
      org.opencontainers.image.source="https://github.com/ThreatFlux/anthropic_rust_sdk" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.licenses="MIT"

COPY --from=builder /app/target/release/check_my_usage /usr/local/bin/check_my_usage
COPY --from=builder /app/target/release/test_api /usr/local/bin/test_api

CMD ["/usr/local/bin/check_my_usage"]
