# Stateless Virtual Company Architecture

## 设计原则

- **计算与状态分离**：Agent 为短生命周期计算单元，状态仅保留在 Matrix/Conduwuit。
- **无本地持久化**：不落地 SQLite / Vector DB，避免跨实例同步复杂度。
- **可弹性扩展**：每个 Agent 副本可随时重启，不依赖本地恢复。

## 组件

- `src/main.rs`：单次执行循环（拉上下文 -> 推理 -> 回写）。
- `src/matrix.rs`：直接调用 Matrix Client-Server API 获取消息/发送消息。
- `src/llm.rs`：OpenAI Chat Completions 适配层。
- `src/mcp.rs`：STDIO 模式的外部工具桥接。
- `deploy/conduwuit/conduwuit.toml`：RocksDB 内存与保留策略。
- `docker-compose.yml`：Conduwuit + Agent 资源隔离部署。
- `.github/workflows/docker-publish.yml`：`main`/`dev` 自动发布 GHCR 镜像。

## 分支发布策略

- 推送 `main`：发布 `latest` + `sha-*`。
- 推送 `dev`：发布 `dev` + `sha-*`。

## 构建兼容性（避免冲突分支回退）

- 项目 MSRV 为 **Rust 1.83**（见 `Cargo.toml` 的 `rust-version`）。
- Agent 镜像构建使用 `rust:1.85-alpine`，用于避免 `icu_normalizer_data` 在旧工具链（如 1.75）下编译失败。
- 如果你在其它分支 cherry-pick 本仓提交，请优先保留以下两项：
  1. `Cargo.toml` 中的 `rust-version = "1.83"`
  2. `deploy/agent/Dockerfile` 中的 `FROM rust:1.85-alpine`
