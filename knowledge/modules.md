# Project Codebase Topology (代码库目录拓扑)

> 渐进式披露护栏：本文件仅细化到二级目录，拒绝写入单个文件和函数级细节，防止 Token 过载。

## 1. 物理目录树大纲
```text
rust-quant-analysis-system/
├── .cargo/               # Rust构建配置（Windows栈大小等）
├── .memguard/            # MemGuard运行时状态（runtime_state.json / search_index.json / backups）
├── .omo/                 # Opencode运行时配置
├── .opencode/            # Opencode技能配置（knowledgeguard等）
├── .sisyphus/            # Sisyphus运行时配置
├── apps/
│   ├── cli/              # CLI应用程序（quant-cli）
│   └── desktop/          # 桌面端应用程序（Tauri + Vue 3）
│       ├── frontend/     # Vue 3前端（Vite构建）
│       └── src-tauri/    # Tauri Rust后端
├── crates/               # 核心库crate（26个物理目录；23个在workspace中，3个未加入：report-builder, reporting, research-validation）
│   ├── app-service/      # 核心服务编排（已模块化：lib.rs ~5,900行 + 15 helper modules，含V8 workspace / execution_replay / llm_history / prompts / scenarios / strategy_perspectives / divergence_ledger（TASK-093 本地 divergence 观察台账））
│   ├── backtest-engine/  # 回测引擎
│   ├── core-domain/      # 核心领域模型（V6新增 core-domain::research 子模块；V7新增 confirmation/recovery/calibration/consensus）
│   ├── data-ingestion/   # 数据获取（Eastmoney/Tencent/FRED）
│   ├── execution-engine/ # 执行层（V5 新增，Pattern Library，收盘前执行过滤）
│   ├── execution-replay/ # V8 Execution Platform：Evidence Registry / Context Integrity / Shadow Mode / Shadow Deployment / Holding Risk / Risk Lifecycle（51个模块，计算+formatter成对组织）
│   ├── gt-regime-generator/ # Ground Truth regime生成
│   ├── indicator-engine/ # 技术指标计算
│   ├── llm-context/      # LLM上下文组装（V6新增）
│   ├── macro-engine/     # 宏观因子与regime分类
│   ├── market-fingerprint-engine/ # 市场指纹引擎（V7.2B新增，历史证据相似度匹配）
│   ├── market-state-extractor/ # 市场状态提取
│   ├── market-store/     # 数据存储抽象（ClickHouse/SQLite）
│   ├── regime-audit/     # Regime审计
│   ├── report-builder/   # 文档输入与Builder组装（V6 Reporting Platform新增；物理目录未加入workspace）
│   ├── report-engine/    # 报告生成
│   ├── reporting/        # 报告领域模型与渲染抽象（V6新增；物理目录未加入workspace）
│   ├── report-renderer/  # 报告渲染器（V6新增）
│   ├── research-benchmark/ # 研究基准
│   ├── research-context/ # 研究上下文（V6 canonical semantic contract）
│   ├── research-skills/  # LLM技能路由
│   ├── research-validation/ # 研究验证（物理目录未加入workspace）
│   ├── rotation-engine/  # 轮动排名
│   ├── signal-engine/    # 信号生成
│   ├── strategy-engine/   # 策略引擎
│   └── task-runner/      # 任务运行器
├── config/               # 配置文件
│   ├── calendars/        # 交易日历（静态JSON，覆盖2024-2027）
│   ├── universe.json     # 标的池配置
│   ├── llm.toml          # LLM配置（gitignored，支持${ENV_VAR}插值）
│   ├── llm.toml.example  # LLM配置示例
│   ├── prompts.toml      # LLM persona 配置（ADR-106：只承载视角指令，含 market_adversarial_lens）
│   ├── scenarios.toml    # 场景权重配置（4 预设场景，RV1 策略多视角）
│   ├── fred.toml         # FRED宏观因子配置（支持enabled开关）
│   ├── fred.toml.example # FRED配置示例
│   └── benchmark-providers.toml # 基准提供者配置
├── data/                 # 运行时数据目录
├── infra/                # 基础设施
│   └── docker/           # Docker Compose（ClickHouse）
├── docs/                 # 项目文档
├── knowledge/            # KnowledgeGuard资产库
├── memory/               # MemGuard运行时记忆
│   ├── archive/          # 归档记忆
│   ├── context.md        # 当前状态
│   ├── decisions.md      # ADR决策记录
│   ├── decisions_archive.md # ADR决策归档
│   ├── glossary.md       # 术语表（MemGuard维护）
│   ├── history/          # 历史记忆
│   ├── product.md        # 产品定义（MemGuard维护）
│   ├── structure.md      # 结构定义（MemGuard维护）
│   ├── tasks_archive.md  # 任务归档
│   ├── tech.md           # 技术约束（MemGuard维护）
│   └── tests/            # 测试用例/压力测试记录
├── research/             # 研究产物
│   └── agents/           # Agent相关研究
├── reports/             # 生成的报告
├── screen_pic/           # 项目截图资源（README引用）
├── shadow-production/   # Shadow Production 运行期产物与运维脚本
│   ├── daily-log.ps1 / weekly-review.ps1  # 每日/每周观察脚本
│   ├── shadow-validation-daily.ps1 / shadow-validation-weekly.ps1  # V8 Phase 2C Shadow Validation 运行脚本
│   ├── kill-criteria.md  # Kill criteria 定义
│   └── historical-replay/ # Historical Replay 产物与报告
├── sql/                 # SQL脚本
└── target/              # Rust构建产物（未在目录树中显式列出，但存在）
```

> 说明：`workspace/` 是 V8 运行时生成的研究资产目录（gitignored），由 `app-service::workspace` 管理，CLI 通过 `--save-evidence` 或 Historical Replay 写入。首次运行前可能不存在。该目录下另有非 Research Asset 的运行时观察子树：`workspace/divergence-ledger/{scope}/{symbol}/{YYYY-MM-DD}.json`（scope 小写，TASK-093，由 `app-service::divergence_ledger` 写入；无 RA-XXXXXX / AssetKind，无 DB 写入）。

## 2. 核心模块调用边界与依赖方向

- app-service 依赖：backtest-engine, core-domain, data-ingestion, execution-engine, execution-replay, indicator-engine, llm-context, macro-engine, market-fingerprint-engine, market-store, report-builder, report-engine, report-renderer, reporting, research-context, research-skills, rotation-engine, signal-engine, strategy-engine
- execution-replay 依赖：execution-engine（消费 ExecutionEvent，V8 Shadow Validation 证据重放）
- data-ingestion 依赖：core-domain, macro-engine
- macro-engine 依赖：core-domain
- signal-engine 依赖：core-domain
- rotation-engine 依赖：core-domain
- execution-engine 依赖：core-domain
- desktop (Tauri) 依赖：app-service, core-domain, market-store, report-engine, research-skills
