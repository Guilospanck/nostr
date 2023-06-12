# nostr [![codecov](https://codecov.io/gh/Guilospanck/nostr/branch/main/graph/badge.svg?token=1CF85SBYD9)](https://codecov.io/gh/Guilospanck/nostr)

Yet another Nostr implementation in Rust.

## NIPs implemented

- [x] NIP01
- [x] NIP10

## How to run

Both `client`, `relay` and `nostr-sdk` read from `.env` variables to work. If it is not found, it is going to use the default values.
You can find `.env` example files inside each folder with the name `example.env`. Create a `.env` file inside each of them with the values
you desire.

```bash
##! Inside each folder
cp example.env .env
```

### Relay

```bash
make relay-run
```

Will start listening on the value defined by the `RELAY_HOST` environment variable. If it doesn't find it, will default to `0.0.0.0:8080`.

### Client

```bash
make client-run
```

Client will try to connect automatically to the addresses defined by the `RELAY_LIST` environment variable. If: If it doesn't find it, will default to `ws://127.0.0.1:8080/`.

## Debugging

`CMD/Ctrl P` then `>Debug: Select and Start Debugging`. Then you can choose which part (client or relay) you wanna debug.
