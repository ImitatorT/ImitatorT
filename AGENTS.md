# ImitatorT Stateless Virtual Company Framework

基于 Rust 的无状态虚拟公司框架，采用 Swarms 风格编排，以 Conduwuit (Matrix 服务器) 作为状态中枢。

## 项目概述

本项目是一个**无状态智能体框架**，核心设计理念：

- **计算与状态分离**：Agent 为短生命周期计算单元，状态仅保留在 Matrix/Conduwuit
- **无本地持久化**：不落地 SQLite / Vector DB，避免跨实例同步复杂度
- **可弹性扩展**：每个 Agent 副本可随时重启，不依赖本地恢复
- **资源优化**：针对 1GB RAM 环境优化部署

### 核心架构流程

1. Agent 从 Matrix 房间拉取最近 N 条消息作为上下文
2. 可选执行一次 MCP STDIO 工具
3. 调用 LLM 推理
4. 将结论写回房间，作为下一轮上下文

## 技术栈

- **语言**: Rust (MSRV 1.83)
- **异步运行时**: Tokio
- **HTTP 客户端**: reqwest
- **CLI 解析**: clap
- **日志**: tracing
- **Matrix 服务器**: Conduwuit (轻量级 Matrix 服务端)
- **LLM 接口**: OpenAI API
- **容器化**: Docker / Docker Compose

## 项目结构

```
.
├── Cargo.toml              # Rust 项目配置
├── docker-compose.yml      # 部署编排
├── .env.example            # 环境变量模板
├── src/
│   ├── main.rs             # 主入口：单次执行循环
│   ├── config.rs           # CLI/环境变量配置定义
│   ├── matrix.rs           # Matrix Client-Server API 封装
│   ├── llm.rs              # OpenAI Chat Completions 适配层（支持 Tool Calling）
│   └── tool.rs             # Tool/Function Calling 工具定义与执行
├── deploy/
│   ├── agent/
│   │   └── Dockerfile      # Agent 容器构建（多阶段 + UPX 压缩）
│   ├── conduwuit/
│   │   └── conduwuit.toml  # RocksDB 内存与保留策略配置
│   └── one_click_deploy.sh # 一键部署脚本
├── docs/
│   └── architecture.md     # 架构详细文档
└── .github/workflows/
    └── docker-publish.yml  # GHCR 自动发布工作流
```

## 构建与运行

### 本地开发

```bash
# 复制环境变量模板
cp .env.example .env
# 编辑 .env 填入实际值

# 本地运行
cargo run -- \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token <token> \
  --matrix-room-id '!room:matrix.local' \
  --openai-api-key <api_key>
```

### Docker 部署

```bash
# 完整部署（Conduwuit + Agent）
docker compose up --build

# 或使用一键部署脚本
./deploy/one_click_deploy.sh [tag]
```

### 构建兼容性注意事项

- **MSRV**: Rust 1.83（见 `Cargo.toml` 的 `rust-version`）
- **Docker 构建镜像**: 使用 `rust:1.85-alpine`，用于避免 `icu_normalizer_data` 在旧工具链（如 1.75）下编译失败
- 如果在其它分支 cherry-pick 本仓提交，请优先保留：
  1. `Cargo.toml` 中的 `rust-version = "1.83"`
  2. `deploy/agent/Dockerfile` 中的 `FROM rust:1.85-alpine`

## 配置说明

配置通过环境变量或命令行参数传入（使用 clap 解析）：

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `MATRIX_HOMESERVER` | Matrix 服务器地址 | - |
| `MATRIX_ACCESS_TOKEN` | Matrix 访问令牌 | - |
| `MATRIX_ROOM_ID` | 目标房间 ID | - |
| `OPENAI_API_KEY` | OpenAI API 密钥 | - |
| `OPENAI_MODEL` | 模型名称 | gpt-4o-mini |
| `CONTEXT_LIMIT` | 上下文消息数量 | 50 |
| `SYSTEM_PROMPT` | 系统提示词 | （见代码） |


## CI/CD 与发布

GitHub Actions 工作流 `.github/workflows/docker-publish.yml`：

- **触发条件**: `main` 或 `dev` 分支推送
- **发布目标**: GHCR (GitHub Container Registry)
- **镜像标签**:
  - `main` 分支: `latest` + `sha-*`
  - `dev` 分支: `dev` + `sha-*`

## 代码风格与约定

- 使用标准 Rust 格式化：`cargo fmt`
- 错误处理：统一使用 `anyhow::Result`
- 异步函数：使用 `async/await` + Tokio
- 日志记录：使用 `tracing` 宏（info, debug, error 等）
- 中文注释：项目文档和注释主要使用中文

## 模块职责

### `src/config.rs`
定义 `AppConfig` 结构体，使用 `clap::Parser` 派生宏支持从命令行参数和环境变量读取配置。

### `src/matrix.rs`
`MatrixClient` 封装 Matrix Client-Server API：
- `latest_context()`: 获取房间历史消息
- `send_text_message()`: 发送文本消息到房间

### `src/llm.rs`
`OpenAIClient` 封装 OpenAI Chat Completions API：
- `complete()`: 传入系统提示词、上下文和任务，返回 LLM 响应

### `src/tool.rs`
Tool/Function Calling 工具定义与执行：
- `ToolRegistry::get_tools()`: 获取所有可用工具定义
- `ToolRegistry::execute()`: 执行指定的工具调用
- 内置工具：`execute_command`（执行系统命令）、`fetch_url`（获取网页内容）

### `src/main.rs`
主循环逻辑：
1. 初始化 tracing 日志
2. 解析配置
3. 构建 Matrix 和 LLM 客户端
4. 拉取上下文
5. 调用 LLM（带 tools 定义），检测是否需要 tool call
6. 如有 tool call，执行工具并将结果再次传给 LLM 获取最终回复
7. 回写结果到 Matrix

## 部署架构

### 资源限制（docker-compose.yml）

- **Conduwuit**: 600MB 内存限制
- **Agent**: 256MB 内存限制

### Conduwuit 配置

使用 RocksDB 后端，配置内存优化参数：
- `block_cache_capacity_mb = 256`
- `limit_memtables_to_block_cache = true`
- `max_background_jobs = 2`
- 保留策略：timeline 30天，media 7天

### Agent 镜像

多阶段构建：
1. 使用 `rust:1.85-alpine` 编译
2. 使用 UPX 压缩二进制
3. 最终镜像基于 `scratch`（空镜像），仅包含 CA 证书和压缩后的二进制

## 安全注意事项

- **敏感信息**: Matrix Token 和 OpenAI API Key 通过环境变量传入，不要硬编码
- **容器安全**: Agent 最终镜像使用 `scratch`，无 shell、无包管理器，最小攻击面
- **网络**: Conduwuit 默认关闭注册和联邦功能
- **证书**: 生产环境确保证书有效（Agent 镜像内置 CA 证书）

## 扩展建议

- 添加更多 LLM 提供商支持（修改 `src/llm.rs`）
- 扩展 MCP 工具支持（修改 `src/mcp.rs`）
- 添加健康检查端点
- 支持更多 Matrix 事件类型
