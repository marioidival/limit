FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5

ENV ORT_LIB_LOCATION=/opt/onnxruntime/lib
ENV ORT_PREFER_DYNAMIC_LINK=true

RUN apt-get update && apt-get install -y --no-install-recommends \
    wget \
    ca-certificates \
    && mkdir -p /opt/onnxruntime \
    && wget -q https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-linux-aarch64-1.23.2.tgz \
    && tar xzf onnxruntime-linux-aarch64-1.23.2.tgz -C /opt/onnxruntime --strip-components=1 \
    && rm onnxruntime-linux-aarch64-1.23.2.tgz \
    && apt-get purge -y wget \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*
