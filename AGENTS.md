# ImitatorT Stateless Virtual Company Framework

基于 Rust 的无状态虚拟公司框架，采用 Swarms 风格编排，支持 A2A (Agent-to-Agent) 协议作为内部通信机制，Matrix 仅作为前端展示层。

## 项目概述

本项目是一个**无状态智能体框架**，核心设计理念：

- **计算与状态分离**：Agent 为短生命周期计算单元，状态仅保留在内部存储
- **灵活的输出模式**：支持 Matrix 前端、命令行输出、A2A 协议多种模式
- **A2A 协议支持**：Agent 间通过 A2A 协议通信，Matrix 仅作为展示层
- **可弹性扩展**：每个 Agent 副本可随时重启，不依赖本地恢复
- **资源优化**：针对 1GB RAM 环境优化部署

### 核心架构流程

1. Agent 从内部存储获取最近 N 条消息作为上下文
2. 可选执行一次 MCP STDIO 工具
3. 调用 LLM 推理
4. 将结论写入输出通道（Matrix/CLI/A2A），作为下一轮上下文

## 技术栈

- **语言**: Rust (MSRV 1.85)
- **异步运行时**: Tokio
- **HTTP 客户端**: reqwest
- **CLI 解析**: clap
- **日志**: tracing
- **Matrix 服务器**: Conduwuit (轻量级 Matrix 服务端) - 仅作为前端
- **LLM 接口**: OpenAI API
- **Agent 协议**: A2A (Agent-to-Agent) 简化实现
- **存储**: 内存存储（默认）/ sled 持久化（可选特性）
- **容器化**: Docker / Docker Compose

## 项目结构

```
.
├── Cargo.toml              # Rust 项目配置
├── docker-compose.yml      # 部署编排
├── .env.example            # 环境变量模板
├── src/
│   ├── main.rs             # 主入口：支持多种运行模式
│   ├── config.rs           # CLI/环境变量配置定义
│   ├── matrix.rs           # Matrix Client-Server API 封装（前端展示）
│   ├── llm.rs              # OpenAI Chat Completions 适配层（支持 Tool Calling）
│   ├── tool.rs             # Tool/Function Calling 工具定义与执行
│   ├── a2a.rs              # A2A 协议简化实现
│   ├── output.rs           # 输出抽象层（Matrix/CLI/A2A）
│   └── store.rs            # 轻量级消息存储（内存/sled）
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

# 命令行模式（最简单，无需 Matrix）
cargo run -- \
  --output-mode cli \
  --openai-api-key <api_key> \
  --input-message "Hello, Agent!"

# 交互式命令行模式
cargo run -- \
  --output-mode cli \
  --openai-api-key <api_key> \
  --interactive

# Matrix 前端模式
cargo run -- \
  --output-mode matrix \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token <token> \
  --matrix-room-id '!room:matrix.local' \
  --openai-api-key <api_key>

# A2A 协议模式（Agent 间通信）
cargo run -- \
  --output-mode a2a \
  --agent-id agent-001 \
  --agent-name "Agent One" \
  --a2a-peer-agents "agent-002,agent-003" \
  --openai-api-key <api_key>

# 混合模式（Matrix 前端 + A2A 内部通信）
cargo run -- \
  --output-mode hybrid \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token <token> \
  --matrix-room-id '!room:matrix.local' \
  --agent-id agent-001 \
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

- **MSRV**: Rust 1.85（见 `Cargo.toml` 的 `rust-version`）
- **Docker 构建镜像**: 使用 `rust:1.85-alpine`
- 如果在其它分支 cherry-pick 本仓提交，请优先保留：
  1. `Cargo.toml` 中的 `rust-version = "1.85"`
  2. `deploy/agent/Dockerfile` 中的 `FROM rust:1.85-alpine`

### 特性开关

```bash
# 启用持久化存储（使用 sled）
cargo run --features persistent-store -- ...
```

## 配置说明

配置通过环境变量或命令行参数传入（使用 clap 解析）：

### 核心配置

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `OUTPUT_MODE` | 输出模式: `cli`, `matrix`, `a2a`, `hybrid` | cli |
| `OPENAI_API_KEY` | OpenAI API 密钥 | - |
| `OPENAI_MODEL` | 模型名称 | gpt-4o-mini |
| `CONTEXT_LIMIT` | 上下文消息数量 | 50 |
| `SYSTEM_PROMPT` | 系统提示词 | （见代码） |

### Matrix 配置（matrix/hybrid 模式下需要）

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `MATRIX_HOMESERVER` | Matrix 服务器地址 | - |
| `MATRIX_ACCESS_TOKEN` | Matrix 访问令牌 | - |
| `MATRIX_ROOM_ID` | 目标房间 ID | - |

### A2A 配置

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `AGENT_ID` | 当前 Agent 唯一标识 | agent-001 |
| `AGENT_NAME` | Agent 显示名称 | Virtual Agent |
| `A2A_TARGET_AGENT` | A2A 默认目标 Agent ID | - |
| `A2A_PEER_AGENTS` | 注册为 Peer 的 Agents（逗号分隔） | - |

### 存储配置

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `STORE_TYPE` | 存储类型: `memory`, `persistent` | memory |
| `STORE_PATH` | 持久化存储路径（persistent 类型使用） | ./data |
| `STORE_MAX_SIZE` | 存储消息数量上限 | 1000 |

### CLI 配置

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `INPUT_MESSAGE` | 单次执行的输入消息 | - |
| `INTERACTIVE` | 是否以交互模式运行 | false |
| `CLI_ECHO` | 是否在 CLI 模式下回显消息 | true |

## 输出模式详解

### CLI 模式

最简单的模式，无需 Matrix 服务器。适用于本地测试、脚本集成。

```bash
# 单次执行
cargo run -- --output-mode cli --openai-api-key <key> --input-message "Hello"

