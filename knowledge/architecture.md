# Project Architecture Topology (技术架构拓扑)

## 1. 显式技术栈清单
| 层级 | 采用技术/主流框架 | 核心版本 | 关键用途 |
| :--- | :--- | :--- | :--- |
| Backend/Core | Rust | 2021 edition | 核心量化计算引擎 |
| Desktop/Frontend | Tauri + Vue 3 | Tauri 2, Vue 3.5 | 桌面端应用容器与UI |
| Frontend Markdown | marked | 18.x | LLM markdown 渲染（转义后解析，防注入） |
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
- app-service: 核心服务编排（已模块化，lib.rs ~5,900行 + 14 helper modules：breadth / config_loader / core / dashboard / execution_replay / llm / llm_history / prompts / research_evidence / scenarios / strategy_perspectives / sync / trust / workspace，后续仍可拆分）
- execution-replay: V8 Execution Platform 证据重放引擎（51 个模块，计算+formatter 成对组织）：Evidence Registry、Context Integrity Gate/Validator/Audit、Shadow Mode、Shadow Deployment、Holding Risk Bundle/Calibration/Persistence、Risk Lifecycle、Decision Gate/Margin、Bearish Analysis、Transition Analysis 等
- core-domain: 核心领域模型；V7 新增 `core-domain::research` 子模块（confirmation / recovery / calibration / consensus / stretch / rotation）
- data-ingestion: 数据获取（Eastmoney/Tencent/FRED）
- execution-engine: 执行层（V5 新增，Pattern Library，收盘前执行过滤）
- macro-engine: 宏观因子计算与regime分类
- market-fingerprint-engine: 市场指纹引擎（V7.2B 新增），提供 Normalizer、DistanceMetric、SimilarityMatcher、OutcomeProfiler
- market-state-extractor: 市场状态提取
- rotation-engine: 轮动排名计算
- signal-engine: 信号生成
- backtest-engine: 回测引擎
- report-engine: 报告生成
- report-builder: 文档输入与 Builder 组装（V6 Reporting Platform 新增；物理目录未加入 workspace）
- reporting: 报告领域模型与渲染抽象（V6 Reporting Platform 新增；物理目录未加入 workspace）
- report-renderer: 报告渲染器（V6 新增）
- llm-context: LLM 上下文组装（V6 新增）
- market-store: 数据存储抽象
- research-context: 研究上下文（V6 canonical semantic contract）
- research-skills: LLM技能路由

## 3. 全局架构约束

- Workspace 结构：25 个 members（apps/cli, apps/desktop/src-tauri, 23 个 crates）
- 注意：以下 crates 物理存在但未加入 workspace members：crates/research-validation、crates/report-builder、crates/reporting
- V7 Research Platform 1.0 已冻结（ADR-077）：Observation (V6) + Market Evolution (V7.1) + Historical Evidence (V7.2) + Research Synthesis (V7.3) 的语义架构、接口和职责全部冻结；新增内容属于 Research Content Evolution，只能通过现有冻结层消费
- 数据源策略：Eastmoney主源，Tencent兜底，FRED宏观因子；FRED 运行时支持已持久化 `macro_snapshot` 历史回退
- 统一日线口径：Eastmoney fqt=1，Tencent qfq（前复权）
- 当前环境限制：Eastmoney从当前环境不可达，全部标的走Tencent fallback
- Tencent 当日 bar 定稿时分标的不一致：股票/指数/行业ETF 收盘后较快，黄金ETF（518880，跟踪SGE夜盘品种）当日 qfq bar 定稿明显更晚，且 CDN 节点间不同步（同一请求形状不同时刻返回结果不一致）；当日 ingest 缺该 bar 属正常，次日 refresh 自动补齐，无需代码干预
- 静态JSON日历覆盖2024-2027，后续需要人工维护
- TradingCalendar当前只覆盖CN/HK
- P2 turnover修复仅影响新拉取数据，存量ClickHouse数据需ingest-daily回填
- V6 Reporting Platform 已冻结：Production Surface（DashboardSnapshot / sync-and-export / ResearchContext）稳定，新增消费者应建立在此平台之上
- **V7 Research Platform 1.0 已冻结**：Observation / Evolution / Evidence / Consensus 四层语义架构稳定，新增市场观测内容只能作为 Research Content Evolution 输入现有层，不允许修改 Semantic Architecture
- 分层架构不可变规则见 `docs/architecture-invariants.md`（ADR-069）：数据所有权、语义所有权、展示所有权、渲染所有权、消费者边界
- **V8 Research Asset**：统一身份 `RA-XXXXXX`、统一生命周期 `Draft → Verified → Published → Superseded → Archived`，本地 `workspace/` 持久化（gitignored）；P3（Evidence Score/Weight）在 1000+ 资产 / 30 天 Replay 稳定 / 2 周期 Calibration 稳定前不得启动
- **V8 Execution Platform（Phase 2C Shadow Validation）**：`execution-replay` crate 提供 Evidence → Risk State → Shadow Assessment 链路；当前为只读观察，禁止 DecisionEngine 消费 ShadowRiskAssessment、禁止修改 ExecutionPolicy、禁止自动交易、禁止新增 Evidence
