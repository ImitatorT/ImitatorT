FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev ca-certificates upx
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN upx --best --lzma /app/target/x86_64-unknown-linux-musl/release/imitatort_stateless_company

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/imitatort_stateless_company /agent
ENTRYPOINT ["/agent"]
