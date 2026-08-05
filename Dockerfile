# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Build stage
# -----------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/casiros

# Copy the entire workspace source into the build container.
COPY . .

# Build the API and worker binaries in release mode.
RUN cargo build --release -p casiros-api
RUN cargo build --release -p casiros-worker

# -----------------------------------------------------------------------------
# API runtime stage
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS api

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash casiros

WORKDIR /app

COPY --from=builder /usr/src/casiros/target/release/casiros-api /usr/local/bin/casiros-api
COPY --from=builder /usr/src/casiros/migrations /app/migrations
COPY --from=builder /usr/src/casiros/config/default.toml /app/config/default.toml

USER casiros

ENV CASIROS__BIND_ADDR=0.0.0.0:8080

EXPOSE 8080

ENTRYPOINT ["casiros-api"]

# -----------------------------------------------------------------------------
# Worker runtime stage
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS worker

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash casiros

WORKDIR /app

COPY --from=builder /usr/src/casiros/target/release/casiros-worker /usr/local/bin/casiros-worker

USER casiros

ENTRYPOINT ["casiros-worker"]
