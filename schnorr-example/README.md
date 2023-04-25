# Schnorr Signature

This repository shows how to use `Schnorr` signatures defined in Bitcoin [BIP340](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki).

It also shows the similarities with `ECDSA` signatures and how one can be translated into another.

## Brief Description

According to [this passage of BIP340](https://bips.xyz/340#public-key-conversion):

> Public Key Conversion: 
	As an alternative to generating keys randomly, it is also possible and
safe to repurpose existing key generation algorithms for ECDSA in a
compatible way.
	The secret keys constructed by such an algorithm can be used as sk
directly.
	The public keys constructed by such an algorithm (assuming they use the
33-byte compressed encoding) need to be converted by dropping the first
byte. Specifically, BIP32 and schemes built on top of it remain usable.

## Running Tests

```bash
cargo run test --tests
```