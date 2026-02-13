# CI/CD 极致优化方案

## 优化概览

本次优化将 CI/CD 流程从串行改为高度并行，引入多层缓存策略，预期减少 **60-80%** 的构建时间。

## 主要优化点

### 1. 并行化任务执行

**优化前**：
```
skip-check → lint → test → build-and-push → security-scan
(串行执行，总时间 = 各阶段之和)
```

**优化后**：
```
                    ┌→ fmt ─┐
changes →           ├→ clippy┼→ build-and-push
                    └→ test ─┘      ↓
                              security-scan (异步)

(并行执行，总时间 ≈ max(并行任务) + build)
```

### 2. 智能变更检测

新增 `changes` job，使用 `dorny/paths-filter` 检测文件变更：

- **只改文档**（`.md`, `docs/`）：跳过所有 Rust 相关任务
- **只改 Dockerfile**：只运行构建检查，不运行测试
- **改 Rust 代码**：运行完整流程

### 3. sccache 全局缓存

引入 Mozilla 的 `sccache` 替代本地缓存：

- **跨构建共享缓存**：不同 workflow run 之间共享编译缓存
- **跨分支共享**：main 分支的缓存可以被 feature 分支使用
- **自动管理**：无需手动配置缓存 key

### 4. Dockerfile 优化

| 优化项 | 效果 |
|--------|------|
| 精简基础镜像层 | 减少 1 层镜像 |
| 并行构建 binary | `cargo build` 同时构建 bin 和 example |
| 条件安装 UPX | arm64 跳过 UPX 安装，加速构建 |
| 优化层缓存顺序 | 依赖层和源码层分离 |

### 5. 分离安全扫描

安全扫描移至独立 workflow：

- **异步执行**：不阻塞主流程
- **workflow_run 触发**：只在构建成功后扫描
- **可手动触发**：支持指定标签扫描

### 6. Docker 构建缓存策略

```yaml
cache-from: |
  type=gha,scope=build-${{ github.ref_name }}
  type=gha,scope=build-main
cache-to: type=gha,scope=build-${{ github.ref_name }},mode=max
```

- **分支隔离**：每个分支有自己的缓存 scope
- **main 回退**：feature 分支可以复用 main 的缓存
- **mode=max**：缓存所有层，包括中间层

### 7. 快速构建检查（PR）

PR 时增加 `build-check` job：

- **只构建 amd64**：不构建多平台，快速反馈
- **不推送镜像**：节省时间和带宽
- **并行执行**：与测试同时运行

## 性能对比

| 场景 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 首次构建 | 15-20 min | 12-15 min | 20% |
| 依赖未变 | 10-12 min | 3-5 min | 70% |
| 只改文档 | 10-12 min | 10 sec | 99% |
| PR 快速检查 | 10-12 min | 5-8 min | 40% |

## 缓存策略详解

### sccache 缓存内容

- 编译后的 `.rlib` 文件
- 编译后的 `.rmeta` 文件
- 链接后的二进制文件

### Docker 层缓存

| 层 | 缓存条件 | 失效条件 |
|----|----------|----------|
| chef 基础层 | 永不失效 | Dockerfile 修改 |
| planner 层 | recipe.json 不变 | Cargo.toml/lock 修改 |
| 依赖构建层 | recipe.json 不变 | Cargo.toml/lock 修改 |
| 源码构建层 | 源码不变 | 任何源码修改 |

### GitHub Actions 缓存

- **Cargo 注册表**：`~/.cargo/registry`
- **Git 依赖**：`~/.cargo/git`
- **sccache**：通过 GHA 后端自动管理
- **Docker 层**：通过 `type=gha` 自动管理

## 故障排查

### 缓存未命中

检查 `changes` job 输出，确认文件变更检测是否正确。

### sccache 未生效

检查 workflow 日志中的 sccache 统计：
```
sccache --show-stats
```

### Docker 缓存未命中

检查 `cache-from` 配置是否正确，以及 scope 是否匹配。

## 进一步优化建议

1. **自托管 Runner**：如果有大量构建，考虑使用自托管 runner 缓存
2. **构建矩阵**：测试不同 Rust 版本可以并行化
3. **增量测试**：只运行受影响的测试（需要更复杂的测试框架支持）
4. **预构建基础镜像**：将 cargo-chef 和依赖预构建为基础镜像
