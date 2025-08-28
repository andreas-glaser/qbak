# syntax=docker/dockerfile:1.4
ARG RUST_VERSION=1.71
FROM rust:${RUST_VERSION}-slim AS base
WORKDIR /workspace

RUN apt-get update && apt-get install -y \
    build-essential pkg-config \
    gcc-aarch64-linux-gnu gcc-arm-linux-gnueabihf gcc-mingw-w64-x86-64 \
    musl-tools git libssl-dev ca-certificates curl jq \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

RUN rustup target add \
    x86_64-unknown-linux-musl aarch64-unknown-linux-gnu \
    aarch64-unknown-linux-musl armv7-unknown-linux-gnueabihf \
    x86_64-apple-darwin aarch64-apple-darwin x86_64-pc-windows-gnu

FROM base AS development
WORKDIR /workspace

# Allow matching host UID/GID to avoid permission issues on bind mounts
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN groupadd -g ${GROUP_ID} developer || true && \
    useradd -m -s /bin/bash -u ${USER_ID} -g ${GROUP_ID} developer || true && \
    mkdir -p /home/developer/.cargo && \
    chown -R ${USER_ID}:${GROUP_ID} /workspace /home/developer && \
    echo 'export PATH="/home/developer/.cargo/bin:/usr/local/cargo/bin:$PATH"' > /etc/profile.d/cargo_path.sh

USER developer
ENV CARGO_HOME=/home/developer/.cargo
# Ensure cargo/rustc available for non-root user even when CARGO_HOME is empty volume
ENV PATH="/home/developer/.cargo/bin:/usr/local/cargo/bin:$PATH"

FROM base AS builder
WORKDIR /build
COPY --chown=root:root . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest AS runtime
RUN apk --no-cache add ca-certificates
WORKDIR /root/
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/qbak /usr/local/bin/qbak
ENTRYPOINT ["qbak"]
