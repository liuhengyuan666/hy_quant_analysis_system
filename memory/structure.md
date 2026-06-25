# Structure Memory

> 当前活跃拓扑 — 目录结构、模块职责、依赖关系。  
> 最后更新：2026-05-21

---

## 顶层结构

```text
hy_quant_analysis_system/
├── apps/                          # 交付面：CLI + Tauri 桌面
│   ├── cli/                       #   quant-cli：clap 命令封装
│   │   └── src/main.rs
│   └── desktop/
│       ├── frontend/              #   Vite + Vue 3 + vue-i18n@11 前端
│       │   ├── src/
│       │   │   ├── main.js        #     根状态、scope/date 流、refresh UI、全局渲染
│       │   │   ├── main-vue.js    #     Vue 应用入口，挂载 App.vue 到 #vue-app
│       │   │   ├── App.vue        #     Vue 根组件，组合所有面板
│       │   │   ├── store.js       #     共享响应式状态（10 属性）+ 事件桥接
│       │   │   ├── i18n.js        #     vue-i18n@11 配置 + setLocale/getLocale
│       │   │   ├── styles.css     #     全局样式 + Vue CSS 变量桥接
│       │   │   ├── lib/
│       │   │   │   └── dashboard-utils.js   # locale-aware 格式化工具函数
│       │   │   ├── components/    #     20+ Vue 面板组件
│       │   │   │   ├── DashboardHero.vue / TrustSummaryPanel.vue / StatusPanel.vue
│       │   │   │   ├── SignalsPanel.vue / SignalDetailModal.vue / BacktestPanel.vue
│       │   │   │   ├── RotationPanel.vue / RegimePanel.vue / EnvironmentPanel.vue
│       │   │   │   ├── BreadthPanel.vue / DataHealthPanel.vue / HealthStrip.vue
│       │   │   │   ├── RecentReportsPanel.vue / UsageGuidesPanel.vue
│       │   │   │   ├── RefreshProgress.vue / DateSelector.vue / TimeContext.vue
│       │   │   │   ├── LanguageToggle.vue / MetricCard.vue / Notice.vue / Skeleton.vue
│       │   │   │   └── App.vue
│       │   │   ├── locales/       #     翻译文件
│       │   │   │   ├── zh.json    #     中文翻译（~280 keys）
│       │   │   │   └── en.json    #     英文翻译（~280 keys）
│       │   │   ├── features/      #     已清空（Phase 0 删除 dead code）
│       │   │   └── renderers/     #     已清空（Phase 0 删除 dead code）
│       │   ├── dist/              #   Vite 构建产物（生成物）
│       │   └── node_modules/
│       └── src-tauri/             #   Tauri Rust 桥接层
│           ├── src/
│           │   ├── main.rs
│           │   └── lib.rs         #     命令注册、refresh 协调器、artifact 打开
│           ├── gen/               #     自动生成 schema（生成物）
│           ├── Cargo.toml
│           └── tauri.conf.json
│
├── crates/                        # Rust Workspace 核心实现层
│   ├── core-domain/               #   共享契约（DTO / Enum / AnalysisScope / TradingCalendar）
│   │   └── src/
│   │       ├── lib.rs             #     Instrument, DailyBar, IndicatorSnapshot, MacroSnapshot,
│   │       │                      #     MarketRegimeSnapshot, EnvironmentSnapshot, SignalSnapshot,
│   │       │                      #     StrategyPreferenceSnapshot, TrustSummary 等
│   │       └── calendar.rs        #     TradingCalendar（静态 JSON 日历，CN/HK 休市日）
│   ├── data-ingestion/            #   外部行情/宏观拉取 + 前复权统一（Eastmoney fqt=1 / Tencent qfq）
│   ├── indicator-engine/          #   技术指标：MA, EMA, MACD, RSI, ATR, VOL_MA
│   ├── macro-engine/              #   宏观因子规范化 + per-scope market regime 评分
│   ├── rotation-engine/           #   相对强弱 RS / 动量排名
│   ├── strategy-engine/           #   四类策略偏好评分（ValueLeft/TrendPullback/TrendBreakout/MomentumRight）
│   ├── signal-engine/             #   最终信号生成 + data-starved 统计（regime_missing/rotation_missing）
│   ├── backtest-engine/           #   信号驱动回测（含 drawdown 控制、strategy-state 集成）
│   ├── report-engine/             #   DashboardSnapshot / TrustSummary / markdown 报告渲染
│   ├── market-store/              #   ClickHouse + SQLite 持久化边界（SQL/IO/date-gating 全部集中于此）
│   ├── app-service/               #   编排门面：trust 组装、refresh 守卫、pipeline diagnostics、recent reports
│   └── task-runner/               #   占位工具 crate
│
├── config/                        # 运行时配置
│   ├── universe.json              #   标的池（symbol/name/market/category/provider 元数据）
│   └── calendars/
│       ├── cn_holidays.json       #   CN 休市日历（2024-2027）
│       └── hk_holidays.json       #   HK 休市日历（2024-2027）
│
├── docs/                          # 手册 / 架构 / 设计文档
│   ├── 日常操作手册.md
│   ├── 分析使用手册.md
│   ├── 系统架构与数据流.md
│   ├── 功能模块与处理逻辑.md
│   ├── 文档状态说明.md
│   ├── V2-Phase1-环境层详细技术设计.md
│   ├── 手动同步流水线优化方案.md
│   ├── 市场情绪指标-MA30规划.md
│   ├── 市场情绪指标-MA30-V1实施计划.md
│   ├── 阶段性更新-2026-04-26.md
│   ├── 阶段性更新-2026-05-07.md
│   └── AGENTS.md
│
├── infra/                         # 基础设施
│   └── docker/
│       └── docker-compose.yml     #   ClickHouse 容器定义
│
├── sql/                           # 存储初始化 DDL
│   ├── clickhouse/001_init.sql
│   └── sqlite/001_init.sql
│
├── memory/                        # 项目记忆系统
│   ├── context.md                 #   当前运行状态（阶段/目标/约束/风险）
│   ├── decisions.md               #   ADR 决策记录（append-only）
│   ├── structure.md               #   本文件
│   ├── tech.md                    #   技术栈与架构原则
│   ├── product.md                 #   产品定位与场景
│   ├── glossary.md                #   术语规范化
│   └── history/                   #   里程碑摘要（18 条目，2026-04-24 ~ 2026-05-20）
│
├── runtime/                       # Agent 运行规范
│   └── memory.md                  #   （历史遗留，当前行为准则已通过 skill 注入）
│
├── .omo/                          # OpenCode 工作目录
│   ├── plans/                     #   改造方案文档
│   ├── evidence/                  #   验证证据
│   └── run-continuation/          #   会话续接状态
│
├── data/                          # 本地轻状态（SQLite app_state.db 等 — 运行时产物）
├── reports/                       # 导出报告（daily-report-*.md, data-health-*.md — 运行时产物）
├── target/                        # Rust 构建产物（生成物）
│
├── 设计规划-v2.md                 # V2 路线图（归档参考）
├── 设计规划.md                    # 原始设计规划（归档参考）
├── 实施路径-v1.md                 # V1 实施路径（归档参考）
├── 数据源方案评审.md              # 数据源方案评审（归档参考）
├── 数据质量复核报告-Oracle.md     # Oracle 数据质量复核报告（归档参考）
│
├── Cargo.toml                     # Workspace 根（14 members, resolver=2）
├── Cargo.lock
├── README.md                      # 主入口文档（当前事实来源）
├── PROJECT_STRUCTURE.md           # 项目结构说明书（Mermaid 依赖图 + 结构化全貌）
├── AGENTS.md                      # Agent 协作入口（项目知识库索引）
├── jsconfig.json                  # VS Code / 前端路径别名
└── .gitignore
```

