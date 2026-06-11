# Multi-stage build for Zeus Cloud API

FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/zeus-cloud /usr/local/bin/

# Copy Zeus compiler (assumes it's built separately)
COPY --from=zeus-compiler /app/zeus_compiler/target/release/zeus_compiler /usr/local/bin/zeus

# Create non-root user
RUN useradd -m -u 1000 zeus

# Set working directory
WORKDIR /app

# Switch to non-root user
USER zeus

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
CMD ["zeus-cloud"]
