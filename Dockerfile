# syntax=docker/dockerfile:1
FROM rust:1.96-alpine AS chef
RUN cargo install cargo-chef --version 0.1.77 --locked

FROM chef AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release && \
    cp target/release/stream-prism /app/stream-prism

FROM alpine:latest

RUN apk add --no-cache ca-certificates dumb-init

WORKDIR /app
COPY --from=builder /app/stream-prism /app/stream-prism

# Default environment configuration
ENV PROVIDERS_DIR=/app/providers \
    RUST_LOG=info \
    WEB_HOST=0.0.0.0 \
    WEB_PORT=8080

# Create default providers folder structure
RUN mkdir -p /app/providers

VOLUME ["/app/providers"]
EXPOSE 8080

ENTRYPOINT ["dumb-init", "/app/stream-prism"]
