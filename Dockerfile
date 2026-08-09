FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && apt-get install -y libxml2-dev libxmlsec1-dev libxmlsec1-openssl pkg-config libclang-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p godwit-bin

# trixie (Debian 13) ships GLIBC >= 2.39, matching the cargo-chef builder's libc
# so the release binary can actually load on the runtime image.
FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates curl libxml2 libxmlsec1 libxmlsec1-openssl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/godwit /usr/local/bin/godwit
COPY config.example.yaml /app/config.yaml
ENV CONFIG_PATH=/app/config.yaml
EXPOSE 3000
CMD ["godwit"]
