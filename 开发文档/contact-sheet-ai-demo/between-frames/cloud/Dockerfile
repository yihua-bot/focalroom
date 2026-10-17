# M4/M5 云 API：中国镜像加速 + 多阶段构建
FROM rust:slim AS builder
WORKDIR /build
# rsproxy 中国镜像（解决 crates.io 下载慢）
RUN mkdir -p /usr/local/cargo && \
    printf '[source.crates-io]\nreplace-with = "rsproxy"\n[source.rsproxy]\nregistry = "sparse+https://rsproxy.cn/index/"\n' > /usr/local/cargo/config.toml
COPY Cargo.toml ./
COPY static/ ./static/
COPY src/ ./src/
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/bf-cloud /usr/local/bin/bf-cloud
ENV BF_CLOUD_DB=/data/bf-cloud.sqlite
ENV BF_PREVIEW_DIR=/data/previews
ENV BF_CLOUD_PORT=3001
EXPOSE 3001
CMD ["bf-cloud"]
