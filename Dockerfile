# ── Stage 1: Build ───────────────────────────────────────────
FROM rust:1-slim AS builder

WORKDIR /app

# Install OpenSSL dev libs (needed by reqwest's native-tls)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies separately from source code
# Cargo.lock is auto-generated on first build if not present
COPY Cargo.toml ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && \
    cargo build --release 2>/dev/null; true

# Now copy real source and build
COPY src/ ./src/
# Touch main.rs to invalidate the dummy build cache
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime ─────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/atom /usr/local/bin/atom
COPY static/ /app/static/

ENV VAULT_PATH=/app/vault
ENV BIND_ADDR=0.0.0.0:8080

EXPOSE 8080

CMD ["atom"]
