# Current Context

## 当前阶段

- 阶段：执行模式
- 当前目标：
  - 进行第一轮清理与归档。
  - 在不做顶层目录迁移的前提下提升代码库可读性。
  - 为后续热点文件拆分做准备。
- 关键任务：
  - 统一文档状态说明，区分当前事实、活跃设计、历史归档与运行产物。
  - 移除前端未使用依赖与明显陈旧项。
  - 记录下一轮可执行的热点拆分目标。

## 当前约束

- 时间：当前轮次以低风险清理与归档为主，不直接进行大规模重构。
- 技术限制：需要兼顾 Rust workspace、Tauri 目录约定与现有文档/脚本路径。

## 当前风险

- 规划文档较多，可能存在口径陈旧或重复。
- 生成产物与源码并存，容易混淆“应清理”与“应保留”的边界。
- 顶层目录重组可能影响 Cargo workspace、Tauri 配置、文档路径与脚本命令。

## 当前发现

- 明显可清理/归档对象包括：`target/`、前端 `node_modules/`、前端 `dist/`、`reports/`、`data/`、部分 IDE 目录。
- 当前更真实的源码与架构事实主要由 `README.md`、`docs/系统架构与数据流.md`、`docs/功能模块与处理逻辑.md`、workspace/AGENTS 文档共同定义。
- `apps/desktop/frontend/src/main.js`、`crates/app-service/src/lib.rs`、`crates/market-store/src/lib.rs` 是当前最明显的热点与后续拆分候选点。
- 当前已规划但未完成的主线集中在：scope 信任闭环、真实 scoped strategy/signal/backtest、双阶段策略状态机、结构化解释与 drilldown、执行层原型、工作流效率改进。
- 第一轮清理已新增统一文档状态说明，并将根目录历史规划文档标注为归档参考。
- 前端已确认不再依赖 `vue`，构建仍可通过。
- 第二轮已开始拆解前端热点文件：`main.js` 的首批纯工具函数已迁移到 `apps/desktop/frontend/src/lib/dashboard-utils.js`。
- 当前轮次继续沿前端 area split 推进：guide viewer 相关逻辑已迁移到 `apps/desktop/frontend/src/features/usage-guides.js`。
- 当前轮次继续沿前端 area split 推进：data-health 相关缓存、加载、导出、渲染与事件绑定已迁移到 `apps/desktop/frontend/src/features/data-health.js`。
- 当前轮次继续沿 render area split 推进：environment layer 与 watchlist breadth renderer 已迁移到 `apps/desktop/frontend/src/renderers/environment-breadth.js`。

## 当前已确认的功能设计方向

- 需要优先修补 `scope-aware` 环境解释与 `GLOBAL` signal/backtest 之间的语义裂缝。
- 可信度表达应收敛为“主可信度结论 + freshness / data-health 证据层”。
- `Environment layer` 与 `Watchlist Breadth` 暂时双面板保留，但需要持续避免它们被用户误读为两套独立结论。
- `Recent reports` 未来应向“研究结果管理入口”演进，而不是停留在导出路径列表。
- `desktop refresh` 是默认用户路径；CLI 手动链路保留为显式工程路径。
- `Recent reports` 当前已进入“研究结果管理入口”的第一阶段：daily reports 可回跳 matching snapshot，所有 artifact 可复制路径。
- `Recent reports` 当前已进入第二阶段：daily reports 可回跳 matching snapshot，所有 artifact 可直接打开并复制路径。

## 当前执行焦点

- 正在执行 P0：统一当前 truth-source docs 与桌面端 provenance / trust 展示，减少 scope 与 signal/backtest 语义误读。
- 当前轮次优先解决的是“怎么读懂现在系统”，不是继续扩大功能面。

## 当前最新进展

- `TrustSummary` 现已补充 freshness/data-health 结构化 digest，用于承载更明确的证据层摘要。
- 桌面端现在以 `renderTrustSummaryPanel()` 作为主可信度入口区块，而不再只把 trust summary 当成单行 notice。
- `Pipeline freshness` 与 `Data health` 继续保留为证据层 / drilldown，而不是再与 trust summary 并列竞争主入口地位。
