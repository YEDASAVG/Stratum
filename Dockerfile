# LogAI Backend Dockerfile
# Multi-stage build for smaller final image

# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first (for layer caching)
COPY Cargo.toml Cargo.lock ./
COPY crates/logai-core/Cargo.toml crates/logai-core/
COPY crates/logai-api/Cargo.toml crates/logai-api/
COPY crates/logai-cli/Cargo.toml crates/logai-cli/
COPY crates/logai-worker/Cargo.toml crates/logai-worker/
COPY crates/logai-anomaly/Cargo.toml crates/logai-anomaly/
COPY crates/logai-rag/Cargo.toml crates/logai-rag/

# Create dummy source files for dependency caching
RUN mkdir -p crates/logai-core/src && echo "pub fn dummy() {}" > crates/logai-core/src/lib.rs
RUN mkdir -p crates/logai-api/src && echo "fn main() {}" > crates/logai-api/src/main.rs
RUN mkdir -p crates/logai-cli/src && echo "fn main() {}" > crates/logai-cli/src/main.rs
RUN mkdir -p crates/logai-worker/src && echo "fn main() {}" > crates/logai-worker/src/main.rs
RUN mkdir -p crates/logai-anomaly/src && echo "pub fn dummy() {}" > crates/logai-anomaly/src/lib.rs
RUN mkdir -p crates/logai-rag/src && echo "pub fn dummy() {}" > crates/logai-rag/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release || true

# Copy actual source code
COPY crates/ crates/
COPY config/ config/

# Touch files to invalidate cache for source changes
RUN touch crates/logai-core/src/lib.rs
RUN touch crates/logai-api/src/main.rs
RUN touch crates/logai-cli/src/main.rs
RUN touch crates/logai-worker/src/main.rs
RUN touch crates/logai-anomaly/src/lib.rs
RUN touch crates/logai-rag/src/lib.rs

# Build release binaries
RUN cargo build --release

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder
COPY --from=builder /app/target/release/logai-api /app/
COPY --from=builder /app/target/release/logai-worker /app/
COPY --from=builder /app/target/release/logai /app/
COPY --from=builder /app/target/release/logai-simulate /app/
COPY --from=builder /app/target/release/logai-stress /app/

# Copy config files
COPY --from=builder /app/config/ /app/config/

# Create directory for fastembed model cache
RUN mkdir -p /app/.fastembed_cache
ENV FASTEMBED_CACHE_PATH=/app/.fastembed_cache

# Default environment variables
ENV PORT=3000
ENV RUST_LOG=info
ENV NATS_URL=nats:4222
ENV QDRANT_URL=http://qdrant:6334
ENV CLICKHOUSE_URL=http://clickhouse:8123

# Expose API port
EXPOSE 3000

# Default command (can be overridden)
CMD ["./logai-api"]
