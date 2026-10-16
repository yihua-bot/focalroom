# M4 云 API：多阶段构建（Rust 编译 → 轻量运行时）
FROM rust:slim AS builder
WORKDIR /build
COPY Cargo.toml ./
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
