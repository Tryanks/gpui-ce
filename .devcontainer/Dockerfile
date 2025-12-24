FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

USER root

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        cmake \
        curl \
        elfutils \
        gettext-base \
        git \
        jq \
        libasound2-dev \
        libfontconfig-dev \
        libgit2-dev \
        libsqlite3-dev \
        libssl-dev \
        libvulkan-dev \
        libvulkan1 \
        libwayland-dev \
        libx11-xcb-dev \
        libxkbcommon-x11-dev \
        libzstd-dev \
        make \
        mold \
        musl-dev \
        musl-tools \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

USER vscode

# Match the pinned toolchain in `rust-toolchain.toml`.
RUN rustup toolchain install 1.91.1 --profile minimal --component rustfmt --component clippy \
    && rustup default 1.91.1 \
    && rustup target add wasm32-wasip2 x86_64-unknown-linux-musl
