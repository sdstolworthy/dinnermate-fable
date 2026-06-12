# syntax=docker/dockerfile:1.7
#
# Multi-stage build for the dinnermate-api binary.
#
#   builder  — compiles the workspace with cached deps
#   runtime  — minimal Debian slim image that runs the binary
#
# Migrations are embedded at compile time via sqlx::migrate!("../../migrations")
# in dinnermate-db, so the runtime image needs no migrations directory.

FROM rust:1.95-slim-bookworm AS builder

WORKDIR /build

# rust-toolchain.toml pins `stable`; copying it first lets rustup resolve the
# channel before any source is present.
COPY rust-toolchain.toml ./

# Copy manifests first so dep compilation is cached across source changes.
COPY Cargo.toml Cargo.lock ./
COPY crates/dinnermate-core/Cargo.toml crates/dinnermate-core/Cargo.toml
COPY crates/dinnermate-db/Cargo.toml   crates/dinnermate-db/Cargo.toml
COPY crates/dinnermate-api/Cargo.toml  crates/dinnermate-api/Cargo.toml

# Stub entry points so dependencies compile without the real source.
# dinnermate-api is a hybrid bin+lib crate, so it needs both stubs.
RUN set -eux; \
    for crate in dinnermate-core dinnermate-db dinnermate-api; do \
        mkdir -p "crates/$crate/src"; \
        echo '' > "crates/$crate/src/lib.rs"; \
    done; \
    echo 'fn main() {}' > crates/dinnermate-api/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release -p dinnermate-api

# Now copy the real source and rebuild.
COPY crates ./crates
COPY migrations ./migrations

# COPY preserves host mtimes, which can predate the stub artifacts and fool
# cargo's fingerprint check into keeping the stub binary. Touch to force.
RUN find crates -name '*.rs' -exec touch {} +

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release -p dinnermate-api && \
    mkdir -p /out && \
    cp target/release/dinnermate-api /out/dinnermate-api

# ---------------------------------------------------------------------------

FROM debian:bookworm-slim AS runtime

# curl is needed by the HEALTHCHECK below.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 1000 --home /app --shell /usr/sbin/nologin dinnermate

WORKDIR /app

COPY --from=builder /out/dinnermate-api /usr/local/bin/dinnermate-api

USER dinnermate
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info,tower_http=info,sqlx=warn

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --retries=5 \
    CMD curl -fsS http://localhost:8080/healthz || exit 1

CMD ["/usr/local/bin/dinnermate-api"]
