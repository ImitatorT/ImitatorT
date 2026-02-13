# 快速开始

## 最简单的 CLI 模式

无需 Matrix 服务器，直接运行：

```bash
# 设置环境变量
export OPENAI_API_KEY="your-api-key"

# 单次执行
cargo run -- --output-mode cli --input-message "Hello, who are you?"

# 交互式模式
cargo run -- --output-mode cli --interactive
```

## Matrix 前端模式

需要启动 Conduwuit Matrix 服务器：

```bash
# 启动 Conduwuit
docker compose up -d conduwuit

# 获取访问令牌并运行 Agent
cargo run -- \
  --output-mode matrix \
  --matrix-homeserver http://localhost:6167 \
  --matrix-access-token YOUR_TOKEN \
  --matrix-room-id '!YOUR_ROOM:matrix.local' \
  --openai-api-key YOUR_API_KEY
```

## A2A 多 Agent 模式

启动多个 Agent 进行协作：

```bash
# 终端 1: 启动 Agent Alice
cargo run -- \
  --output-mode a2a \
  --agent-id alice \
  --agent-name "Alice" \
  --a2a-peer-agents "bob" \
  --openai-api-key $OPENAI_API_KEY \
  --interactive

# 终端 2: 启动 Agent Bob
cargo run -- \
  --output-mode a2a \
  --agent-id bob \
  --agent-name "Bob" \
  --a2a-peer-agents "alice" \
  --openai-api-key $OPENAI_API_KEY \
  --interactive
```

## 持久化存储

启用 sled 数据库存储（需要 `persistent-store` 特性）：

```bash
cargo run --features persistent-store -- \
  --store-type persistent \
  --store-path ./data \
  --output-mode cli \
  --openai-api-key $OPENAI_API_KEY \
  --interactive
```

## 环境变量配置

创建 `.env` 文件：

```bash
OUTPUT_MODE=cli
OPENAI_API_KEY=your-api-key
OPENAI_MODEL=gpt-4o-mini
CONTEXT_LIMIT=50
STORE_TYPE=memory
CLI_ECHO=true
```

然后直接运行：

```bash
cargo run -- --interactive
```

## 交互式命令

在交互式模式下可用的命令：

- `/quit` 或 `/exit` - 退出程序
- `/help` - 显示帮助信息
- `/clear` - 清空上下文历史
- `/context` - 显示当前上下文内容
