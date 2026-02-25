# Simple Company Example

这是一个简单的虚拟公司示例，展示如何使用 ImitatorT 框架。

## 运行

```bash
# 设置OpenAI API Key
export OPENAI_API_KEY=your-key-here

# 编辑 company_config.yaml，替换 api_key

# 运行
cargo run --bin simple_company
```

## 配置

编辑 `company_config.yaml` 来定制你的公司：

- `name`: 公司名称
- `organization.departments`: 部门列表
- `organization.agents`: Agent列表

每个Agent需要：
- `id`: 唯一标识
- `name`: 显示名称
- `role.system_prompt`: 系统提示词（决定Agent的行为）
- `llm_config`: LLM配置（模型、API key等）
