# Rustfuif

![Rust](https://github.com/BartWillems/rustfuif/workflows/Rust/badge.svg?branch=master)

![alt text](logo.png "Rustfuif Logo")

## Development

```bash
docker-compose up

# cargo install cargo-watch
cargo watch -x run
```

### Configuration

| Since   | Variable              | Description                                     | Example                                         | Default                          |
| ------- | --------------------- | ----------------------------------------------- | ----------------------------------------------- | -------------------------------- |
| `0.1.0` | `API_HOST`            | The hostname/ip address the rustfuif listens on | `0.0.0.0`                                       | `localhost`                      |
| `0.1.0` | `API_PORT`            | The port the rustfuif listens on                | `80`                                            | `8080`                           |
| `0.1.0` | `RUST_LOG`            | loglevel for different crates                   | `rustfuif=info`                                 | `rustfuif=debug,actix_web=debug` |
| `0.1.0` | `DATABASE_URL`        | URL to the database                             | `postgres://rustfuif:secret@127.0.0.1/rustfuif` | ``                               |
| `0.1.0` | `SESSION_PRIVATE_KEY` | secret used for cookies(minimum 32 characters)  | `...random_characters...`                       | ``                               |
| `0.1.0` | `OPENTELEMETRY_AGENT` | opentelemetry tracing endpoint                  | `jaeger:6831`                                   | `127.0.0.1:6831`                 |

### Observability

- collect jaeger traces by setting the OPENTELEMETRY_AGENT
- /metrics constains prometheus metrics
- /health returns http 200
- /stats shows the following live stats:
  - total handled requests
  - active websocket connections
  - active games
  - active db connections
  - idle db connections
- more logging can be acquired by setting the RUST_LOG to a lower loglevel, eg:
  - RUST_LOG="actix_server=info,actix_web=trace,rustfuif=trace"
  - RUST_LOG="debug"
  - ...

## Market influence ideas

- time based (every n-seconds, update prices)
- purchase count based (every n-purchases, update prices)
- update prices on every purchase

## Game Configuration/variables

- inflation rate, how fast do the prices rise & fall
- special events rate, eg: corona virus outbreak, all pricess fall
