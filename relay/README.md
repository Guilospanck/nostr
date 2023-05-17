# Relay

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

## Cross-compilation

This whole step is condensed into the `Makefile` script. It may take a while on first iteration. Just run:

```sh
make compile-to-x86_64-unknown-linux-gnu
```

Libs necessary to compile the project (present in the `cross-pre_build.sh` file):

```sh
apt-get update
apt-get install -y wget build-essential checkinstall zlib1g-dev openssl pkg-config libssl-dev
# see http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/?C=M;O=D
wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb
dpkg -i libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb
```

After running the `make` command, the compiled application will be available at `./target/x86_64-unknown-linux-gnu/release/relay`. Just copy it and run in a `x86_64` linux machine.