# 交互式模式
cargo run -- --output-mode cli --openai-api-key <key> --interactive
```

交互式命令：
- `/quit`, `/exit` - 退出
- `/help` - 显示帮助
- `/clear` - 清空上下文
- `/context` - 显示当前上下文

### Matrix 模式

Matrix 作为前端展示层，Agent 将消息发送到 Matrix 房间。

```bash
cargo run -- \
  --output-mode matrix \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token <token> \
  --matrix-room-id '!room:matrix.local' \
  --openai-api-key <key>
```

### A2A 模式

Agent 间通过 A2A 协议通信，实现多 Agent 协作。

```bash
# Agent 1
cargo run -- \
  --output-mode a2a \
  --agent-id agent-001 \
  --agent-name "Alice" \
  --a2a-peer-agents "agent-002" \
  --openai-api-key <key> \
  --input-message "Hello from Alice!"

# Agent 2
cargo run -- \
  --output-mode a2a \
  --agent-id agent-002 \
  --agent-name "Bob" \
  --a2a-peer-agents "agent-001" \
  --openai-api-key <key> \
  --input-message "Hello from Bob!"
```

### Hybrid 模式

Matrix 作为前端展示层，同时内部使用 A2A 协议进行 Agent 间通信。

```bash
cargo run -- \
  --output-mode hybrid \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token <token> \
  --matrix-room-id '!room:matrix.local' \
  --agent-id agent-001 \
  --a2a-peer-agents "agent-002,agent-003" \
  --openai-api-key <key>
```

## CI/CD 与发布

GitHub Actions 工作流 `.github/workflows/docker-publish.yml`：

### 触发条件
- `push` 到 `main` 或 `dev` 分支
- 推送 `v*` 标签
- PR 到 `main` 或 `dev` 分支
- 手动触发 (`workflow_dispatch`)

### Pipeline 流程

```
┌─────────┐    ┌─────────┐    ┌──────────────────┐    ┌───────────────┐
│  lint   │───→│  test   │───→│  build-and-push  │───→│ security-scan │
└─────────┘    └─────────┘    └──────────────────┘    └───────────────┘
  - fmt检查                     (多平台构建)              (Trivy扫描)
  - clippy检查                  (自动标签)
                               (构建信息注入)
