# nostr

## TODO

### Required

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
- [x] [CLIENT] Change this serialization to transform a vector into a spread of objects (...filter). NEED TO TEST
- [x] [RELAY] When sending events requested to client, do not send it as a vector.
- [x] [RELAY] Verify [`UnboundedSender<Message>`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.UnboundedSender.html) for tips on dealing with closed connection.
- [x] [RELAY] Add listener to CTRL C.
- [x] [RELAY] Fix JSON `as_json` and `from_json` of communications.
- [x] [RELAY] Check filter function on `REQ` message. I suppose it is not working OR
the database is not saving events properly. Example:

```json
["EVENT",{"kind":1,"content":"Hello modafoca","tags":[],"created_at":1684144532,"pubkey":"5081ce98f7da142513444079a55e2d1676559a908d4f694d299057f8abddf835","id":"2c53b58e0882b75b6540659ec0f4217d41000a12497ecbcabe9574384839273c","sig":"054a5e289356e5b0cb3a5b5e71e07e91b178c67c236bc2c77f98faeef418439fb9a944f054f0f010d08dfbc8fb68e36afaf485be24f1526f38134df61a58c311"}]

["REQ","5968712077837064",{"authors":["5081ce98f7da142513444079a55e2d1676559a908d4f694d299057f8abddf835"],"kinds":[1,6]}]
```

It is working. The problem was with sending the message as BINARY to the client.

- [x] [RELAY] ~~I believe I'm sending too many events at a time. Need to check.~~ No, I'm doing fine.
- [x] [RELAY] Add `nginx.conf` to the repository -> remember also when setting up a new nginx server that if certbot is not finding it, is because probably because you are using `www.<domain>` and not only `<domain>`. Also, remember about the `Proxied` thing of cloudfare.
- [x] [CLIENT/RELAY] Improve error and normal functioning logging/handling (`env::logger`, `tracing`).
- [x] [ALL] Create Makefiles (just like the `relay` one) and change githooks to use them.
- [x] [CLIENT/RELAY] Check why GithubActions is failing. Didn't do anything. Just worked again.
- [x] [RELAY] Maybe I can close a channel by sending `tx.unbounded_send(Message::Close()).unwrap()`.
- [x] [RELAY] Should check event to verify if the signature is valid.
- [x] [RELAY] Fix Dockerfile. Take a look at [this](https://github.com/scsibug/nostr-rs-relay/blob/master/Dockerfile) for an example.
- [x] [RELAY] Generate binary release with Github Actions. See this [example](https://github.com/Asone/nostrss/blob/main/.github/workflows/release.yml)
- [ ] [CLIENT] Should sign events properly.
- [ ] [CLIENT] Send `METADATA` when connecting to RELAY.
- [ ] [CLIENT] When client is sending message, it is alternating between different relays <--.
- [ ] [CLIENT] Clients should NOT be allowed to open more than one connection to the same server.
- [ ] [CLIENT] Should save its own filters in order to request data from different relays.
- [ ] [CLIENT] Should have a way of handling duplicated events, since a client can be connected to multiple relays.
- [ ] [CLIENT] Must validate signature.
- [ ] [CLIENT/RELAY] Finish the implementation of all the required NIPs (just `NIP01`)

### Improvements

- [ ] [CLIENT/RELAY/SDK] Use cargo `workspaces` (maybe).
- [ ] [CLIENT] Should save its own events.
- [ ] [RELAY] Improve cross-compilation to darwin and windows.
- [ ] [RELAY] Change the way Relay reads from DB (putting all that data in memory is not the best case scenario).
- [ ] [CLIENT/RELAY] Add data validation to prevent panics.
- [ ] [CLIENT/RELAY] Check `tracing`.
- [ ] [CLIENT/RELAY] Implement `optional` NIPs
- [ ] [CLIENT] Create abstraction function to follow someone(i.e.: send a new REQ message with a filter requiring its pubkey).

## NIPs implemented

- [ ] NIP01
- [ ] NIP10

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
