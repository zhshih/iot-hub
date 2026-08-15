# syntax=docker/dockerfile:1

FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --uid 10001 iot-hub

WORKDIR /app
COPY --from=builder /app/target/release/iot-hub /usr/local/bin/iot-hub
COPY static ./static

USER iot-hub
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/iot-hub"]