---

## Workspace 成员

| # | Member | 类型 | 核心职责 |
|---|--------|------|----------|
| 1 | `apps/cli` | App | `quant-cli`：薄 clap 命令行封装，全部逻辑委托给 `app-service` |
| 2 | `apps/desktop/src-tauri` | App | `quant-desktop`：Tauri 桥接，命令注册、refresh 协调、artifact 打开 |
| 3 | `crates/core-domain` | Lib | 共享 DTO / Enum / TradingCalendar — 依赖最轻（chrono + serde） |
| 4 | `crates/data-ingestion` | Lib | 外部行情/宏观拉取，provider 验证，前复权统一 |
| 5 | `crates/indicator-engine` | Lib | MA/EMA/MACD/RSI/ATR/VOL_MA 纯计算 |
| 6 | `crates/macro-engine` | Lib | 宏观因子规范化 + per-scope regime 评分（不 fetch，不 persist） |
| 7 | `crates/rotation-engine` | Lib | RS/动量排名计算 |
| 8 | `crates/strategy-engine` | Lib | 四类策略偏好评分 |
| 9 | `crates/signal-engine` | Lib | 最终信号生成 + data-starved 统计 + signal-vs-strategy 对齐守卫 |
| 10 | `crates/backtest-engine` | Lib | 信号驱动回测（drawdown 控制、strategy-state 集成） |
| 11 | `crates/report-engine` | Lib | DashboardSnapshot / TrustSummary DTO + markdown 报告渲染 |
| 12 | `crates/market-store` | Lib | ClickHouse + SQLite IO — 唯一持久化边界 |
| 13 | `crates/app-service` | Lib | 编排门面：trust 组装、refresh 管道、pipeline diagnostics、recent reports |
| 14 | `crates/task-runner` | Lib | 占位工具（尚无实质功能） |

