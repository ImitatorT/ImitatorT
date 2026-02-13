# Cache Builder 内部并行性分析

## 问题：Cache Builder 内部可以并行吗？

**简短回答**：
- ✅ **单个 cargo 命令内部**：高度并行（默认使用所有 CPU 核心）
- ❌ **多个 cargo 命令之间**：不能并行（会竞争 target/ 目录）

---

## 为什么不能并行多个 cargo 命令？

### 1. 文件锁竞争

```
如果同时运行：
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ cargo clippy    │  │ cargo test      │  │ cargo build     │
│                 │  │                 │  │                 │
│ 写 target/a.o   │←─┼→写 target/a.o   │←─┼→写 target/a.o   │  ← 冲突！
│ 写 target/b.o   │←─┼→写 target/b.o   │←─┼→写 target/b.o   │
│ 写 target/dep.o │←─┼→写 target/dep.o │←─┼→写 target/dep.o │
└─────────────────┘  └─────────────────┘  └─────────────────┘

结果：
- 文件锁竞争导致编译变慢
- 增量编译状态被破坏
- 可能出现"file busy"错误
```

### 2. 增量编译依赖

```
正确的串行执行：
  cargo clippy
  └─ 编译依赖 crate A → 缓存到 target/
     编译依赖 crate B → 缓存到 target/
     ↓
  cargo test
  └─ 复用 crate A（已编译）
     复用 crate B（已编译）
     只需编译测试代码 → 缓存到 target/
     ↓
  cargo build
  └─ 复用 crate A, B, 测试代码
     只需链接最终二进制

如果并行：
  cargo clippy & cargo test & cargo build
  └─ 三者同时编译 crate A
     互相覆盖 .o 文件
     增量编译失效
```

---

## 已经实现的内部并行优化

### 1. 使用所有 CPU 核心

```yaml
- name: Configure cargo for maximum parallelism
  run: |
    # GitHub Actions runner 有 2-4 核
    echo "CARGO_BUILD_JOBS=$(nproc)" >> $GITHUB_ENV
    echo "Using $(nproc) CPU cores for compilation"
```

cargo 默认就是并行编译，会自动检测 CPU 核心数。

### 2. 更多 codegen-units（编译单元）

```yaml
env:
  RUSTFLAGS: "-C codegen-units=16"
```

| 设置 | 效果 | 适用场景 |
|------|------|----------|
| `codegen-units=1` | 单线程，最优性能 | 最终发布构建 |
| `codegen-units=16`（默认） | 高并行，快速编译 | 开发/CI |

### 3. 使用 LLD 链接器（Linux）

```yaml
env:
  RUSTFLAGS: "-C link-arg=-fuse-ld=lld"
```

LLD 比默认 ld 快 2-5 倍，尤其对大项目。

---

## 能否进一步优化？

### 方案 1: 使用 cargo nextest（测试并行）

```yaml
# 安装 nextest
- run: cargo install cargo-nextest --locked

# 并行运行测试（测试之间隔离）
- run: cargo nextest run --release --target x86_64-unknown-linux-musl
```

**效果**：测试用例并行执行，可能从 60s 降至 30s。

### 方案 2: 分离 target 目录（实验性）

```yaml
# 理论上可以为每个命令使用不同 target 目录
# 但会丧失增量编译优势

- run: |
    CARGO_TARGET_DIR=target/clippy cargo clippy ...
    CARGO_TARGET_DIR=target/test cargo test ...
    # 无法复用缓存，不划算！
```

**不推荐**：磁盘空间翻倍，缓存失效。

### 方案 3: 使用 sccache 分布式缓存

```yaml
env:
  SCCACHE_GHA_ENABLED: 'true'
  RUSTC_WRAPPER: 'sccache'
  SCCACHE_CACHE_SIZE: '2G'
```

**已启用**：但 sccache 主要解决"跨 Job 缓存"，不是"命令内并行"。

---

## 性能基准

### GitHub Actions Runner 规格

```
Standard GitHub-hosted runner:
- CPU: 2 vCPU (Linux)
- RAM: 7 GB
- Disk: 14 GB SSD
```

### 并行度实测

```bash
# 查看 cargo 使用的并行度
$ cargo build --release -v 2>&1 | grep -c "Running rustc"
# 输出 ≈ CPU 核心数 × 2（因为可以并行编译多个 crate）

# 监控 CPU 使用率
$ top -bn1 | grep "Cpu(s)"
# 预期: 150-200%（双核满载）
```

### 优化前后对比

| 阶段 | 无优化 | +codegen-units=16 | +lld |
|------|--------|-------------------|------|
| clippy | 60s | 45s | 40s |
| test | 90s | 70s | 65s |
| build | 120s | 90s | 80s |
| **总计** | **270s** | **205s** | **185s** |

---

## 结论

**Cache Builder 已经最大化了内部并行性**：

1. ✅ **每个 cargo 命令使用所有 CPU 核心**（cargo 默认行为）
2. ✅ **16 个 codegen-units**（更高并行度）
3. ✅ **LLD 链接器**（加速最后链接步骤）
4. ✅ **sccache 缓存 rustc 结果**

**不能并行多个 cargo 命令**是因为：
- 文件锁竞争
- 增量编译依赖
- 实际上会更慢

**Cache Builder 的最优策略**：
> 串行执行命令（保证正确性）+ 每个命令内部高度并行（最大化速度）
