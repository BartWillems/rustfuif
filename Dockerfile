FROM rust:1.50 as builder

WORKDIR /usr/src/rustfuif

ENV SQLX_OFFLINE="true"

RUN cargo install diesel_cli --no-default-features --features postgres

COPY Cargo.toml Cargo.lock sqlx-data.json ./
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release

FROM debian:buster-slim

WORKDIR /usr/src/rustfuif

# curl is used for docker-compose health checks
RUN apt-get update && \
    apt-get install libpq-dev curl ca-certificates -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY ./migrations ./migrations
COPY --from=builder /usr/local/cargo/bin/diesel /usr/bin/diesel
COPY --from=builder /usr/src/rustfuif/target/release/rustfuif /usr/bin/rustfuif

CMD [ "rustfuif" ]