---

## 分层边界

| 层 | 位置 | 约束 |
|---|---|---|
| **前端** | `apps/desktop/frontend/src/` | Vite + Vue 3 + Plain JS 混合架构。不得包含量化逻辑。Vue 组件迁移已完成（20+ 组件），Plain JS 保留 hero/trust/refresh/reports 入口。i18n 通过 vue-i18n@11 实现（默认中文）。 |
| **桥接** | `apps/desktop/src-tauri/` | Tauri 命令注册 + refresh 协调。保持薄封装，不得包含业务逻辑。 |
| **后端计算** | `crates/*` (engine crates) | 纯计算 crate 不 fetch、不 persist。`macro-engine` 严格执行此约束。 |
| **持久化** | `crates/market-store/` | 唯一数据库访问边界。ClickHouse + SQLite 全部 IO 集中于此。 |
| **编排** | `crates/app-service/` | 组装 trust、驱动 refresh 管道、生成 diagnostics。最高耦合层，但不得直接访问 DB。 |
| **契约** | `crates/core-domain/` | 共享类型。字段变更影响全链路。 |

---

## Crate 依赖方向

```
apps/cli ──────────────┐
apps/desktop/src-tauri ┤
                        ▼
                  app-service ──────┬── report-engine
                                    ├── market-store ──→ ClickHouse / SQLite
                                    ├── [all engine crates]
                                    └── core-domain
                                         ▲
                  [all engine crates] ──┘
```

- `core-domain` 在最底层，被所有 crate 依赖。
- Engine crates（indicator/macro/rotation/strategy/signal/backtest）依赖 `core-domain`，互不依赖。
- `data-ingestion` 依赖 `core-domain`，不依赖任何 engine。
- `market-store` 依赖 `core-domain`，不依赖 engine。
- `app-service` 依赖所有 crate，对外暴露统一 API。
- `apps/cli` 和 `apps/desktop/src-tauri` 只依赖 `app-service`。

---

## 已知热点

| 文件 | 规模 | 状态 |
|------|------|------|
| `crates/market-store/src/lib.rs` | god-module | SQL/IO/桥接全部集中，待 domain 拆分 |
| `crates/app-service/src/lib.rs` | 4,083 行 | 已模块化：AppContext 高层编排 + 7 个 helper 模块（core, trust, breadth, dashboard, llm, sync, config_loader）。Dashboard 优化已移走 `check_data_health` 热路径调用，但 lib.rs 仍待进一步拆分 |

---

## 重组约束

以下路径被多个 crate 硬引用，不可随意移动：

- `Cargo.toml` — workspace 根，`market-store` 通过它定位项目根
- `config/universe.json` — `market-store` 直接读取
- `data/app_state.db` — SQLite 默认路径
- `sql/*.sql` — 初始化 DDL 路径
- `docs/*.md` — `app-service` 读取（usage guides）+ `reports/` 写入
- `apps/desktop/src-tauri/tauri.conf.json` — 使用 `../frontend/dist` 相对路径
