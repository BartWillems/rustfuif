FROM rust:1.41 as builder

WORKDIR /usr/src/rustfuif

COPY . .

RUN cargo build --release

FROM debian:buster-slim

COPY --from=builder /usr/src/rustfuif/target/release/rustfuif /usr/bin/rustfuif

CMD [ "rustfuif" ]