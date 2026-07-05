# Project Architecture Topology (技术架构拓扑)

## 1. 显式技术栈清单
| 层级 | 采用技术/主流框架 | 核心版本 | 关键用途 |
| :--- | :--- | :--- | :--- |
| Backend/Core | Rust | 2021 edition | 核心量化计算引擎 |
| Desktop/Frontend | Tauri + Vue 3 | Tauri 2, Vue 3.5 | 桌面端应用容器与UI |
| Database/Store | ClickHouse + SQLite | ClickHouse 0.13, rusqlite 0.32 | 时序数据存储与本地状态 |
| Data Ingestion | attohttpc + serde | attohttpc 0.28 | HTTP数据获取与序列化 |
| Async Runtime | Tokio | 1.x | 异步运行时 |
| LLM Integration | async-openai | 0.34 | OpenAI-compatible API调用 |
| Config | TOML + clap | toml 0.8, clap 4 | 配置管理与CLI解析（LLM/FRED均使用TOML） |
| Observability | tracing + tracing-subscriber | 0.1 / 0.3 | 日志与链路追踪 |
| Security | keyring | 3.x | API Key安全存储（LLM） |
| Serialization | serde + serde_json + serde_yaml | 1.x / 0.9 | 数据序列化 |

## 2. 组件通信与数据流拓扑

数据流：
```
ingest-daily → compute-indicators → compute-macro → compute-rotation → compute-strategy-preferences → compute-signals → run-backtest → export-report
```

关键组件：
- app-service: 核心服务编排（已模块化，lib.rs ~4,890行 + 7 helper modules，后续仍可拆分）
- data-ingestion: 数据获取（Eastmoney/Tencent/FRED）
- execution-engine: 执行层（V5 新增，Pattern Library，收盘前执行过滤）
- macro-engine: 宏观因子计算与regime分类
- rotation-engine: 轮动排名计算
- signal-engine: 信号生成
- backtest-engine: 回测引擎
- report-engine: 报告生成
- report-builder: 文档输入与 Builder 组装（V6 Reporting Platform 新增）
- reporting: 报告领域模型与渲染抽象（V6 Reporting Platform 新增）
- report-renderer: 报告渲染器（V6 Reporting Platform 新增）
- llm-context: LLM 上下文组装（V6 新增）
- market-store: 数据存储抽象
- research-skills: LLM技能路由

## 3. 全局架构约束

- Workspace 结构：23 个 members（apps/cli, apps/desktop/src-tauri, 21 个 crates）
- 注意：以下 crates 物理存在但未加入 workspace members：crates/research-validation、crates/report-builder、crates/reporting
- 数据源策略：Eastmoney主源，Tencent兜底，FRED宏观因子；FRED 运行时支持已持久化 `macro_snapshot` 历史回退
- 统一日线口径：Eastmoney fqt=1，Tencent qfq（前复权）
- 当前环境限制：Eastmoney从当前环境不可达，全部标的走Tencent fallback
- 静态JSON日历覆盖2024-2027，后续需要人工维护
- TradingCalendar当前只覆盖CN/HK
- P2 turnover修复仅影响新拉取数据，存量ClickHouse数据需ingest-daily回填
- V6 Reporting Platform 已冻结：Production Surface（DashboardSnapshot / sync-and-export / ResearchContext）稳定，新增消费者应建立在此平台之上
- 分层架构不可变规则见 `docs/architecture-invariants.md`（ADR-069）：数据所有权、语义所有权、展示所有权、渲染所有权、消费者边界
