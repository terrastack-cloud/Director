# Stage 1: Builder
FROM rust:slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./


COPY src ./src

RUN cargo build --release

# Stage 2: Runtime
FROM debian:stable-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/apt/lists/*

COPY --from=builder /app/target/release/director /usr/local/bin/director

COPY config.yml /app/config.yml


ENTRYPOINT ["/usr/local/bin/director"]

CMD ["run","-c","/app/config.yml"]
