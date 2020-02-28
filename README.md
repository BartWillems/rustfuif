# Rustfuif

## Development

``` bash
docker-compose up

# cargo install cargo-watch
cargo watch -x run
```

## Market influence ideas

* time based (every n-seconds, update prices)
* purchase count based (every n-purchases, update prices)
* purchase based (update prices every purchase)

## Game management ideas

### Free-for-All

Every user can create a game, but only the creator of that game can invite other users to join.
I will probably use this one.

#### Pros

* fun for everyone
* less hassle

### Admin Based

Only the admin can create games, he can then send game URLs to other users(who don't need an account).

#### Pros

* Don't need to care about user management

#### Cons

* annoying for users & the admin

### Game Configuration/variables

* inflation rate, how fast do the prices rice & fall
* special events rate, eg: corona virus outbreak, all pricess fall