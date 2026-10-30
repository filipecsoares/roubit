# Stage 1: Build binary
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --bin roubit-server

# Stage 2: Minimal hardened runtime
FROM debian:bookworm-slim
RUN useradd -m -u 1000 appuser && \
    apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*
USER appuser
WORKDIR /app
COPY --from=builder /app/target/release/roubit-server /app/roubit-server
EXPOSE 8080
ENV RUST_LOG=roubit=info,tower_http=info
CMD ["/app/roubit-server"]
