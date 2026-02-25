# ImitatorT - 多Agent公司模拟框架

基于 Rust 的轻量级框架，让多个 AI Agent 像真人一样在公司中协作。

## 核心理念

- **自主Agent**: Agent 可以自主决定创建群聊、发起私聊、执行任务
- **组织架构**: 支持部门和层级关系，Agent 知道该向谁汇报
- **消息驱动**: 通过消息通信实现协作，模拟真人公司的沟通方式

## 快速开始

```rust
use imitatort_stateless_company::{CompanyBuilder, CompanyConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从配置文件创建公司
    let config = CompanyConfig::from_file("company_config.yaml")?;
    let company = CompanyBuilder::from_config(config).build();

    // 启动运行
    company.run().await?;

    Ok(())
}
```

## 配置示例

```yaml
name: "AI研究公司"
organization:
  departments:
    - id: "research"
      name: "研究部"
    - id: "writing"
      name: "撰写部"
      parent_id: "research"

  agents:
    - id: "ceo"
      name: "CEO"
      role:
        title: "首席执行官"
        system_prompt: "你是CEO，负责公司决策..."
      llm_config:
        model: "gpt-4o-mini"
        api_key: "your-key"
```

## 项目结构

```
ImitatorT/
├── src/              # Rust 后端框架
│   ├── domain/      # 领域层：Agent、消息、组织架构
│   ├── core/        # 核心层：Agent运行时、消息总线
│   ├── application/ # 应用层：VirtualCompany API
│   └── infrastructure/ # 基础设施：LLM客户端
├── frontend/        # React + TypeScript 前端（核心GUI）
│   ├── src/        # 前端源码
│   └── package.json
├── examples/
│   └── simple_company/  # 示例项目
└── Cargo.toml
```

## 启动前端

```bash
cd frontend
npm install
npm run dev
```

## 依赖

- Rust 1.85+
- OpenAI API Key

## License

Apache License 2.0
