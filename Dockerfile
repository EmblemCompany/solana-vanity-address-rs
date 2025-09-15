# Use official Rust image for building
FROM rust:1.89 as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy source code
COPY . .

# Build the API binary with features
RUN cargo build --release --features api --bin solana-vanity-api

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/solana-vanity-api /usr/local/bin/solana-vanity-api

# Add start script
COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

# Expose port (Railway sets PORT env variable)
EXPOSE 8080

# Run via start script
CMD ["/usr/local/bin/start.sh"]