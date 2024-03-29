# ### !# This file must be run from the root directory, not inside relay/                   

# Using `linux/amd64` OS arch (via digest)
FROM lukemathwalker/cargo-chef:latest-rust-1.69.0@sha256:eb4708b22ba93755b7f54be41de576ac504cc7567f18b041a4fc3b5d69a2e2c6 AS chef
WORKDIR /app

FROM chef AS planner
WORKDIR /app/relay
# Copy necessary data
COPY ./crates/nostr-sdk .
COPY ./crates/relay .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/relay/recipe.json /app/relay/recipe.json
# Build dependencies - this is the caching Docker layer!
WORKDIR /app/relay
COPY nostr-sdk /app/nostr-sdk
COPY relay .
RUN RUST_BACKTRACE=full cargo chef cook --release --recipe-path recipe.json
# Build application
RUN cargo build --release --bin relay

# We do not need the Rust toolchain to run the binary!
# Using `linux/amd64` OS arch (via digest)
FROM debian:bookworm-slim@sha256:b396b38adb888af839a4b967ba919c49086987126500d6cd8f662531e202d038 AS runtime
# Install necessary packages
COPY ./pre_build.sh /app/pre_build.sh
RUN /app/pre_build.sh
COPY --from=builder /app/relay/target/release/relay /usr/local/bin/relay
ENTRYPOINT [ "/usr/local/bin/relay" ]