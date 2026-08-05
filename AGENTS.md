# NovelAgent — Rust AI / Vibe Coding 约束

> 本文件是 **AI 写/改本仓库 Rust 代码时的硬约束**。  
> 蒸馏自： [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) · [Apollo rust-best-practices](https://github.com/apollographql/rust-best-practices) · [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/) · [rust10x handbook](https://github.com/rust10x/rust10x) · [Canonical Rust best practices](https://canonical.github.io/rust-best-practices/) · 社区共识（thiserror/anyhow、clippy CI）。

**原则排序：** 正确性 → 意图清晰 → 可维护 → **有证据后** 再谈性能。  
**默认姿态：** 写最少、最直白、能通过门禁的代码；禁止为“架构完整”而堆抽象。

---

## 0. Vibe coding 铁律（先读）

| # | 铁律 | 违反时 |
|---|------|--------|
| V1 | **一刀一切片**：一次只做一个可验证行为 | 拆任务；禁止巨型 PR |
| V2 | **先红后绿**：行为变更先写/改测试再实现 | 禁止“先堆代码再补测” |
| V3 | **门禁未绿不算完成** | 见 §8 |
| V4 | **不擅自重构无关代码** | 禁止顺手清理用户实验、重命名大扫除 |
| V5 | **不发明依赖/API** | 新 crate 须说明用途；禁止假 import |
| V6 | **声明假设** | 对接口不明处写 `// ASSUMPTION:` 或先问用户 |
| V7 | **证据门禁** | 声称 “DONE / 已修 / 更快” 必须有命令输出或 path |

---

## 1. 项目形态（本仓库）

- 当前为 **binary 起步**（`src/main.rs`）；业务逻辑应尽快落到 **`lib` + 薄 `main`**。
- `main` / bin：**编排 + I/O**；`lib`：**可测纯逻辑**。
- 多 crate 时用 **workspace**，共享依赖写在 `[workspace.dependencies]`。
- 模块按 **领域/功能** 划分，不按“技术层万能文件夹”堆文件。
- 可见性默认 **私有**；`pub` 只暴露稳定边界。
- 公共 struct **字段默认私有**（API Guidelines C-STRUCT-PRIVATE）。

推荐演进（够用再加，禁止预建空架子）：

```text
src/
  main.rs          # 薄入口
  lib.rs           # 库根
  error.rs         # 类型化错误（库）或 re-export
  <domain>/        # 按领域模块，非“utils 垃圾桶”
tests/             # 集成测试（公共行为）
```

---

## 2. 所有权与 API 形状

- **默认借用** `&T` / `&mut T`；需要所有权再 `clone` / 传 owned。
- 公有 API 入参优先 `&str` / `&[T]` / `impl AsRef<Path>` / `impl Into<…>`，避免无故要 `String`/`Vec`。
- 小 `Copy` 按值传；大结构/堆数据按引用。
- **禁止热路径无脑 `.clone()`**；`clippy::redundant_clone` 当错误处理。
- 转换优先 `From` / `TryFrom` / `AsRef`，禁止手写无意义 bit 转换。
- 副作用留在 **边界**（I/O、网络、DB、进程）；中间变换保持纯。
- 指针阶梯：`&T` → `Box` → `Rc`/`Arc` → `Mutex`/`RwLock`；**禁止**为图方便上 `Arc<Mutex<…>>`。
- 静态分发默认（`impl Trait` / 泛型）；`dyn Trait` 仅插件/异构集合。
- Newtype 区分语义（`UserId` ≠ `u64`）；**Parse, don't validate**。
- 复杂构造用 Builder；简单配置用 `with_*` 链式即可，勿过度 Builder。
- 非法状态尽量 **类型不可表达**（type-state / enum）；简单状态用 enum + 运行时检查即可。

命名（API Guidelines）：

- `as_` 廉价借用 · `to_` 昂贵转换 · `into_` 消耗 self  
- Getter **不要** `get_` 前缀（布尔用 `is_`/`has_`）  
- Feature 名禁止 `use-foo` 等占位词；feature **必须 additive**

---

## 3. Option / Result / 控制流

- 传播用 `?`；分支用 `let … else` / `if let` / `match`。
- 昂贵默认值用 `*_or_else` / `*_else`，禁止 `unwrap_or(expensive())` 式抢跑分配。
- 纯变换优先 **iterator 链**；需要 `break`/`continue`/复杂副作用再用 `for`。
- **生产路径禁止** 新增 `unwrap` / `expect` / `panic!`（测试与“不可能失败且写明不变量”的 `expect("invariant: …")` 除外）。
- 禁止吞掉 `Err` 后假装成功。

---

## 4. 错误处理

| 层 | 推荐 | 禁止 |
|----|------|------|
| **库 / 模块边界** | 类型化错误：`thiserror`（或等价 enum + `Display` + `Error`） | `Result<T, String>` 作为长期公开 API |
| **二进制 / 应用顶层** | `anyhow` + `.context()` / `.with_context()` | 到处 `unwrap` |
| **跨 crate** | 边界 `From`/`map_err` 转换，保留 `source` 链 | 丢弃根因 |

- 错误 `Display`：**小写、无句号**（可组合进错误链）。
- 异步错误跨 `.await` 需 `Send + Sync + 'static`（按运行时要求）。
- 行为测试须覆盖 **成功 + 失败** 路径。

---

## 5. Unsafe / 并发 / 异步

- `unsafe` 仅：FFI、实测性能、底层抽象；**每块必须有 `// SAFETY:`**；`unsafe fn` 必须有 `# Safety` 文档。
- 禁止为绕过借用检查而 `unsafe`。
- 含 `unsafe` 时优先考虑 `miri`（有条件再进 CI）。
- **禁止** 在 `.await` 间持有 `std::sync::Mutex` guard；跨 await 用 `tokio::sync` 或先 drop。
- 阻塞 I/O / CPU 重活：`spawn_blocking` / `rayon`；勿堵 async runtime。
- 通道默认 **有界**（背压）；无界通道需书面理由。
- CPU 并行 → `rayon`；I/O 并发 → async runtime。共享可变默认消息传递优于锁。

---

## 6. 数值 / 集合 / 性能

- 收窄转换：`TryFrom`，禁止 `as` 静默截断。
- 溢出场景显式：`checked_*` / `saturating_*` / `wrapping_*`。
- 浮点不用 `==`；集合默认 `Vec`，按访问模式选 `HashMap`/`BTreeMap`/`IndexMap`。
- 已知容量 → `with_capacity`；热循环复用 buffer。
- **无 profile / bench 证据禁止“优化重构”**（Apollo / Microsoft 共识）。
- 性能主张必须附：`cargo bench` / flamegraph / 前后对比命令。

---

## 7. 文档、注释、TODO

- `//` 只写 **why / 不变量 / 安全**；禁止复述代码的废话注释。
- 公开项 `///`；模块用途 `//!`。
- 用户明确要求改代码时：**不要**堆说明性注释。
- `TODO` 须可追踪：`// TODO(#issue): …`；禁止裸 `TODO fix later`。
- 禁止无理由 `#[allow(...)]`；局部抑制用 `#[expect(…, reason = "…")]`。

---

## 8. 质量门禁与工具链（DONE 定义）

**DONE = 机器门禁绿 + 行为证据。** 文风靠 `AGENTS.md`；对 AI 最有效的是 **可失败的命令**。

### 8.1 一键入口（优先）

```powershell
# 仓库根。L0 = 必过；L1 = 供应链；L2 = 深度质量（慢）
pwsh -File scripts/ai-gate.ps1              # L0
pwsh -File scripts/ai-gate.ps1 -Level L1
pwsh -File scripts/ai-gate.ps1 -Level all
# 可选工具安装: pwsh -File scripts/install-ai-tools.ps1
```

等价手写 L0：

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# 若已装 nextest: cargo nextest run --workspace --all-features
```

| 检查 | 要求 |
|------|------|
| fmt | 通过（`rustfmt.toml`） |
| clippy | 通过；`Cargo.toml` `[lints.clippy]` + `-D warnings` |
| test | 相关行为绿 |
| 文档 | 公开 API 与行为一致 |
| 性能 | 无 profile/bench **禁止**声称更快 |

### 8.2 工具分层（约束 AI 的职责）

| 层 | 工具 | 约束 AI 什么 | 何时 |
|----|------|--------------|------|
| **L0 必过** | `rustfmt` | 禁止格式战争 / 乱缩进 | 每次 diff |
| | `clippy`（`-D warnings` + workspace lints） | 反模式、多余 clone、烂写法 | 每次 diff |
| | `cargo test` / **`cargo-nextest`** | 行为正确；nextest 更快更稳 | 每次 diff |
| **L1 供应链** | **`cargo-deny`**（`deny.toml`） | 许可证、漏洞公告、未知源、git 依赖、多版本 | 加/改依赖后必跑；日常 L1 |
| | `cargo-audit` | RustSec 漏洞（与 deny advisories 重叠，作补充） | L1 |
| | **`cargo-machete`** | 未使用依赖（AI 乱加 crate） | L1；优先于 udeps（stable） |
| | `cargo-udeps` | 更准的未用依赖（**需 nightly**） | 可选深扫 |
| **L2 深度** | **`cargo-llvm-cov`** | 覆盖率；防“假测” | 大行为变更 / 合并前 |
| | **`cargo-mutants`** | 变异测试：测是否真能抓 bug | 关键逻辑；ThoughtWorks 雷达推荐 |
| | **`cargo-hack`** | feature 组合可编译（防 AI 拆坏 feature） | 有 features 后 |
| | **`cargo-semver-checks`** | 公开 API  semver 破坏 | **库**对外发布时 |
| | `miri` | UB（有 `unsafe` 时） | 引入 unsafe 后 |
| | `cargo-criterion` / flamegraph | 性能证据 | 仅当用户要优化 |

### 8.3 仓库内已落地配置

| 文件 | 作用 |
|------|------|
| `Cargo.toml` `[lints.*]` | rustc/clippy 默认 deny/warn（AI 无法靠“忘了传 -D”绕过） |
| `rustfmt.toml` | 格式基线 |
| `deny.toml` | 源/许可证/公告策略；**未知 git registry → deny** |
| `scripts/ai-gate.ps1` | L0/L1/L2 统一入口；缺可选工具 → `SKIP` 并提示安装 |
| `scripts/install-ai-tools.ps1` | 安装 L1/L2 CLI |

### 8.4 AI 使用工具的硬规则

1. **每次行为变更**：至少跑 **L0**；报告命令与 exit code。
2. **改动 `Cargo.toml` / 新依赖**：必须 **L1**（至少 `cargo deny check` + machete 若已装）。
3. **禁止** 为过门禁而 `#[allow(clippy::all)]`、清空 `deny.toml` allow 列表、或把 `wildcards`/`unknown-git` 改成 allow 却不写 reason。
4. 可选工具未安装 → 在报告写 `SKIP: cargo-deny (not installed)`，**不得假装已审计**。
5. 新依赖：优先 crates.io；git 依赖须先改 `deny.toml` `[sources].allow-git` 并说明原因。
6. 覆盖率 / mutants **不替代** 行为测试；L2 失败要修测试或实现，禁止关阈值。

### 8.5 推荐安装（本机一次）

```powershell
# 快速路径（有 binstall 时）
cargo binstall -y cargo-deny cargo-machete cargo-nextest cargo-audit cargo-llvm-cov cargo-mutants cargo-hack

# 或
pwsh -File scripts/install-ai-tools.ps1
```

rustup 已含：`rustfmt` · `clippy`（本机已验证）。

---

## 9. 测试约束

- **一个测试一个行为**；名称描述行为（非 `test1`）。
- 单元测内部；集成测 `tests/` 只测公共契约。
- 文档示例用 `?`，不用 `unwrap`（API Guidelines C-QUESTION-MARK）。
- Snapshot（如 `insta`）仅用于 **稳定、复杂结构化输出**。
- 禁止为绿测而改业务语义去迁就错误实现。

---

## 10. AI 输出纪律（防 vibe 翻车）

**必须：**

1. 改前读相关文件；改后跑 §8。
2. Diff **外科手术**：只动任务所需行。
3. 新模块有明确边界与错误类型策略。
4. 依赖：优先 std；新增 crate 写一行理由。
5. 秘密不进仓库（`.env` 已存在则永不打印 key）。

**禁止：**

| 反模式 | 说明 |
|--------|------|
| God `utils.rs` / `helpers.rs` | 按领域拆 |
| 过早 workspace 五六 crate | 单 crate 直到编译/边界真疼 |
| 全库 `anyhow::Error` 当公开类型 | 库用 thiserror |
| 全局可变 + `lazy_static` 乱飞 | 注入依赖 / 参数传递 |
| 为 AI 方便 `clone` 一切 | 借用优先 |
| 假代码 / 未用 import / 编造 API | 编译不过 = 未完成 |
| 大段重写“更优雅” | 无请求不做 |
| 静默 `#[allow(clippy::all)]` | 禁止 |

---

## 11. 任务开工模板（AI 自检）

```text
GOAL: <一个可观察行为>
NON-GOALS: <明确不做>
TOUCH: <预期文件列表，越短越好>
TESTS: <新增/修改的测试名>
GATES: L0 (scripts/ai-gate.ps1) [+ L1 if Cargo.toml deps]
ASSUMPTIONS: <若有>
```

完工报告：

```text
DONE: <一句话>
EVIDENCE: <ai-gate / cargo 命令与结果摘要>
DIFF: <主要文件>
TOOLS: L0=pass | L1=pass|skip|fail
FOLLOW-UP: <可选债务，写成 TODO(#) 或明确不跟>
```

---

## 12. 参考索引（深挖时再开）

| 主题 | 源 |
|------|----|
| 官方 API 清单 | https://rust-lang.github.io/api-guidelines/checklist.html |
| Apollo 手册 + AGENTS 范本 | https://github.com/apollographql/rust-best-practices |
| Microsoft 务实指南（含 LLM 向） | https://microsoft.github.io/rust-guidelines/ |
| rust10x 生产范式（AI pack） | https://github.com/rust10x/rust10x |
| Canonical 风格/一致性 | https://canonical.github.io/rust-best-practices/ |
| cargo-deny | https://embarkstudios.github.io/cargo-deny/ |
| cargo-nextest | https://nexte.st/ |
| cargo-semver-checks | https://github.com/obi1kenobi/cargo-semver-checks |
| cargo-mutants | https://mutants.rs/ |
| cargo-machete | https://github.com/bnjbvr/cargo-machete |
| Clippy 配置 | `Cargo.toml` `[lints]` + `cargo clippy` |
| 错误：lib thiserror / bin anyhow | GreptimeDB 等大规模实践同方向 |

---

*维护：规则与官方/社区冲突时，以 **可编译 + 本仓库既有风格 + API Guidelines** 为准；新增硬规则必须落到 **clippy / test / deny / ai-gate** 之一。*
