# Ubuntu 22.04 provides glibc 2.35, compatible with ONNX Runtime 1.23.2
# (requires glibc 2.27+, CXXABI_1.3.11, GLIBCXX_3.4.22)
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV ORT_LIB_LOCATION=/opt/onnxruntime/lib
ENV ORT_PREFER_DYNAMIC_LINK=true

# Install cross-compilation toolchain and build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    cmake \
    curl \
    git \
    pkg-config \
    libc6-dev-arm64-cross \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

# Install Rust via rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cross-compilation target
RUN rustup target add aarch64-unknown-linux-gnu

# Download ONNX Runtime 1.23.2 (now compatible with glibc 2.35)
RUN apt-get update && apt-get install -y --no-install-recommends \
    wget \
    && mkdir -p /opt/onnxruntime \
    && wget -q https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-linux-aarch64-1.23.2.tgz \
    && tar xzf onnxruntime-linux-aarch64-1.23.2.tgz -C /opt/onnxruntime --strip-components=1 \
    && rm onnxruntime-linux-aarch64-1.23.2.tgz \
    && apt-get purge -y wget \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

# Set library path for ONNX Runtime
ENV LD_LIBRARY_PATH=/opt/onnxruntime/lib:${LD_LIBRARY_PATH}
