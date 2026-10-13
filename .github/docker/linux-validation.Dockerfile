FROM rust:1.95-bookworm@sha256:6258907abe69656e41cd992e0b705cdcfabcbbe3db374f92ed2d47121282d4a1

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt

WORKDIR /workspace
ENV CARGO_TARGET_DIR=/tmp/terminal-rust-target
