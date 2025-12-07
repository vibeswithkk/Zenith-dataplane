# Zenith AI Infrastructure - Multi-stage Docker Build
# Author: Wahyu Ardiansyah
# Copyright 2025 Zenith AI Contributors

# ============= BUILD STAGE =============
FROM rust:1.75-bookworm AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY zenith-proto ./zenith-proto
COPY zenith-runtime-cpu ./zenith-runtime-cpu
COPY zenith-runtime-gpu ./zenith-runtime-gpu
COPY zenith-scheduler ./zenith-scheduler
COPY zenith-bench ./zenith-bench

# Build release
RUN cargo build --release -p zenith-scheduler

# ============= RUNTIME STAGE =============
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --gid 1000 zenith && \
    useradd --uid 1000 --gid zenith --shell /bin/bash --create-home zenith

# Create data directory
RUN mkdir -p /var/lib/zenith && chown zenith:zenith /var/lib/zenith

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/zenith-scheduler /app/zenith-scheduler

# Switch to non-root user
USER zenith

# Expose ports
EXPOSE 50051 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command
ENTRYPOINT ["/app/zenith-scheduler"]
CMD ["--listen-addr", "0.0.0.0:50051", "--http-addr", "0.0.0.0:8080"]
