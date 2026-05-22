# Structure Memory

> 长期：结构记忆（目录结构、模块划分、依赖关系）。  
> 最后更新：2026-05-20（同步 Trading-Aware Partial Coverage、前端渐进拆分、默认 export-report fail-loud 等落地状态）。

---

## 当前顶层结构

```text
rust-quant-analysis-system/
├── apps/                # CLI + desktop shell
│   ├── cli/             # quant-cli：薄 clap 命令行封装
│   └── desktop/         # quant-desktop：Tauri + 前端
│       ├── frontend/    # Vite + Plain JS 前端
│       │   ├── src/
│       │   │   ├── main.js
│       │   │   ├── styles.css
│       │   │   ├── lib/
│       │   │   │   └── dashboard-utils.js
│       │   │   ├── features/
│       │   │   │   ├── data-health.js
│       │   │   │   ├── recent-reports.js
│       │   │   │   └── usage-guides.js
│       │   │   └── renderers/
│       │   │       └── environment-breadth.js
│       │   ├── dist/    # Vite 构建产物
│       │   └── node_modules/
│       └── src-tauri/   # Tauri Rust 桥接层
│           ├── src/
│           │   ├── main.rs
│           │   └── lib.rs
│           ├── Cargo.toml
│           └── tauri.conf.json
│
├── crates/              # Rust Workspace 核心实现层
│   ├── core-domain/     # 共享契约（DTO / Enum / TradingCalendar）
│   ├── data-ingestion/  # 数据拉取与标准化
│   ├── indicator-engine/# 技术指标计算
│   ├── macro-engine/    # 宏观因子与市场状态 (Regime)
│   ├── rotation-engine/ # 相对强弱与轮动排名
│   ├── strategy-engine/ # 四类策略偏好评分
│   ├── signal-engine/   # 最终信号生成（含 data-starved 统计）
│   ├── backtest-engine/ # 信号驱动回测
│   ├── report-engine/   # Dashboard / Report 载荷塑造
│   ├── market-store/    # ClickHouse + SQLite 持久化
│   ├── app-service/     # 编排门面（最高耦合层）
│   └── task-runner/     # 占位工具 crate
│
├── config/              # 运行时配置
│   ├── universe.json    # 标的池
│   └── calendars/       # 静态交易日历（CN / HK）
│
├── docs/                # 使用手册、架构文档、设计文档
├── infra/               # Docker / ClickHouse 启动资源
│   └── docker/
│       └── docker-compose.yml
│
├── sql/                 # 初始化 DDL
│   ├── clickhouse/001_init.sql
│   └── sqlite/001_init.sql
│
├── data/                # 本地轻状态（SQLite 等）
├── reports/             # 导出报告（运行时产物）
├── memory/              # 项目记忆系统
│   ├── product.md
│   ├── tech.md
│   ├── structure.md     # 本文件
│   ├── glossary.md
│   ├── decisions.md
│   ├── context.md
│   └── history/
│
├── runtime/             # Agent 运行规范
├── target/              # Rust 构建产物
├── Cargo.toml           # Workspace 根
├── PROJECT_STRUCTURE.md # 项目结构说明书（结构化全貌）
└── README.md            # 当前事实来源主入口
```

---

## Workspace 成员清单

| # | Member | 类型 | 说明 |
|---|--------|------|------|
| 1 | `apps/cli` | App | 命令行入口 `quant-cli` |
| 2 | `apps/desktop/src-tauri` | App | 桌面端 Tauri 桥接 `quant-desktop` |
| 3 | `crates/core-domain` | Crate | 共享契约 |
| 4 | `crates/data-ingestion` | Crate | 数据拉取 |
| 5 | `crates/indicator-engine` | Crate | 技术指标 |
| 6 | `crates/macro-engine` | Crate | 宏观 / Regime |
| 7 | `crates/rotation-engine` | Crate | 轮动排名 |
| 8 | `crates/strategy-engine` | Crate | 策略偏好 |
| 9 | `crates/signal-engine` | Crate | 信号生成 |
| 10 | `crates/backtest-engine` | Crate | 回测引擎 |
| 11 | `crates/report-engine` | Crate | 报告塑造 |
| 12 | `crates/market-store` | Crate | 持久化边界 |
| 13 | `crates/app-service` | Crate | 编排门面 |
| 14 | `crates/task-runner` | Crate | 占位工具 |

---

## 前后端边界线索

- **前端**：`apps/desktop/frontend/src/`，Plain JS + Vite，已按 `lib/` → `features/` → `renderers/` 渐进拆分。
- **桌面桥接**：`apps/desktop/src-tauri/`，Tauri 命令注册与刷新协调，保持薄封装。
- **后端计算与存储**：`crates/*` + `apps/cli`，Rust 后端能力树。
- **配置共享**：`config/`, `sql/`, `data/`, `reports/` 位于根目录，被 `market-store` 与 `app-service` 直接引用。

---

## 当前结构评估结论

### 已经天然分层的部分

- `apps/desktop/frontend` 是明确的前端资源树。
- `crates/*` + `apps/cli` 是明确的后端能力树。
- `apps/desktop/src-tauri` 是典型桥接层，依赖前端产物与 Rust 后端服务。

### 热点文件（已知需要拆分）

| 文件 | 规模 | 问题 |
|------|------|------|
| `crates/app-service/src/lib.rs` | ~795 行 | monolith，编排逻辑过度集中 |
| `crates/market-store/src/lib.rs` | god-module | SQL / IO / 桥接全部集中 |

### 已确认的重组阻力

- `market-store::StorageConfig::project_root()` 通过 `Cargo.toml` + `crates/` 查找项目根目录。
- `market-store` 默认依赖根路径下的 `data/app_state.db`、`config/universe.json`、`sql/...`。
- `app-service` 会直接读取根目录下 `docs/*.md` 并写入 `reports/`。
- `apps/desktop/src-tauri/tauri.conf.json` 当前使用 `../frontend/dist` 相对路径。

### 目前不适合直接大搬家的共享根目录内容

- `Cargo.toml`（workspace 根）
- `config/`, `data/`, `sql/`, `infra/`, `docs/`, `reports/`

---

## 待进一步评估的问题

- 是否需要把现有 `apps/desktop/frontend` 上提为更显式的 `front/`，或保持 `apps/desktop/*` 但增强内部边界。
- `apps/desktop/src-tauri` 与 `crates/*`、`apps/cli` 在目录层次上如何组织，才能既清晰又不破坏 Rust workspace / Tauri 约定。
- 若未来要做显式 front/backend 分层，更适合采用渐进式方案，而不是一步到位的顶层重命名迁移。
- `market-store` 与 `app-service` 的 monolith 拆分优先级：先做 `market-store` domain 拆分，还是先做 `app-service` helper 提取。

---

## 变更记录

| 日期 | 变更内容 |
|------|----------|
| 2026-04-24 | 初始版本：记录顶层结构、前后端边界、重组阻力。 |
| 2026-05-20 | 大规模更新：补全 14 个 workspace member 明细、前端 `lib/features/renderers` 拆分状态、`config/calendars/` 新增、热点文件标注、新增 `PROJECT_STRUCTURE.md` 引用。 |
