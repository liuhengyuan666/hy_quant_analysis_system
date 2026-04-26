# Current Context

## 当前阶段

- 阶段：执行模式
- 当前目标：
  - 收口阶段性文档与 knowledge base。
  - 完成 `/init-deep`，让 root / desktop / shared-contract / macro 边界与当前实现一致。
  - 为 `Recent reports -> compare previous` 保留清晰的下一步实现入口。
- 关键任务：
  - 刷新根/desktop/crates 层 AGENTS，并补齐缺失子层级文件。
  - 把 trust / recent-reports / signal guard / refresh stage control 的当前语义写回阶段总结与 memory。
  - 在继续 `compare previous` 之前，先把 desktop 默认刷新路径与研究结果入口心智固定下来。

## 当前约束

- 当前轮次以知识库 / 文档收口为主，不引入新的重型后端重构。
- 需要保持 desktop 默认路径、scope/provenance 语义、signal fail-loud guard 与 refresh suffix-run 语义一致。

## 当前风险

- 根层/父层 AGENTS 若继续陈旧，会让后续 agent 沿旧的 `main.js` 单文件心智和旧 refresh 语义行动。
- `compare previous` 仍未实现，recent reports 的结果管理入口还差一步闭环。
- 当前仍有一批本地未提交变更，若知识库不先同步，后续提交说明会继续和真实边界脱节。

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
- 当 `strategy_preference` 比 `signal_snapshot` 更新时，`PipelineDateDiagnostics` 现在会给出明确提示：先重跑 `compute-signals`，再信任 dashboard/export 默认日期。
- 对 2026-04-09 导出问题的进一步结论是：当前更像一次运行时序/执行顺序问题，而不是稳定复现的持久写入 bug。
- 当前最新修复方向已经从“提示排障”推进到“机制性 fail loud”：当 `strategy_preference` 新于 `signal_snapshot` 时，`compute-signals` 和 desktop refresh 会直接失败，而不是继续产生看似成功的 stale 状态。
- 当前 latest-date guard 也已经覆盖 `signal_snapshot` 最新日 coverage 不完整的情况，不再只检查日期是否落后。
- 当前 signal guard 已统一复用 `PipelineDateDiagnostics.alerts`，并覆盖 `GLOBAL / CN / HK` 三个标准 scope，而不再只做 Global-only 校验。
- refresh 现在支持 `Retry failed stage` 与 `Run from stage`，并保持 suffix-run 语义：从 `ingest / indicators / macro / rotation / strategy / signals / backtests` 中的指定阶段继续，一直跑到最终一致性校验完成。
- `/init-deep` 当前已完成 root / crates / apps/desktop knowledge-base 刷新，并补齐 `apps/desktop/src-tauri`、`crates/core-domain`、`crates/macro-engine` 的局部 AGENTS，后续 agent 可以在最近边界文件就近获取约束。
