ARG DOCKER_IMAGE="node:20-slim"

FROM ${DOCKER_IMAGE} AS builder
COPY . /app/oauth
WORKDIR /app/oauth
RUN npm install && npm run build

FROM ${DOCKER_IMAGE}
RUN apt-get update && apt-get install -y tini
COPY --from=builder /app/oauth/dist /app/oauth/dist
COPY --from=builder /app/oauth/node_modules /app/oauth/node_modules
COPY --from=builder /app/oauth/package.json /app/oauth/package.json
COPY --from=builder /app/oauth/package-lock.json /app/oauth/package-lock.json
WORKDIR /app/oauth
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["node", "/app/oauth/dist/index.js"]
