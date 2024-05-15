# Build the rust backend
FROM rust:1.77-slim-bookworm as backend
# Required for building 'openssl-sys' crate
RUN apt-get update && apt-get install -y pkg-config libssl-dev
WORKDIR /
RUN USER=root cargo new --bin untis_changes
WORKDIR /untis_changes
COPY ./Cargo.lock ./Cargo.toml ./
RUN cargo build --release && rm src/*.rs target/release/deps/untis_changes*
COPY ./src ./src
RUN cargo build --release

# Create the final image
FROM debian:bookworm-slim
# Install openssl
RUN apt-get update && apt-get install -y openssl ca-certificates && apt-get clean && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /untis_changes/target/release/untis_changes /usr/bin/untis_changes

ENV ROCKET_ADDRESS=0.0.0.0 ROCKET_PORT=80

EXPOSE 80/tcp

ENTRYPOINT ["/usr/bin/untis_changes"]
