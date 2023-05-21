# ### !# This file must be run from the root directory, not inside relay/
# ##! ❯ docker build -t nostr-relay -f relay/Dockerfile .  

FROM lukemathwalker/cargo-chef:latest-rust-1.69.0@sha256:201a896eddfbd24926414df955d491f427aff604aca81c57f42cce4ab6bc3dbc AS chef
WORKDIR /app

FROM chef AS planner
# Install necessary packages
COPY ./relay/cross_pre_build.sh /app/cross_pre_build.sh
RUN /app/cross_pre_build.sh
# Copy necessary data
COPY ./nostr-sdk .
COPY ./relay .
WORKDIR /app/relay
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/relay/recipe.json /app/relay/recipe.json
# Build dependencies - this is the caching Docker layer!
WORKDIR /app/relay
COPY nostr-sdk /app/nostr-sdk
COPY relay .
RUN RUST_BACKTRACE=full cargo chef cook --release --target x86_64-unknown-linux-gnu --recipe-path recipe.json
# Build application
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin relay

# We do not need the Rust toolchain to run the binary!
FROM debian:buster-slim AS runtime
COPY --from=builder /app/relay/target/x86_64-unknown-linux-gnu/release/relay /usr/local/bin/relay
CMD /usr/local/bin/relay