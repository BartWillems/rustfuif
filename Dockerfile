FROM rust:1.52 as builder

WORKDIR /usr/src/rustfuif

ENV SQLX_OFFLINE="true"

COPY Cargo.toml Cargo.lock sqlx-data.json ./
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release

RUN cargo install --version=0.5.2 sqlx-cli --no-default-features --features postgres

FROM debian:buster-slim

WORKDIR /usr/src/rustfuif

# curl is used for docker-compose health checks
RUN apt-get update && \
    apt-get install curl ca-certificates -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY ./migrations ./migrations
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/bin/sqlx
COPY --from=builder /usr/src/rustfuif/target/release/rustfuif /usr/bin/rustfuif
COPY ./entrypoint.sh /entrypoint.sh

RUN chmod +x /entrypoint.sh

ENTRYPOINT [ "/entrypoint.sh" ]

CMD [ "rustfuif" ]