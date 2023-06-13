# nostr [![codecov](https://codecov.io/gh/Guilospanck/nostr/branch/main/graph/badge.svg?token=1CF85SBYD9)](https://codecov.io/gh/Guilospanck/nostr)

Yet another Nostr implementation in Rust.

## FAQ

### Why

Why not?

> No, just kidding, not going to be one of those people.

I believe the best way to understand something is to actually implement it. Nostr really got my attention and as I'm willing to improve my Rust (_and not be rusty_ 🥁), I decided to delve into the protocol and implement it in this ~~fkn crazy, don't know WTF I'm doing~~ beautiful language.

### Why Nostr

We live in the best era of all our society history. We can talk to our loving ones from far away with the simple click of your mouse pointer (or the press of your finger); we can fly to distant places in a relatively comfortable and quick way; we can listen to any music we like at any time we want; we can learn from everyone at everytime, from any era, at any pace, virtually cost-free. Life's amazing.

One of the problems, though, is that society tends to centralize things in the hands of a few people, giving them absolute power of the lives of others and, as the saying goes, _absolute power corrupts absolutely_. No matter how "good" someone is, the system itself is corrupt and will eat them alive.

This centralization happens, most of the times, in subtle ways. We start using some social media and before we know it, we're addicted and cannot live without it - and this is not just because some dopamine hit that we can't live without, but some people _literally_ depend on social media, either because it's in there they can find customers, or they sell in these platforms, or even some extrapolation like "knowing when something is going to happen (say a metro strike, therefore I should not leave my home at the same time so I don't get late to get my children at the school)".

When we're denied the use of said platforms, a descending spiral starts to form in our lives and bad things start to happen because of that. But even more than that, it's not just because we apparently won't lose anything that it should be okay in denying one's access to something.

Nostr is a way to fight that. It abides to the natural law of communication and the right to come and go.

- _Some relay doesn't want you there or will block you for X-reasons?_ Jump to another relay and tell the people you want to talk to (and want to talk to you) where you are.

- _You don't like the client you are using but you really want to still be connected to your peers?_ No problem, just choose another client that exists - or create your own.

The protocol is agnostic. It is like nature: _it does not care who you are_. As long as you follow the rules of the implementation of the protocol (laws of nature), you will be allowed to be "alive".

### Why Rust

Even though I only know the basics of Rust, I think it brings such enrichment to the mind of a software developer in a sense that it makes you shift your paradigms of how you think when programming. In that sense, I would say it makes you a better programmer overall.

### What challenges faced so far

The protocol is indeed simple. The [NIP01](https://github.com/nostr-protocol/nips/blob/master/01.md), which states the required basis for the implementation, is not a complex subject - although I would have preferred that document to be a little more detailed/formatted.

The real challenge for me was/is regarding the async operations in Rust. I think that it's a whole pain to learn and to implement and to debug and whatnot. There are so many concepts in action and many of them crash with the concepts of `Ownership` and `Mutation` that Rust has. Once you get something working, it will become a little, little easier to put everything together.

Keep going. I think it will pay off.

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
