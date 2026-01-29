# syntax=docker/dockerfile:1

FROM rust:1.91.1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        git \
        pkg-config \
        libpq-dev \
        clang \
        libclang-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY diesel.toml ./

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libpq5 \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/simple-sui-indexer /app/simple-sui-indexer

ENV RUST_LOG=info

ENTRYPOINT ["/app/simple-sui-indexer"]
