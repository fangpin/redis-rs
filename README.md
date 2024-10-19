# Build Your Own Redis

This project is to build a toy Redis-Server clone that's capable of handling
basic commands like `PING`, `SET` and `GET`. Also implement the event loops, the Redis protocol and more.

## Prerequisites
install `redis-cli` first (an implementation of redis client for test purpose)
```sh
cargo install mini-redis
```


## Supported Commands
```sh
redis-cli PING
```

```sh
redis-cli ECHO hey
```

```sh
redis-cli SET foo bar
```

```sh
redis-cli SET foo bar px 100
```

```sh
redis-cli GET foo
```