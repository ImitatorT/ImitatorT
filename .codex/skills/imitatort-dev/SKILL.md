---
name: imitatort-dev
description: ImitatorT 项目的开发规范和标准操作流程（SOP）
---

# ImitatorT 项目开发 SOP

## 项目概述

ImitatorT 是一个基于 Rust 的虚拟公司框架，采用分层架构：

```
应用层 (examples/)
    ↑ 使用
框架 API 层 (src/framework.rs)
    ↑ 调用
核心能力层 (src/agent.rs, messaging.rs, router.rs, ...)
    ↑ 依赖
基础设施层 (swarms-rs, axum, ...)
```

## 开发流程

### 1. 开发前准备

- 理解需求，明确修改在哪一层：
  - **框架层** (`src/`)：通用能力，所有应用共享
  - **应用层** (`examples/`)：特定业务逻辑
- 保持接口向后兼容

### 2. 编码规范

#### 2.1 代码风格
- 使用标准 Rust 格式化：`cargo fmt`
- 错误处理：统一使用 `anyhow::Result`
- 异步函数：使用 `async/await` + Tokio
- 日志记录：使用 `tracing` 宏
- 中文注释：项目文档和注释主要使用中文

#### 2.2 模块组织
```
src/
├── lib.rs           # 库入口，对外导出 API
├── main.rs          # 二进制入口（CLI）
├── framework.rs     # 框架 API（VirtualCompany, AppBuilder）
├── agent.rs         # Agent 实现
├── messaging.rs     # 消息通信层
├── router.rs        # 消息路由器
├── a2a_server.rs    # A2A HTTP 服务端
├── a2a_client.rs    # A2A HTTP 客户端
├── ...

examples/
└── werewolf/        # 狼人杀示例应用
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── game.rs
        └── roles.rs
```

#### 2.3 设计原则
1. **框架与应用解耦**：框架提供通用能力，业务逻辑由应用实现
2. **Agent 自主决策**：Agent 自主决定何时发送消息、创建群聊
3. **简单 API**：通过 `VirtualCompany` 和 `AppBuilder` 提供简洁接口
4. **分布式原生**：支持多节点部署，消息自动路由

### 3. 单元测试要求

#### 3.1 测试覆盖
- 每个公共函数至少有一个测试用例
- 边界条件必须测试
- 错误路径必须测试

#### 3.2 测试命名规范
- `test_{功能}_{场景}`

### 4. 提交前检查清单

- [ ] 代码格式化：`cargo fmt`
- [ ] 代码检查：`cargo clippy`（如果有环境）
- [ ] 单元测试通过：`cargo test`（如果有环境）
- [ ] 文档更新（如需要）

### 5. Git 工作流

#### 5.1 分支策略
- `main`：稳定分支
- `dev`：开发分支

#### 5.2 提交规范
```bash
# 开发完成后，先本地测试
cargo fmt

# 提交到 dev 分支
git add .
git commit -m "feat: 添加消息路由功能

- 实现本地和远程消息路由
- 添加跨 Agent 群聊支持
- 完善单元测试"

git push origin dev
```

#### 5.3 提交信息格式
```
<type>: <subject>

<body>
```

类型（type）：
- `feat`：新功能
- `fix`：修复
- `docs`：文档
- `refactor`：重构
- `test`：测试

### 6. 框架开发指南

#### 6.1 添加新功能到框架

1. 在 `src/` 下创建新模块
2. 在 `src/lib.rs` 中导出公共类型
3. 在 `src/framework.rs` 中封装简洁 API
4. 添加单元测试
5. 更新 `FRAMEWORK.md` 文档

#### 6.2 创建新示例应用

1. 在 `examples/` 下创建新目录
2. 创建 `Cargo.toml`，依赖 `imitatort_stateless_company`
3. 实现应用逻辑
4. 使用 `AppBuilder` 快速搭建环境

示例：
```rust
// examples/my_app/main.rs
use imitatort_stateless_company::{AppBuilder, AgentConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let company = AppBuilder::new("http://localhost:8080")
        .bind("0.0.0.0:8080".parse()?)
        .build().await?;
    
    // 创建 Agent、发送消息...
    
    Ok(())
}
```

### 7. 调试技巧

#### 7.1 本地调试
```bash
# 运行示例
cargo run --example werewolf

# 运行测试
cargo test
```

#### 7.2 服务器调试
```bash
# 查看日志
docker logs swarms-agent

# 进入容器
docker exec -it swarms-agent /bin/sh
```

### 8. 发布流程

1. 确保所有测试通过
2. 更新版本号（`Cargo.toml`）
3. 更新 `CHANGELOG.md`
4. 合并 `dev` 到 `main`
5. 打标签：`git tag v0.x.x`
6. 推送标签：`git push origin v0.x.x`
7. GitHub Actions 自动构建并发布到 GHCR

## 参考文档

- [FRAMEWORK.md](../FRAMEWORK.md)：框架 API 文档
- [AGENTS.md](../AGENTS.md)：项目架构文档
- [README.md](../README.md)：项目说明
