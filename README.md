# Rustfuif

![Rust](https://github.com/BartWillems/rustfuif/workflows/Rust/badge.svg?branch=master "ci/cd status")

![alt text](logo.png "Rustfuif Logo")

[Fronted](https://github.com/BartWillems/rustfuif-frontend)

Rustfuif is an open source stock market party implementation.

## Concept

A stock market party is an event where the prices of the beverages rise and fall based upon demand.

All prices are updated on certain intervals.
Sometimes there's a stock market crash, which causes all beverages to drop to their lowest possible price.

You can invite other players(bars/youth clubs) to join your stock market party.
They will then have an influence on your prices, and you on theirs.

### Features

- automatic price updates based on demand
- some nice graphs
- history of purchases
- admin panel to see connected users, active games, total games, ...

## Development

```bash
docker-compose up

# cargo install cargo-watch
cargo watch -x run
```

### Notable Crates

- [actix/actix-web](https://github.com/actix/actix-web)
- [bikeshedder/deadpool](https://github.com/bikeshedder/deadpool)
- [diesel-rs/diesel](https://github.com/diesel-rs/diesel)
- [mitsuhiko/redis-rs](https://github.com/mitsuhiko/redis-rs)
- [open-telemetry/opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust)
- [seanmonstar/reqwest](https://github.com/seanmonstar/reqwest)
- [serde-rs/serde](https://github.com/serde-rs/serde)
- [tokio-rs/tracing](https://github.com/tokio-rs/tracing)

### Configuration

| Required | Variable                 | Description                                     | Example                                         | Default                          |
| -------- | ------------------------ | ----------------------------------------------- | ----------------------------------------------- | -------------------------------- |
| ✗        | `API_HOST`               | The hostname/ip address the rustfuif listens on | `0.0.0.0`                                       | `localhost`                      |
| ✗        | `API_PORT`               | The port the rustfuif listens on                | `80`                                            | `8080`                           |
| ✗        | `RUST_LOG`               | loglevel for different crates                   | `rustfuif=info`                                 | `rustfuif=debug,actix_web=debug` |
| ✓        | `DATABASE_URL`           | URL to the database                             | `postgres://rustfuif:secret@127.0.0.1/rustfuif` | ``                               |
| ✓        | `SESSION_PRIVATE_KEY`    | secret used for cookies(minimum 32 characters)  | `...random_characters...`                       | ``                               |
| ✗        | `REDIS_URL`              | Redis cache URL, this is unused if empty        | `redis://redis`                                 | ``                               |
| ✗        | `SENTRY_DSN`             | Sentry error reporting middleware DSN           | `https://examplePublicKey@ingest.sentry.io/0`   | ``                               |
| ✗        | `PRICE_UPDATE_INTERVAL`  | Interval in seconds between price updates       | `120`                                           | `120`                            |
| ✗        | `OPENTELEMETRY_ENDPOINT` | OpenTelemetry agent endpoint                    | `jaeger:6831`                                   | `127.0.0.1:6831`                 |

### Observability

- `/metrics` constains prometheus metrics
- `/health` returns http 200
- `/stats` shows the following live stats:
  - total handled requests
  - total server errors (http response code >= 500)
  - active websocket connections
  - active games
  - active db connections
  - idle db connections
- more logging can be acquired by setting the `RUST_LOG` to a lower loglevel, eg:
  - `RUST_LOG="actix_server=info,actix_web=trace,rustfuif=trace"`
  - `RUST_LOG="debug"`
  - ...
- jaeger tracing using:
  - [tokio-rs/tracing](https://github.com/tokio-rs/tracing)
  - [tracing-opentelemetry](https://github.com/tokio-rs/tracing/tree/master/tracing-opentelemetry)
  - [OutThereLabs/actix-web-opentelemetry](https://github.com/OutThereLabs/actix-web-opentelemetry)
