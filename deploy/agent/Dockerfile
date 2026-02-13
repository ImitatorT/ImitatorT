# 多阶段构建 Dockerfile
# 支持 linux/amd64 和 linux/arm64 多平台

# ========== 基础阶段：设置交叉编译环境 ==========
FROM --platform=$BUILDPLATFORM rust:1.85-alpine AS base
RUN apk add --no-cache musl-dev pkgconfig openssl-dev ca-certificates upx gcc
RUN cargo install cargo-chef

# 根据目标平台设置环境变量
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") \
        echo "x86_64-unknown-linux-musl" > /target.txt && \
        ln -sf /usr/bin/x86_64-alpine-linux-musl-gcc /usr/bin/musl-gcc ;; \
    "linux/arm64") \
        echo "aarch64-unknown-linux-musl" > /target.txt && \
        apk add --no-cache aarch64-linux-musl-gcc && \
        ln -sf /usr/bin/aarch64-alpine-linux-musl-gcc /usr/bin/musl-gcc ;; \
    *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac

# ========== planner 阶段：生成依赖配方 ==========
FROM base AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
RUN cargo chef prepare --recipe-path recipe.json

# ========== builder 阶段：编译 ==========
FROM base AS builder
WORKDIR /app

# 复制目标平台信息
COPY --from=base /target.txt /target.txt

# 添加 Rust 目标平台
RUN rustup target add $(cat /target.txt)

# 先复制并构建依赖（这层会被缓存）
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target $(cat /target.txt) --recipe-path recipe.json

# 再复制源码（这层变化频繁）
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples

# 构建主程序和示例
RUN cargo build --release --target $(cat /target.txt) && \
    cargo build --release --target $(cat /target.txt) --example werewolf

# 复制到固定位置
RUN TARGET=$(cat /target.txt) && \
    cp /app/target/$TARGET/release/imitatort /tmp/imitatort && \
    cp /app/target/$TARGET/release/examples/werewolf /tmp/werewolf

# UPX 压缩（仅在 amd64 平台）
RUN if [ "$(cat /target.txt)" = "x86_64-unknown-linux-musl" ]; then \
        upx --best --lzma /tmp/imitatort && \
        upx --best --lzma /tmp/werewolf; \
    fi

# ========== 最终镜像 ==========
FROM scratch

# 构建参数（用于注入版本信息）
ARG BUILD_DATE
ARG VCS_REF
ARG VERSION

# 复制 CA 证书和编译好的二进制文件
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /tmp/imitatort /agent
COPY --from=builder /tmp/werewolf /werewolf

# OCI 镜像标签
LABEL org.opencontainers.image.title="ImitatorT Stateless Agent"
LABEL org.opencontainers.image.description="无状态虚拟公司框架 Agent"
LABEL org.opencontainers.image.source="https://github.com/zhengui666/ImitatorT"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.revision="${VCS_REF}"
LABEL org.opencontainers.image.version="${VERSION}"

ENTRYPOINT ["/agent"]
