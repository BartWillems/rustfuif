FROM rust:1.47 as builder

WORKDIR /usr/src/rustfuif

RUN cargo install diesel_cli --no-default-features --features postgres

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release

FROM debian:buster-slim

WORKDIR /usr/src/rustfuif

# curl is used for docker-compose health checks
RUN apt-get update && \
    apt-get install libpq-dev curl -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN mkdir api-spec

COPY ./migrations ./migrations
COPY --from=builder /usr/local/cargo/bin/diesel /usr/bin/diesel
COPY --from=builder /usr/src/rustfuif/target/release/rustfuif /usr/bin/rustfuif

CMD [ "rustfuif" ]