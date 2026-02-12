# ImitatorT Stateless Virtual Company Framework

基于 Rust 的无状态虚拟公司框架，围绕以下目标设计：

- **Swarms 风格编排**：Agent 每次运行只处理单轮任务，不保存本地状态。
- **Conduwuit 作为状态中枢**：全部上下文通过 Matrix 房间历史回放构建。
- **MCP 工具接口**：通过 STDIO 按需调用外部工具，避免常驻进程。
- **1GB RAM 优化部署**：提供 Conduwuit RocksDB 内存限制与容器资源配额。

## Quick Start

```bash
cp .env.example .env
cargo run -- \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token <token> \
  --matrix-room-id '!room:matrix.local' \
  --openai-api-key <api_key>
```

## Docker

```bash
docker compose up --build
```

## 架构流

1. Agent 从 Matrix 房间拉取最近 `N` 条消息。
2. 可选执行一次 MCP STDIO 工具。
3. 调用 LLM 推理。
4. 将结论写回房间，作为下一轮上下文。

详见 `docs/architecture.md`。

## Build Notes (MSRV)

- 最低 Rust 版本为 **1.83**（由依赖树决定）。
- Agent Docker 构建镜像已使用 `rust:1.85-alpine`，用于规避 `icu_normalizer_data` 在 `rustc 1.75` 下的构建失败。
- 如果你在本地直接 `cargo build`，请确认 `rustc --version` >= 1.83。
