# nostr

## TODO

- [x] Change `unreachable()` at line 80 of `relay.rs`. Whenever someone sends something that cannot be parsed as EVENT, REQUEST or CLOSE, it breaks
- [x] Fix `close` message closing the connection even with different id
- [ ? ] Verify `PoisonError` when client closes connection with Ctrl C
- [x] Should not allow a client to close the connection of another client (`subscription_id`)
- [x] Fix relay line 203 and above not sending message to matched filters
- [x] Add received `event` message to the struct of the `ClientConnectionInfo`
- [x] Create a way of storing also the pubkeys of clients
- [x] Filters must be related to the `subscription_id` because we need to have a way of deleting them when `CLOSE` message is sent
- [x] Client REQ message can have multiple filters
- [ ] Should save all events (disconnected clients) in another structure because in the case a client disconnects from the relay, we won't have `ClientConnectionInfo` anymore (because the client is not connected anymore) and, therefore, if we want to preserve the events and send to someone else afterwards, we will need to have this info, otherwise it will be lost. (**Depends on having the public key of the client before**)
- [ ] Use `limit` from filter on first request of events and return the most recent ones up until the number defined by this value
- [ ] Improve error and normal functioning logging
- [ ] Finish the implementation of all the required NIPs (just `NIP01`)
- [ ] Implement `optional` NIPs

## Some info to pay attention

- Clients should only open one connection to each relay. Each connection can support an unlimited number of subscriptions (from that client).
- Request message can have multiple filters.

## How to run

Go to `relay`:

```bash
cargo run
```

it will start listening on the `127.0.0.1:8080` or you can also pass the `host:port` to it like:

```bash
cargo run 127.0.0.1:8080
```

Then go to `client` and run:

```bash
cargo run
```

Client will try to connect automatically to the following addresses (therefore, don't change your `relay` right now
to a different address other than `127.0.0.1:8080` or `127.0.0.1:8081` :P or then change the `client` implementation to read from args):

```rs
pub const LIST_OF_RELAYS: [&str; 2] = ["ws://127.0.0.1:8080/", "ws://127.0.0.1:8081/"];
```

## Debugging

`CMD/Ctrl P` then `>Debug: Select and Start Debugging`. Then you can choose which part (client or relay) you wanna debug.
