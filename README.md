# Rustfuif

![Rust](https://github.com/BartWillems/rustfuif/workflows/Rust/badge.svg?branch=master "ci/cd status")

![alt text](logo.png "Rustfuif Logo")

[frontend](https://github.com/BartWillems/rustfuif-frontend)

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

### Configuration

| Required | Variable              | Description                                     | Example                                         | Default                          |
| -------- | --------------------- | ----------------------------------------------- | ----------------------------------------------- | -------------------------------- |
| ✗        | `API_HOST`            | The hostname/ip address the rustfuif listens on | `0.0.0.0`                                       | `localhost`                      |
| ✗        | `API_PORT`            | The port the rustfuif listens on                | `80`                                            | `8080`                           |
| ✗        | `RUST_LOG`            | loglevel for different crates                   | `rustfuif=info`                                 | `rustfuif=debug,actix_web=debug` |
| ✓        | `DATABASE_URL`        | URL to the database                             | `postgres://rustfuif:secret@127.0.0.1/rustfuif` | ``                               |
| ✓        | `SESSION_PRIVATE_KEY` | secret used for cookies(minimum 32 characters)  | `...random_characters...`                       | ``                               |
| ✗        | `REDIS_URL`           | Redis cache URL                                 | `redis://redis`                                 | ``                               |

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
