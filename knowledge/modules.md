# Project Codebase Topology (代码库目录拓扑)

> 渐进式披露护栏：本文件仅细化到二级目录，拒绝写入单个文件和函数级细节，防止 Token 过载。

## 1. 物理目录树大纲
```text
rust-quant-analysis-system/
├── apps/
│   ├── cli/              # CLI应用程序（quant-cli）
│   └── desktop/          # 桌面端应用程序（Tauri + Vue 3）
│       ├── frontend/     # Vue 3前端（Vite构建）
│       └── src-tauri/    # Tauri Rust后端
├── crates/               # 核心库crate（20个）
│   ├── app-service/      # 核心服务编排（monolith）
│   ├── backtest-engine/  # 回测引擎
│   ├── core-domain/      # 核心领域模型
│   ├── data-ingestion/   # 数据获取（Eastmoney/Tencent/FRED）
│   ├── gt-regime-generator/ # Ground Truth regime生成
│   ├── indicator-engine/ # 技术指标计算
│   ├── macro-engine/     # 宏观因子与regime分类
│   ├── market-state-extractor/ # 市场状态提取
│   ├── market-store/     # 数据存储抽象（ClickHouse/SQLite）
│   ├── regime-audit/     # Regime审计
│   ├── report-engine/    # 报告生成
│   ├── research-benchmark/ # 研究基准
│   ├── research-context/ # 研究上下文
│   ├── research-renderer/ # 研究渲染
│   ├── research-skills/  # LLM技能路由
│   ├── research-validation/ # 研究验证
│   ├── rotation-engine/  # 轮动排名
│   ├── signal-engine/    # 信号生成
│   ├── strategy-engine/  # 策略引擎
│   └── task-runner/      # 任务运行器
├── config/               # 配置文件
│   ├── calendars/        # 交易日历
│   ├── universe.json     # 标的池配置
│   └── llm.toml          # LLM配置
├── data/                 # 运行时数据目录
├── infra/                # 基础设施
│   └── docker/           # Docker Compose（ClickHouse）
├── docs/                 # 项目文档
├── knowledge/            # KnowledgeGuard资产库
├── memory/               # MemGuard运行时记忆
│   ├── archive/          # 归档记忆
│   ├── context.md        # 当前状态
│   ├── decisions.md      # ADR决策记录
│   ├── history/          # 历史记忆
│   └── traps.md          # 陷阱记录
├── research/             # 研究产物
│   └── agents/           # Agent相关研究
├── reports/              # 生成的报告
├── shadow-production/    # 影子生产环境
└── sql/                  # SQL脚本
```

## 2. 核心模块调用边界与依赖方向

- app-service 依赖：backtest-engine, data-ingestion, core-domain, indicator-engine, macro-engine, market-store, report-engine, research-context, research-renderer, research-skills, rotation-engine, signal-engine, strategy-engine
- data-ingestion 依赖：core-domain, macro-engine
- macro-engine 依赖：core-domain
- signal-engine 依赖：core-domain
- rotation-engine 依赖：core-domain
- desktop (Tauri) 依赖：app-service, core-domain, market-store, report-engine, research-skills
