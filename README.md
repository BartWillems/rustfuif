# Rustfuif

![Rust](https://github.com/BartWillems/rustfuif/workflows/Rust/badge.svg?branch=master)

## Development

``` bash
docker-compose up

# cargo install cargo-watch
cargo watch -x run
```

### Configuration

| Since   | Variable              | Description                                          | Example                                             | Default     |
| ------- | --------------------- | ---------------------------------------------------- | --------------------------------------------------- | ----------- |
| `0.1.0` | `API_HOST`            | The hostname/ip address the rustfuif listens on      | `0.0.0.0`                                           | `localhost` |
| `0.1.0` | `RUST_LOG`            | The port the rustfuif listens on                     | `80`                                                | `8080`      |
| `0.1.0` | `DATABASE_URL`        | URL to the database                                  | `postgres://rustfuif:secret@127.0.0.1/rustfuif-dev` | ``          |
| `0.1.0` | `REDIS_HOST`          | Redis Host for cookie storage/cache                  | `localhost`                                         | ``          |
| `0.1.0` | `REDIS_PORT`          | Redis Port                                           | `6379`                                              | ``          |
| `0.1.0` | `SESSION_PRIVATE_KEY` | Private key used for cookies (minimum 32 characters) | `qmslkdj....rgerg`                                  | ``          |

## Market influence ideas

* time based (every n-seconds, update prices)
* purchase count based (every n-purchases, update prices)
* update prices on every purchase

## Game Configuration/variables

* inflation rate, how fast do the prices rice & fall
* special events rate, eg: corona virus outbreak, all pricess fall