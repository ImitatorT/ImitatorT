# 极致优化的多阶段构建 Dockerfile
# 支持 linux/amd64 和 linux/arm64 多平台

# ========== 基础阶段：使用 cargo-chef 缓存依赖 ==========
FROM --platform=$BUILDPLATFORM rust:1.85-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev ca-certificates gcc
RUN cargo install cargo-chef --locked
WORKDIR /app

# ========== planner 阶段：生成依赖配方（极快，只分析 Cargo.toml）==========
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
RUN cargo chef prepare --recipe-path recipe.json

# ========== builder 阶段：编译（使用缓存层）==========
FROM chef AS builder

# 根据目标平台设置交叉编译
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") \
        rustup target add x86_64-unknown-linux-musl && \
        echo "x86_64-unknown-linux-musl" > /target.txt && \
        ln -sf /usr/bin/x86_64-alpine-linux-musl-gcc /usr/bin/musl-gcc ;; \
    "linux/arm64") \
        rustup target add aarch64-unknown-linux-musl && \
        apk add --no-cache aarch64-linux-musl-gcc && \
        echo "aarch64-unknown-linux-musl" > /target.txt && \
        ln -sf /usr/bin/aarch64-alpine-linux-musl-gcc /usr/bin/musl-gcc ;; \
    *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac

# 先构建依赖（这层会被 Docker 缓存，只有 Cargo.toml 变化时才重建）
COPY --from=planner /app/recipe.json recipe.json

# 使用 BuildKit 缓存挂载来缓存 cargo registry 和编译产物
# 这能显著减少重复下载和编译时间
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    TARGET=$(cat /target.txt) && \
    cargo chef cook --release --target $TARGET --recipe-path recipe.json

# 再复制源码（这层变化频繁，但依赖已编译好）
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples

# 并行构建主程序和示例（使用缓存挂载复用之前编译的依赖）
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    TARGET=$(cat /target.txt) && \
    cargo build --release --target $TARGET --bin imitatort --example werewolf && \
    # 从缓存目录复制最终二进制文件到临时位置
    cp /app/target/$TARGET/release/imitatort /tmp/imitatort && \
    cp /app/target/$TARGET/release/examples/werewolf /tmp/werewolf

# UPX 压缩（仅在 amd64 平台，arm64 跳过因为 UPX 支持有限）
RUN TARGET=$(cat /target.txt) && \
    if [ "$TARGET" = "x86_64-unknown-linux-musl" ]; then \
        apk add --no-cache upx && \
        upx --best --lzma /tmp/imitatort && \
        upx --best --lzma /tmp/werewolf; \
    fi

# ========== 最终镜像（最小化）==========
FROM scratch

ARG BUILD_DATE
ARG VCS_REF
ARG VERSION

# 只复制必要的文件
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
