FROM node:24-bookworm-slim AS frontend
WORKDIR /build
RUN npm install -g pnpm@11.9.0
COPY . .
RUN pnpm install --frozen-lockfile && pnpm build --mode web

FROM rust:bookworm AS backend
RUN apt-get update && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev ca-certificates cmake clang && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY src-tauri ./src-tauri
COPY web-server ./web-server
WORKDIR /build/web-server
RUN cargo build --release

FROM node:24-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/* && mkdir -p /app /data && chown node:node /data
WORKDIR /app
COPY --from=frontend /build/dist /app/dist
COPY --from=backend /build/web-server/target/release/flow-web-server /app/flow-web-server
COPY src-tauri/sidecar /app/sidecar
ENV FLOW_WEB_BIND=0.0.0.0:8780 FLOW_WEB_DIST=/app/dist FLOW_WEB_DATA=/data
USER node
EXPOSE 8780
ENTRYPOINT ["/app/flow-web-server"]
