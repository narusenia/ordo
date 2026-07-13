FROM rust:1.95-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    g++ \
    ninja-build \
    pkg-config \
    cmake \
    git \
    curl \
    zip \
    unzip \
    tar \
    clang-format \
    clang-tidy \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /ordo
COPY . .

RUN cargo build --release
RUN cp target/release/ordo /usr/local/bin/ordo

WORKDIR /workspace

ENTRYPOINT ["ordo"]
