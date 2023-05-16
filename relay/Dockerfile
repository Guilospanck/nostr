### !# This file must be run from the root directory, not inside relay/
##! ❯ docker build -t nostr-relay -f relay/Dockerfile .  
FROM rust:1.68.0

# Set the working directory in the container
WORKDIR /app

# Copy the local application code into the container
COPY ./nostr-sdk ./nostr-sdk
COPY ./relay ./relay

# Build the Rust application
WORKDIR /app/relay
RUN cargo build --release

# Specify the command to run when the container starts
CMD ["./target/release/relay"]