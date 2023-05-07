# nostr

## TODO

- [x] Change `unreachable()` at line 80 of `relay.rs`. Whenever someone sends something that cannot be parsed as EVENT, REQUEST or CLOSE, it breaks
- [x] Fix `close` message closing the connection even with different id
- [x] Verify `PoisonError` when client closes connection with Ctrl C
- [x] Should not allow a client to close the connection of another client (`subscription_id`)
- [x] Fix relay line 203 and above not sending message to matched filters
- [x] Add received `event` message to the struct of the `ClientConnectionInfo`
- [x] Create a way of storing also the pubkeys of clients
- [x] Filters must be related to the `subscription_id` because we need to have a way of deleting them when `CLOSE` message is sent
- [x] Client REQ message can have multiple filters
- [x] [RELAY] Should save all events (disconnected clients) in another structure because in the case a client disconnects from the relay, we won't have `ClientConnectionInfo` anymore (because the client is not connected anymore) and, therefore, if we want to preserve the events and send to someone else afterwards, we will need to have this info, otherwise it will be lost.
- [x] [RELAY] Check what `#[serde(untagged)]` does to enum in `event.rs` -> It removes the enum key and prints only the value.
- [x] [CLIENT/RELAY] One thing no note is that the `Tags` and the content are dependent on the `Kind`.
- [x] [RELAY] Use `limit` from filter on first request of events and return the most recent ones up until the number defined by this value
- [x] [CLIENT/RELAY] Tests!
- [ ] [CLIENT] Change this serialization to transform a vector into a spread of objects (...filter)
- [ ] [RELAY] When sending events requested to client, do not send it as a vector.
- [ ] [RELAY] Add listener to CTRL C.
- [ ] [RELAY] Verify [`UnboundedSender<Message>`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.UnboundedSender.html) for tips on dealing with closed connection.
- [ ] [CLIENT] Should sign events properly.
- [ ] [RELAY] Should check event to verify if the signature is valid.
- [ ] [CLIENT/RELAY] Add data validation to prevent panics.
- [ ] [CLIENT] Clients should not be allowed to open more than one connection to the same server.
- [ ] [CLIENT] Should save its own events.
- [ ] [CLIENT] Should save its own filters in order to request data from different relays.
- [ ] [CLIENT] Should have a way of handling duplicated events, since a client can be connected to multiple relays.
- [ ] [CLIENT] Must validate signature.
- [ ] [CLIENT] Create abstraction function to follow someone(i.e.: send a new REQ message with a filter requiring its pubkey).
- [ ] Improve error and normal functioning logging/handling.
- [ ] Finish the implementation of all the required NIPs (just `NIP01`)
- [ ] Implement `optional` NIPs

## NIPs implemented

- [ ] NIP01
- [ ] NIP10

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
