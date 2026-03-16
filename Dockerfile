# Build Stage
FROM rust:1.84-slim-bookworm AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libgeos-dev \
    && rm -rf /var/lib/apt/lists/*

# Install sqlx-cli for migrations
RUN cargo install sqlx-cli --no-default-features --features postgres

WORKDIR /app

# Copy the source code
COPY . .

# Set SQLx to offline mode for building without a database
ENV SQLX_OFFLINE=true

# Build the application
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

# Install runtime dependencies including netcat for the entrypoint script
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libgeos-c1v5 \
    netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary and sqlx-cli from the builder stage
COPY --from=builder /app/target/release/backend /app/backend
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx

# Copy migrations and entrypoint script
COPY migrations /app/migrations
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Set the PORT environment variable
ENV PORT=8080

# Expose the application port
EXPOSE 8080

# Use the entrypoint script to handle DB wait and migrations
ENTRYPOINT ["/app/entrypoint.sh"]
