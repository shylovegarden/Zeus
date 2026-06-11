FROM rust:1.75-slim-bookworm

# Install system dependencies
RUN apt-get update && apt-get install -y \
    clang \
    llvm \
    libz3-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Zeus compiler
WORKDIR /opt/zeus
COPY . .
RUN cargo build --release

# Install to system path
RUN cp target/release/zeus /usr/local/bin/zeus

# Set up entrypoint
WORKDIR /github/workspace
COPY github-action/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
