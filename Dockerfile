# Builds a fully static Linux binary (musl) and ships it alone in an empty image.
#   docker build -t crm .
#   docker run -p 3000:3000 -v crm-data:/data crm

FROM node:22-alpine AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# On Alpine, Rust targets musl and links statically. SQLite is compiled in by rusqlite's `bundled` feature.
FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev gcc
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY migrations ./migrations
COPY --from=web /web/dist ./web/dist
RUN cargo build --release --locked
RUN mkdir /data && chown 10001:10001 /data

FROM scratch
COPY --from=build /src/target/release/crm /crm
COPY --from=build --chown=10001:10001 /data /data
ENV CRM_DB=/data/crm.db CRM_BIND=0.0.0.0:3000
USER 10001:10001
VOLUME /data
EXPOSE 3000
ENTRYPOINT ["/crm"]
