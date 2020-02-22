FROM rust:1.41 as builder

WORKDIR /usr/src/rustfuif

RUN cargo install diesel_cli --no-default-features --features postgres

COPY . .

RUN cargo build --release

FROM debian:buster-slim

RUN apt update && apt install libpq-dev -y

COPY --from=builder /usr/local/cargo/bin/diesel /usr/bin/diesel
COPY --from=builder /usr/src/rustfuif/target/release/rustfuif /usr/bin/rustfuif

CMD [ "rustfuif" ]