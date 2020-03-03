FROM rust:1.41 as builder

RUN cargo install diesel_cli --version=1.4.0 --no-default-features --features postgres

FROM debian:buster-slim

RUN apt-get update && \
    apt-get install libpq-dev -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/diesel /usr/bin/diesel
COPY ./target/release/rustfuif /usr/bin/rustfuif

CMD [ "rustfuif" ]