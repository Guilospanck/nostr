# ### !# This file must be run from the root directory, not inside `relay`
# ### !# As it needs access to `nostr-sdk` as well.                     

FROM lukemathwalker/cargo-chef:latest-rust-1.69.0@sha256:201a896eddfbd24926414df955d491f427aff604aca81c57f42cce4ab6bc3dbc AS chef
WORKDIR /app

FROM chef AS planner
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
FROM debian:bookworm-slim@sha256:b396b38adb888af839a4b967ba919c49086987126500d6cd8f662531e202d038 AS runtime
# Install necessary packages
COPY ./relay/pre_build.sh /app/pre_build.sh
RUN /app/pre_build.sh
COPY --from=builder /app/relay/target/x86_64-unknown-linux-gnu/release/relay /usr/local/bin/relay
ENTRYPOINT [ "/usr/local/bin/relay" ]