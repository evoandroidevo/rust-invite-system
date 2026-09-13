FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libsqlite3-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY assets ./assets
COPY config.example.toml ./config.example.toml

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /app/target/release/rust-invite-system /usr/local/bin/rust-invite-system

RUN mkdir -p /app/data \
    && chown -R appuser:appuser /app

USER appuser

ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/rust-invite-system"]
