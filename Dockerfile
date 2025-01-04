# Builder stage
FROM rust:1.75-slim-bullseye as builder

WORKDIR /app

# Create blank project
RUN cargo new --bin blirumah-cron
WORKDIR /app/blirumah-cron

# Copy manifests
COPY Cargo.toml ./

# Cache dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/blirumah-cron/target/release/blirumah-cron .

# Copy .env file
COPY .env .

# Run the binary
CMD ["./blirumah-cron"] 