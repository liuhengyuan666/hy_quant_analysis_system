# Structure Memory

## 当前顶层结构

```text
rust-quant-analysis-system/
├── apps/          # CLI + desktop shell
├── crates/        # domain, engines, storage, orchestration
├── config/        # universe 等配置
├── data/          # 本地数据目录
├── docs/          # 使用与架构文档
├── infra/         # Docker / ClickHouse 启动资源
├── reports/       # 导出报告（运行产物）
├── runtime/       # agent 运行规范
├── sql/           # 初始化 DDL
├── target/        # Rust 构建产物
└── memory/        # 项目记忆系统
```

## 已确认的前后端边界线索

- 前端主要位于 `apps/desktop/frontend`。
- 桌面桥接 / Tauri Rust 侧位于 `apps/desktop/src-tauri`。
- 后端计算与存储核心主要位于 `crates/*`。
- CLI 位于 `apps/cli`，本质上是后端能力的入口之一。

## 当前结构评估结论（2026-04-24）

### 已经天然分层的部分

- `apps/desktop/frontend` 是明确的前端资源树。
- `crates/*` + `apps/cli` 是明确的 Rust 后端能力树。
- `apps/desktop/src-tauri` 是桌面壳层，但它同时依赖前端产物与 Rust 后端服务，因此是典型桥接层，而不是纯前端或纯后端。

### 若做更显式目录分层，最自然的归组

- 候选 front 组：
  - `apps/desktop/frontend`
  - `apps/desktop/src-tauri`（从产品视角属于桌面前台，但技术上是桥接层）
- 候选 backend 组：
  - `apps/cli`
  - `crates/*`

### 目前不适合直接上来大搬家的共享根目录内容

- `Cargo.toml`（workspace 根）
- `config/`
- `data/`
- `sql/`
- `infra/`
- `docs/`
- `reports/`

### 已确认的重组阻力

- `market-store::StorageConfig::project_root()` 通过 `Cargo.toml` + `crates/` 查找项目根目录。
- `market-store` 默认依赖根路径下的 `data/app_state.db`、`config/universe.json`、`sql/...`。
- `app-service` 会直接读取根目录下 `docs/*.md` 并写入 `reports/`。
- `apps/desktop/src-tauri/tauri.conf.json` 当前使用 `../frontend/dist` 相对路径。

## 待进一步评估的问题

- 是否需要把现有 `apps/desktop/frontend` 上提为更显式的 `front/`，或保持 `apps/desktop/*` 但增强内部边界。
- `apps/desktop/src-tauri` 与 `crates/*`、`apps/cli` 在目录层次上如何组织，才能既清晰又不破坏 Rust workspace / Tauri 约定。
- 若未来要做显式 front/backend 分层，更适合采用渐进式方案，而不是一步到位的顶层重命名迁移。
