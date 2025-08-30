# Multi-stage Dockerfile extending shared Rust template

# Build stage - extends shared Rust template
FROM shared/rust:1.81-slim AS build

WORKDIR /src

# Copy Rust project files
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the application
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked && \
    cp target/release/fks_nodes /usr/local/bin/app

# Runtime stage - using distroless for security
FROM gcr.io/distroless/cc-debian12:latest AS final

WORKDIR /app

# Copy CA certificates and timezone data
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /usr/share/zoneinfo /usr/share/zoneinfo

# Copy the built binary
COPY --from=build /usr/local/bin/app /usr/local/bin/app

# Set service-specific environment variables
ENV SERVICE_NAME=fks-nodes \
    SERVICE_TYPE=nodes \
    SERVICE_PORT=8080

EXPOSE ${SERVICE_PORT}

# Run as non-root user (provided by distroless)
USER 65534:65534

ENTRYPOINT ["/usr/local/bin/app"]
