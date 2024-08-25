FROM lukemathwalker/cargo-chef:latest-rust-1.77.0 AS chef
ARG EXECUTABLE
RUN : "${EXECUTABLE:?Build argument needs to be set and non-empty.}"
WORKDIR /app/${EXECUTABLE}

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/${EXECUTABLE}/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json --bin ${EXECUTABLE}
# Build application
COPY . .
RUN cargo build --release --bin ${EXECUTABLE}

FROM debian:bookworm-slim AS runtime
ARG EXECUTABLE
ENV EXECUTABLE=$EXECUTABLE
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/${EXECUTABLE}/target/release/${EXECUTABLE} /usr/local/bin
ENTRYPOINT /usr/local/bin/${EXECUTABLE}

# TODO: Add env var to run a specific command for custom builds
