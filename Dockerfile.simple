## Simplified Rust Service Dockerfile
## Multi-stage build producing a small runtime image.
## Use APP_BIN build arg to set the binary name (defaults to package name if omitted).

ARG RUST_VERSION=1.86.0
FROM rust:${RUST_VERSION}-slim AS build
WORKDIR /src

ARG APP_BIN=""
ENV CARGO_TERM_COLOR=always \
    RUSTFLAGS="-C target-cpu=native"

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo 'fn main() {println!("pre-build");}' > src/main.rs \
 && cargo build --release || true

# Copy real sources
COPY src ./src
COPY shared ./shared 2>/dev/null || true

RUN cargo build --release --locked \
 && ls -lh target/release

FROM debian:bookworm-slim AS runtime
ARG APP_BIN=""
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

# Determine binary (if APP_BIN unset, infer by listing available release binaries excluding deps)
COPY --from=build /src/target/release /release
RUN set -e; \
 if [ -z "$APP_BIN" ]; then \
   APP_BIN=$(find /release -maxdepth 1 -type f -executable -printf '%f\n' | grep -v '\.d$' | head -1); \
 fi; \
 install -m 0755 /release/$APP_BIN /usr/local/bin/app; \
 echo "$APP_BIN" > /app/.binary_name

ENV RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/app"]
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD ["/usr/local/bin/app","--help"] || exit 1
