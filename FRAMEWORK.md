# 虚拟公司框架 API 文档

## 快速开始

```rust
use imitatort_stateless_company::{VirtualCompany, AppBuilder, AgentConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 创建虚拟公司实例
    let company = AppBuilder::new()
        .with_endpoint("http://localhost:8080")
        .with_server("0.0.0.0:8080".parse()?)
        .build().await?;

    // 2. 创建 Agent
    let agent_config = AgentConfig {
        id: "agent-001".to_string(),
        name: "My Agent".to_string(),
        system_prompt: "You are a helpful assistant.".to_string(),
        model: "gpt-4o-mini".to_string(),
        api_key: "sk-xxx".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        metadata: Default::default(),
    };
    
    let agent = company.create_agent(agent_config).await?;

    // 3. 发送消息
    company.broadcast("agent-001", "Hello everyone!").await?;

    Ok(())
}
```

## 核心概念

### VirtualCompany

框架的核心结构，封装了所有能力：

```rust
pub struct VirtualCompany {
    // Agent 管理
    pub async fn create_agent(&self, config: AgentConfig) -> Result<Arc<Agent>>;
    pub fn get_agent(&self, id: &str) -> Option<Arc<Agent>>;
    pub fn list_agents(&self) -> Vec<Arc<Agent>>;
    
    // 消息通信
    pub async fn send_private(&self, from: &str, to: &str, content: &str) -> Result<()>;
    pub async fn send_group(&self, from: &str, group_id: &str, content: &str) -> Result<()>;
    pub async fn broadcast(&self, from: &str, content: &str) -> Result<()>;
    
    // 群聊管理
    pub async fn create_group(&self, id: &str, name: &str, creator: &str, members: Vec<String>) -> Result<String>;
    pub async fn invite_to_group(&self, group_id: &str, inviter: &str, invitee: &str) -> Result<()>;
    
    // 网络
    pub async fn connect_to_network(&self, seed_endpoints: &[String]) -> Result<()>;
    pub async fn announce_presence(&self, node_info: &AgentInfo) -> Result<()>;
}
```

### Agent

Agent 是虚拟公司的基本单元：

```rust
pub struct Agent {
    // 获取信息
    pub fn id(&self) -> &str;
    pub fn name(&self) -> &str;
    pub fn system_prompt(&self) -> &str;
    pub fn metadata(&self) -> &serde_json::Map<String, serde_json::Value>;
    
    // 运行任务
    pub async fn run(&self, task: &str) -> Result<String>;
    
    // 工具调用
    pub async fn execute_tool(&self, tool_name: &str, arguments: &str) -> Result<String>;
    pub fn available_tools(&self) -> Vec<&Tool>;
}
```

## 消息通信

### 私聊

```rust
// Agent A 发送私聊给 Agent B
company.send_private("agent-a", "agent-b", "Hello!").await?;
```

### 群聊

```rust
// 1. 创建群聊
company.create_group(
    "project-alpha",
    "Project Alpha Team",
    "alice",
    vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()]
).await?;

// 2. 发送群聊消息
company.send_group("alice", "project-alpha", "Meeting at 3pm").await?;

// 3. 邀请新成员
company.invite_to_group("project-alpha", "alice", "david").await?;
```

### 广播

```rust
// 发送给所有 Agent
company.broadcast("host", "Game started!").await?;
```

## 分布式部署

### 多节点架构

```
Node A (Alice)          Node B (Bob)
┌─────────────┐         ┌─────────────┐
│  Agent: A   │◄───────►│  Agent: B   │
│  Agent: C   │  HTTP   │  Agent: D   │
└─────────────┘         └─────────────┘
```

### 启动节点

```rust
// 节点 A（种子节点）
let company_a = AppBuilder::new()
    .with_endpoint("http://node-a:8080")
    .with_server("0.0.0.0:8080".parse()?)
    .build().await?;

// 节点 B（连接到 A）
let company_b = AppBuilder::new()
    .with_endpoint("http://node-b:8081")
    .with_server("0.0.0.0:8081".parse()?);
// 注册远程 Agent
company_b.register_remote_agent("seed-agent", "http://node-a:8080");
    .build().await?;
```

### 跨节点通信

框架自动处理跨节点消息路由：

```rust
// 在节点 A 上
company_a.send_private("agent-a", "agent-b", "Hello from A").await?;
// 即使 agent-b 在节点 B 上，消息也能正确送达
```

## 完整示例：狼人杀

见 `examples/werewolf/` 目录，展示了：

1. 如何定义角色（系统提示词）
2. 如何创建多个 Agent
3. 如何实现游戏逻辑
4. 如何支持分布式部署

运行：

```bash
# 单机运行
cargo run --example werewolf

# 分布式运行
# 终端 1
cargo run --example werewolf -- --bind 0.0.0.0:8080

# 终端 2
cargo run --example werewolf -- --bind 0.0.0.0:8081 --seed http://localhost:8080
```

## 架构设计

### 分层架构

```
┌─────────────────────────────────────┐
│           应用层 (Your App)          │
│    - 业务逻辑、游戏规则、工作流       │
├─────────────────────────────────────┤
│           框架 API 层                │
│    VirtualCompany / AppBuilder      │
├─────────────────────────────────────┤
│           核心能力层                 │
│  Agent │ Messaging │ Router │ A2A   │
├─────────────────────────────────────┤
│           基础设施层                 │
│  swarms-rs │ HTTP │ Tools │ Storage │
└─────────────────────────────────────┘
```

### 设计原则

1. **框架与应用解耦**：框架提供通用能力，业务逻辑由应用实现
2. **Agent 自主决策**：Agent 自主决定何时发送消息、创建群聊
3. **分布式原生**：支持多节点部署，消息自动路由
4. **零配置启动**：简单的 API，快速搭建虚拟公司环境