```

### Jobs 说明

| Job | 说明 |
|-----|------|
| `lint` | 代码质量检查：`cargo fmt --check` + `cargo clippy` |
| `test` | 运行单元测试：`cargo test --release --all-features` |
| `build-and-push` | 多平台 Docker 镜像构建与推送 |
| `security-scan` | Trivy 容器安全扫描 |

### 镜像标签策略

| 场景 | 生成的标签 |
|------|-----------|
| `main` 分支 push | `latest`, `main-<short-sha>` |
| `dev` 分支 push | `dev`, `dev-<short-sha>` |
| Tag (e.g., `v1.2.3`) | `1.2.3`, `1.2` |
| PR | `pr-<number>` (仅构建，不推送) |

### 多平台支持
- `linux/amd64` (x86_64)
- `linux/arm64` (ARM64)

### 安全扫描
- 使用 Trivy 进行漏洞扫描
- 扫描结果上传到 GitHub Security tab
- `CRITICAL` 和 `HIGH` 级别漏洞会导致构建失败

## 服务器部署流程（重要）

### 🚫 严禁在服务器上本地构建

**绝对禁止**在服务器（107.173.156.228）上执行以下操作：
- 安装 Rust/Cargo 环境
- 运行 `cargo build` 或 `cargo run`
- 运行 `docker build` 构建镜像
- 任何其他形式的本地编译或构建操作

**原因**：
1. 服务器资源有限（1GB RAM），构建会耗尽内存
2. 构建过程可能引入安全风险
3. 无法保证构建环境的一致性
4. 违反"计算与状态分离"的设计原则

### ✅ 正确的部署流程

所有部署必须通过 **GitHub Actions CI/CD** 完成：

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ 本地开发    │────→│  push 到 dev    │────→│ GitHub Actions  │
│ 修改代码    │     │  或 main 分支   │     │ 自动构建镜像    │
└─────────────┘     └─────────────────┘     └─────────────────┘
                                                    │
                                                    ▼
┌─────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ 运行部署    │←────│  服务器执行     │←────│ 镜像推送到      │
│ 脚本测试    │     │  ./deploy/      │     │ GHCR 仓库       │
│             │     │  one_click_     │     │                 │
│             │     │  deploy.sh      │     │                 │
└─────────────┘     └─────────────────┘     └─────────────────┘
```

### 部署步骤

1. **本地修改代码**并测试
2. **Push 到 GitHub**：
   ```bash
   git add .
   git commit -m "feat: xxx"
   git push origin dev  # 或 main
   ```
3. **等待 GitHub Actions 完成**：
   - 访问 `https://github.com/zhengui666/ImitatorT/actions`
   - 确认 `build-and-push` job 成功
4. **在服务器上更新部署**：
   ```bash
   ssh zzy@107.173.156.228
   cd /home/zzy/ImitatorT
   git pull
   ./deploy/one_click_deploy.sh
   ```

### 狼人杀游戏部署

狼人杀游戏已打包在镜像中（路径 `/werewolf`），通过 docker-compose 启动：

```bash
# 启动狼人杀服务（使用已构建的镜像）
docker compose up -d werewolf

# 查看日志
docker logs -f werewolf
```

**注意**：狼人杀服务依赖 LLM 服务（litellm），请确保 `.env` 中的 `OPENAI_API_KEY` 和 `OPENAI_BASE_URL` 配置正确。

## 代码风格与约定

- 使用标准 Rust 格式化：`cargo fmt`
- 错误处理：统一使用 `anyhow::Result`
- 异步函数：使用 `async/await` + Tokio
- 日志记录：使用 `tracing` 宏（info, debug, error 等）
- 中文注释：项目文档和注释主要使用中文

## 模块职责

### `src/config.rs`
定义 `AppConfig` 结构体，使用 `clap::Parser` 派生宏支持从命令行参数和环境变量读取配置。

