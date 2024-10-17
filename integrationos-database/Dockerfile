# syntax = devthefuture/dockerfile-x

ARG EXECUTABLE=integrationos-database
INCLUDE Dockerfile.common

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/integrationos-database/target/release/integrationos-database /usr/local/bin
ENTRYPOINT /usr/local/bin/integrationos-database
