# 测试指南

本目录包含项目的所有测试代码，按照分层架构组织。

## 目录结构

```
tests/
├── unit/           # 单元测试（按模块组织）
│   ├── core/
│   ├── infrastructure/
│   ├── protocol/
│   └── application/
├── integration/    # 集成测试
├── fixtures/       # 测试数据和模拟对象
└── common/         # 测试通用工具
```

## 运行测试

```bash
# 运行所有测试
cargo test

# 运行单元测试
cargo test --test unit

# 运行集成测试
cargo test --test integration

# 运行特定模块的测试
cargo test --test messaging_tests

# 运行带有特定名称的测试
cargo test test_private_message_creation

# 显示测试输出
cargo test -- --nocapture

# 运行测试并生成覆盖率报告
cargo tarpaulin --out Html
```

## 测试命名规范

- 单元测试：`test_<function_name>_<scenario>`
- 集成测试：`test_<feature>_<scenario>`

## 测试辅助工具

- `fixtures/` - 提供测试数据和模拟对象
- `common/` - 提供测试通用工具和初始化函数

## 编写新测试

1. 根据被测代码所在层级选择对应的测试目录
2. 使用 `fixtures` 中的数据生成函数
3. 使用 `common` 中的测试工具函数
4. 遵循 Arrange-Act-Assert 模式编写测试
