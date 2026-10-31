# Stage 1: build
FROM rust:1.99.1-bookworm AS builder
WORKDIR /app

# Build dependencies first with a dummy main so they stay cached
# until Cargo.toml or Cargo.lock changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src target/release/deps/wage3_backend*

# migrations/ is embedded into the binary by sqlx::migrate!()
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release


# Stage 2: run
FROM debian:bookworm-slim
# ca-certificates: HTTPS calls to Google and the RPC node
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --no-create-home app
USER app
WORKDIR /app

COPY --from=builder /app/target/release/wage3-backend .
EXPOSE 8080
CMD ["./wage3-backend"]
