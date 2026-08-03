# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Build stage
# -----------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/casiros

# Copy the entire workspace source into the build container.
COPY . .

# Build the API binary in release mode.
RUN cargo build --release -p casiros-api

# -----------------------------------------------------------------------------
# Runtime stage
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash casiros

WORKDIR /app

COPY --from=builder /usr/src/casiros/target/release/casiros-api /usr/local/bin/casiros-api

USER casiros

ENV CASIROS_BIND_ADDR=0.0.0.0:8080

EXPOSE 8080

ENTRYPOINT ["casiros-api"]
