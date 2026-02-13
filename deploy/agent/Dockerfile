# 使用 cargo-chef 实现更好的层缓存
# 依赖层仅在 Cargo.toml/Cargo.lock 变化时重新构建

FROM rust:1.85-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev ca-certificates upx gcc
RUN cargo install cargo-chef
RUN ln -sf /usr/bin/x86_64-alpine-linux-musl-gcc /usr/bin/x86_64-linux-musl-gcc \
    && ln -sf /usr/bin/x86_64-alpine-linux-musl-gcc /usr/bin/x86_64-unknown-linux-musl-gcc \
    && ln -sf /usr/bin/x86_64-alpine-linux-musl-gcc /usr/bin/musl-gcc
ENV CC_x86_64_unknown_linux_musl=/usr/bin/x86_64-alpine-linux-musl-gcc \
    CC_x86_64_linux_musl=/usr/bin/x86_64-alpine-linux-musl-gcc \
    CC=/usr/bin/x86_64-alpine-linux-musl-gcc \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=/usr/bin/x86_64-alpine-linux-musl-gcc

#  planner 阶段：生成依赖配方
FROM chef AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
RUN cargo chef prepare --recipe-path recipe.json

# builder 阶段：利用缓存构建
FROM chef AS builder
WORKDIR /app

# 先复制并构建依赖（这层会被缓存）
COPY --from=planner /app/recipe.json recipe.json
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# 再复制源码（这层变化频繁）
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples

# 构建主程序和示例
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl --example werewolf

# UPX 压缩
RUN upx --best --lzma /app/target/x86_64-unknown-linux-musl/release/imitatort
RUN upx --best --lzma /app/target/x86_64-unknown-linux-musl/release/examples/werewolf

# 最终镜像
FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/imitatort /agent
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/examples/werewolf /werewolf
ENTRYPOINT ["/agent"]
