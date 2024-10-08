# syntax = devthefuture/dockerfile-x

ARG EXECUTABLE=integrationos-storage
INCLUDE Dockerfile.common

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/integrationos-storage/target/release/integrationos-storage /usr/local/bin
ENTRYPOINT /usr/local/bin/integrationos-storage
