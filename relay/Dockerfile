# ### !# This file must be run from the root directory, not inside relay/
# ##! ❯ docker build -t nostr-relay -f relay/Dockerfile .  

# # Get base image from Cross to cross compile to linux
# FROM --platform=linux/amd64 ghcr.io/cross-rs/x86_64-unknown-linux-gnu:main as builder
# # Install necessary packages
# RUN apt-get update && apt-get install -y build-essential checkinstall zlib1g-dev openssl pkg-config libssl-dev
# WORKDIR /usr/src/nostr-relay

# # Copy the local application code into the container
# COPY ./nostr-sdk ./nostr-sdk
# COPY ./relay ./relay

# # RUN cargo install --path ./relay
# RUN rustup target add x86_64-unknown-linux-gnu
# RUN rustup default nightly
# RUN cross build --release --target x86_64-unknown-linux-gnu

# Second stage
FROM alpine:latest

# Copy from builder
COPY /target/x86_64-unknown-linux-gnu/release/relay .

# Specify the command to run when the container starts
CMD ["./relay"]