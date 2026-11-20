# ============================================================
# Multi-stage Dockerfile for Hanisha Rust Bot - Koyeb Free Tier
# ============================================================
# Build stage: compile the Rust binary
# ============================================================
FROM rust:alpine AS builder

# Install build dependencies (musl-dev for musl compilation)
# pkg-config not needed for these dependencies
RUN apk add --no-cache gcc musl-dev

# Set work directory
WORKDIR /app

# Copy cargo.toml and source files first (docker cache optimization)
COPY rust-bot/Cargo.toml .
# Create empty src to cache deps download before copying all source
RUN mkdir -p src/main.rs.dummy
COPY rust-bot/src/main.rs src/main.rs.dummy/

# Download dependencies (this layer is cached if only source changes)
RUN cargo build --release 2>&1 | tail -5

# Copy the rest of the source code
RUN rm -rf src/main.rs.dummy
COPY rust-bot/src/ ./src/

# Build the binary (statically linked if possible)
RUN cargo build --release 2>&1 | tail -5

# ============================================================
# Final stage: minimal runtime image
# ============================================================
# Using gcr.io/distroless/cc to keep image small and secure
# Final image size: ~25-40 MB with mimalloc allocation
FROM gcr.io/distroless/cc AS runtime

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/hanisha-rust-bot /hanisha-rust-bot

# Expose the Koyeb keep-alive port
EXPOSE 8000

# Environment defaults (can be overridden at runtime)
ENV RUST_LOG=info \
    API_ID="" \
    API_HASH="" \
    BOT_TOKEN="" \
    DATABASE_URI="" \
    DATABASE_NAME=Cluster0 \
    COLLECTION_NAME=mn_files \
    ADMINS="1892771262" \
    CHANNELS="-1002490892111 -1002097504396 -1003157977363 -1003034885508" \
    LOG_CHANNEL="-1002345447637" \
    CUSTOM_FILE_CAPTION="" \
    DELETE_TIME=300

# Entrypoint: start the Rust bot
# The binary binds to 0.0.0.0:8000 (or PORT env var) for keep-alive
CMD ["/hanisha-rust-bot"]