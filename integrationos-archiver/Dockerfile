# syntax = devthefuture/dockerfile-x

ARG EXECUTABLE=integrationos-archiver
INCLUDE Dockerfile.common

FROM mongo:6
WORKDIR /app
COPY --from=builder /app/integrationos-archiver/target/release/integrationos-archiver /usr/local/bin
ENTRYPOINT /usr/local/bin/integrationos-archiver
