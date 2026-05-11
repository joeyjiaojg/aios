FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    build-essential \
    gcc \
    gcc-multilib \
    g++ \
    make \
    qemu-system-x86 \
    grub-common \
    grub-pc-bin \
    xorriso \
    mtools \
    ovmf \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup component add rust-src --toolchain nightly \
 && rustup target add x86_64-unknown-none --toolchain nightly

WORKDIR /aios
