ARG DOCKER_IMAGE="node:20-slim"

FROM ${DOCKER_IMAGE} AS builder
COPY . /app/platform-oauth
WORKDIR /app/platform-oauth
RUN npm install && npm run build

FROM ${DOCKER_IMAGE}
RUN apt-get update && apt-get install -y tini
COPY --from=builder /app/platform-oauth/dist /app/platform-oauth/dist
COPY --from=builder /app/platform-oauth/node_modules /app/platform-oauth/node_modules
COPY --from=builder /app/platform-oauth/package.json /app/platform-oauth/package.json
COPY --from=builder /app/platform-oauth/package-lock.json /app/platform-oauth/package-lock.json
WORKDIR /app/platform-oauth
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["node", "/app/platform-oauth/dist/index.js"]
