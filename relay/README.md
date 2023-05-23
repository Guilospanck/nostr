# Relay

This is the relay part of the Nostr protocol. Check [NIPs implemented](#nips-implemented) to see which NIPs are currently implemented.

## Running

There are basically three (3) ways you can run it at the moment, using [Local Rust](#local-rust), [Docker](#docker) or [Compiled](#compiled).

### Local Rust

Here you'll need [Rust](https://www.rust-lang.org/tools/install). Just clone the repository, go into the `relay/` folder and run `make run`. The server will start listening on `0.0.0.0:8080`.

### Docker

For this part you'll need [Docker engine](https://docs.docker.com/engine/install/). One important thing to notice here is that you need to run the `docker build` command from the root (one level above `relay/`), as it needs to have access to `nostr-sdk` folder as well to build `relay`. You can run do it by running:

```sh
# root directory
docker build -t nostr-relay -f relay/Dockerfile . 
```

> To build it for different targets you can check the `build/` folder.

### Compiled

It is as simple as:

```bash
make build-debug-mode
# or
make build-release-mode
```

Then you can run it with:

```bash
./target/{release | debug}/relay 
# or
make cargo run --release
# or
make cargo run
```

Or just copy the binary and run it somewhere.

> See [Cross-Compilation](#cross-compilation) to build it for different architectures.

## NIPs implemented

As the relay is more of a "dumb rock", the most necessary thing for it to work is to implement the NIP01. Virtually speaking, it can work with any client that implements the Nostr protocol. Whenever it has some specific tag that it doesn't know (i.e.: it is not `e` or `p` tags), it will parse it into a custom tag.

- [x] NIP 01
- [x] NIP 10

## Cross-compilation

### Requirements

This whole step is condensed into the `Makefile` script. It may take a while on first iteration.

You will need some libraries to compile this:

- `Cross`:

```sh
cargo install cross --git https://github.com/cross-rs/cross
```

The configuration for `Cross` resides in `./Cross.toml`.

- `Docker buildx`:

```sh
brew install docker-buildx
```

Then, run the caveats:

```sh
mkdir -p ~/.docker/cli-plugins
ln -sfn /opt/homebrew/opt/docker-buildx/bin/docker-buildx ~/.docker/cli-plugins/docker-buildx
```

Also, be sure to have in your `.rc` file (`.bashrc`, `.zshrc`) if using `colima`:

```sh
export DOCKER_HOST="unix://$HOME/.colima/docker.sock"
```

### Compiling

```sh
make compile-to-x86_64-unknown-linux-gnu
```

Libs necessary to compile the project (present in the `pre_build.sh` file):

```sh
apt-get update
apt-get install -y wget build-essential checkinstall zlib1g-dev openssl pkg-config libssl-dev
# see http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/?C=M;O=D
wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb
dpkg -i libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb
```

After running the `make` command, the compiled application will be available at `./target/x86_64-unknown-linux-gnu/release/relay`. Just copy it and run in a `x86_64` linux machine.

### Thoughts on running it on the server machine

You can spin it up as a [`systemd` service](https://www.shellhacks.com/systemd-service-file-example/).