新增配置项：
- `OUTPUT_MODE`: 输出模式选择
- `AGENT_ID`, `AGENT_NAME`: A2A Agent 标识
- `A2A_PEER_AGENTS`: Peer Agents 列表
- `STORE_TYPE`, `STORE_PATH`: 存储配置
- `INTERACTIVE`, `CLI_ECHO`: CLI 模式配置

### `src/matrix.rs`
`MatrixClient` 封装 Matrix Client-Server API：
- `latest_context()`: 获取房间历史消息
- `send_text_message()`: 发送文本消息到房间

**注意**: Matrix 现在仅作为前端展示层，不再是状态中枢。

### `src/llm.rs`
`OpenAIClient` 封装 OpenAI Chat Completions API：
- `chat()`: 带工具调用的对话接口
- `complete()`: 简单完成接口（向后兼容）

### `src/tool.rs`
Tool/Function Calling 工具定义与执行：
- `ToolRegistry::get_tools()`: 获取所有可用工具定义
- `ToolRegistry::execute()`: 执行指定的工具调用
- 内置工具：`execute_command`（执行系统命令）、`fetch_url`（获取网页内容）

### `src/a2a.rs`
A2A (Agent-to-Agent) 协议简化实现：
- `AgentCard`: Agent 能力描述
- `A2AAgent`: Agent 运行时，支持消息发送/接收
- `A2AClient`: HTTP 客户端（用于远程通信）
- 支持消息类型：Text, Task, TaskResponse, Status

### `src/store.rs`
轻量级消息存储：
- 内存存储（默认）：`MessageStore::new(max_size)`
- 持久化存储（可选特性）：`MessageStore::new_persistent(path, max_size)`
- 自动消息数量限制（LRU 策略）

### `src/output.rs`
输出抽象层，统一不同输出模式的接口：
- `Output` trait: 统一输出接口
- `MatrixOutput`: Matrix 前端输出
- `CliOutput`: 命令行输出
- `A2AOutput`: A2A 协议输出
- `HybridOutput`: 混合输出
- `OutputFactory`: 输出工厂

### `src/main.rs`
主循环逻辑，支持多种运行模式：
1. 初始化 tracing 日志
2. 解析配置
3. 创建存储和输出处理器
4. 构建 LLM 客户端
5. 拉取上下文
6. 调用 LLM（带 tools 定义），检测是否需要 tool call
7. 如有 tool call，执行工具并将结果再次传给 LLM 获取最终回复
8. 回写结果到输出通道

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

多阶段多平台构建：
1. 使用 `rust:1.85-alpine` 交叉编译（支持 `linux/amd64` 和 `linux/arm64`）
2. 使用 `cargo-chef` 实现依赖层缓存
3. UPX 压缩二进制（仅 x86_64 平台）
4. 最终镜像基于 `scratch`（空镜像），仅包含 CA 证书和二进制文件
5. 注入 OCI 标准标签（版本、构建时间、Git SHA 等）

## 安全注意事项

- **敏感信息**: Matrix Token 和 OpenAI API Key 通过环境变量传入，不要硬编码
- **容器安全**: Agent 最终镜像使用 `scratch`，无 shell、无包管理器，最小攻击面
- **网络**: Conduwuit 默认关闭注册和联邦功能
- **证书**: 生产环境确保证书有效（Agent 镜像内置 CA 证书）
- **A2A 通信**: 当前实现为简化版本，生产环境建议添加身份验证和加密

## 扩展建议

- 添加更多 LLM 提供商支持（修改 `src/llm.rs`）
- 扩展 MCP 工具支持（修改 `src/tool.rs`）
- 完善 A2A HTTP 服务端实现（添加 HTTP 端点到 `src/a2a.rs`）
- 添加 WebSocket 支持用于实时 A2A 通信
- 添加健康检查端点
- 支持更多 Matrix 事件类型
- 添加消息持久化的自动清理策略
