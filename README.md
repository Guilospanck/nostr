# nostr

## How to run

Go to `relay`:

```bash
cargo run
```

it will start listening on the `127.0.0.1:8080` or you can also pass the host:port to it like:

```bash
cargo run 127.0.0.1:8080
```

Then go to `client-example` and run:

```bash
cargo run
```

## Debugging

`CMD/Ctrl P` then `>Debug: Select and Start Debugging`. Then you can choose which part (client or relay) you wanna debug.
