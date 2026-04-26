# Decisions

## [2026-04-24] 采用 TOOLS.md 作为项目内 AI Agent 最高行为准则

- 背景：项目新增 `TOOLS.md`，需要统一 Agent 的探索、执行与记忆行为。
- 备选方案：
  - 继续沿用会话级临时约束。
  - 引入项目内持久化规则与 memory 机制。
- 决策：以 `TOOLS.md` 及其引用的 `runtime/memory.md` 作为后续协作中的最高行为准则。
- 原因：需要统一探索模式 / 执行模式切换、记忆读写规范与上下文优先级。
- 影响：后续工作前优先读取 `memory/`；关键节点需要维护项目记忆。
- 状态：进行中

## [2026-04-25] 目录整理采用方案 A：先内部清理与渐进拆解，再考虑顶层重组

- 背景：项目希望提升可读性，并评估过更显式的 front/backend 目录分层。
- 备选方案：
  - 方案 A：保持当前顶层结构，先做低风险清理、归档和热点文件渐进拆解。
  - 方案 B：渐进式做更显式的 front/backend 归组。
  - 方案 C：直接推进顶层 `/front` `/backend` 重组。
- 决策：本阶段采用方案 A，先处理文档真相源、历史文档归档语义、未使用依赖与热点文件拆分准备，不进行顶层目录搬迁。
- 原因：当前 `market-store`、`app-service`、`src-tauri` 与根目录路径存在真实耦合，先做内部清理更稳妥。
- 影响：后续执行顺序以“清理与标注 → 热点拆分 → 再评估目录重组”为主。
- 状态：进行中

## [2026-04-25] `main.js` 拆分先抽离纯工具层，不先打散状态流与渲染流

- 背景：`apps/desktop/frontend/src/main.js` 是当前最明显的前端热点文件，但其事件绑定、状态管理、Tauri 调用与视图渲染仍集中在同一入口。
- 备选方案：
  - 先按 render / events / loaders 大拆。
  - 先抽离不依赖状态的纯工具函数，再继续拆视图与状态流。
- 决策：先把 formatting / normalization / markdown / tone 这类纯函数迁移到 `src/lib/dashboard-utils.js`，保留 `main.js` 中的状态对象、异步加载、事件绑定与主渲染流程。
- 原因：纯工具层最容易验证等价性，风险最低，也能立即降低 `main.js` 顶部噪音。
- 影响：后续前端拆分应继续按 `snapshot / health / guides / render` 之类的状态边界推进，而不是把新工具代码继续堆回 `main.js`。
- 状态：进行中

## [2026-04-25] `main.js` 第二段拆分优先抽离 usage-guides slice

- 背景：在纯工具层抽离后，usage-guides 已成为最独立的前端功能块，具备自己的加载、开关、渲染和事件绑定。
- 备选方案：
  - 继续零散抽 helper。
  - 直接拆 guide 相关状态边界。
  - 先动 data health / snapshot 主流程。
- 决策：先把 guide viewer 相关逻辑迁移到 `src/features/usage-guides.js`，保留 `main.js` 对全局 state 与主 render 流的控制权。
- 原因：guides 按项目约定本就应与 dashboard 热路径解耦，是下一刀风险最低、收益最直接的 area split。
- 影响：后续前端拆分可继续在 `health`、`snapshot`、`render` 这几条状态边界上推进。
- 状态：进行中

## [2026-04-25] `main.js` 第三段拆分优先抽离 data-health slice

- 背景：guides 抽离后，data-health 成为下一个结构清晰且与主 dashboard 快照流相对解耦的功能块，具备自己的缓存、加载、导出、渲染和事件绑定。
- 备选方案：
  - 先拆 snapshot 主流程。
  - 先拆 render 主框架。
  - 先拆 data-health 独立 slice。
- 决策：先把 data-health 相关逻辑迁移到 `src/features/data-health.js`，保留 `main.js` 对全局 render 调度和 dashboard 主流程的控制。
- 原因：data-health 既有独立交互面，又有明确的文档语义（异步、会话缓存、手动刷新），适合作为低风险的第三段拆分。
- 影响：后续前端拆分应继续向 `snapshot` 或 `render` 这样的主流程边界推进。
- 状态：进行中

## [2026-04-25] `main.js` 第四段拆分优先抽离 environment/breadth renderers

- 背景：在 utils、guides、data-health 之后，`renderEnvironmentPanel()` 与 `renderWatchlistBreadthPanel()` 成为最适合继续抽离的 render-only cluster。
- 备选方案：
  - 先拆 snapshot orchestration。
  - 先拆 environment + breadth 这组 paired renderers。
  - 先拆 signal / backtest / rotation 等更大的 render cluster。
- 决策：先把 environment layer 与 watchlist breadth 的 paired renderers 迁移到 `src/renderers/environment-breadth.js`，保留 `main.js` 对主 render 组合和 async 主流程的控制。
- 原因：这两块本来就共享“解释层 + 原始 proxy 视图”的产品语义，而且只依赖 snapshot 展示数据与 render helper，不涉及异步 orchestration。
- 影响：后续前端拆分可继续在剩余 render cluster 或 snapshot 边界上推进，但不需要再回头处理这组 paired panels。
- 状态：进行中

## [2026-04-25] 功能设计复盘后的 5 项确认结论

- 背景：对当前系统进行了功能设计 review，重点检查了 scope 语义、可信度表达、环境层/广度展示、历史研究入口与刷新路径。
- 备选方案：
  - 继续保留现有分散设计，不做明确产品收敛。
  - 针对关键设计冲突给出明确确认，并以此指导后续实现顺序。
- 决策：确认以下 5 点作为后续产品与实现方向：
  1. 当前最大设计问题是“scope-aware 环境解释”与“GLOBAL signal/backtest 语义”之间的裂缝。
  2. `Pipeline freshness`、`Data health`、`trust summary` 的产品表达应收敛为“一个主可信度入口 + 两个证据层”。
  3. `Environment layer` 与 `Watchlist Breadth` 暂时保留双面板，但要明确其分别代表解释层与原始 proxy 视图。
  4. `Recent reports` 不再只作为文件路径列表看待，后续应升级为“研究结果管理入口”。
  5. `desktop refresh` 作为默认用户路径；CLI 手动全链路保留为工程/高级用户路径。
- 原因：这样既能保持当前系统的研究型分层优势，又能减少用户心智分裂与历史设计残留造成的误读。
- 影响：后续优先级应围绕“语义一致性 → 可信度主入口 → 研究结果管理 → 其余体验优化”推进，而不是继续平铺功能面。
- 状态：进行中

## [2026-04-25] P0 先做语义一致性收口，不先做更大语义重构

- 背景：功能设计复盘确认后，当前最高优先级是修补 `scope-aware` 环境解释与 signal/backtest 用户理解之间的语义裂缝。
- 备选方案：
  - 直接重写后端语义链或 trust 聚合逻辑。
  - 先统一主参考文档与前端 provenance/trust 展示，再决定是否继续深入语义重构。
- 决策：先完成文档与展示层的 P0 语义收口：更新 truth-source docs 的 scope/provenance 表述，并补强桌面端对 signal/backtest/trust provenance 的直观展示。
- 原因：当前代码层已经暴露出大量 provenance 字段，最先需要解决的是用户和维护者继续被旧文档/弱展示误导。
- 影响：后续可在此基础上继续推进“一个主可信度入口 + 两个证据层”，而不是继续让文档、UI、实现各讲各的版本。
- 状态：进行中

## [2026-04-25] trust summary 先提升为主入口，再决定是否继续深挖后端重构

- 背景：P0 文档与展示收口后，下一阶段确认的方向是把 `trust summary` 真正推进成“一个主可信度入口 + 两个证据层”。
- 备选方案：
  - 先大改后端信任聚合与 transport/API 结构。
  - 先做最小 contract 收敛：扩展 `TrustSummary` digest，并在前端将 trust summary 升格为主 panel。
- 决策：先扩展 `TrustSummary` 的 freshness/data-health 证据摘要，并在桌面端把 trust summary 提升为主入口区块；`Pipeline freshness` 与 `Data health` 继续保留为证据层与下钻层。
- 原因：当前问题不在“trust 不存在”，而在“trust 仍然像一个附属 notice”。先解决入口层级最有价值，也最不容易引发大范围语义回归。
- 影响：后续如果继续做 deeper refactor，应围绕 trust contract 和 transport/API 统一推进，而不是再回到纯展示修补。
- 状态：进行中

## [2026-04-25] `Recent reports` 先升级成可操作入口，不先做 schema/API 大改

- 背景：功能设计复盘已确认 `Recent reports` 不应继续停留在导出路径列表，而应向研究结果管理入口演进。
- 备选方案：
  - 先直接改 storage/schema，为 report artifact 增加显式 `scope`、比较链路等元数据。
  - 先做前端最小可用动作：从现有 `report_type / report_date / artifact_path` 出发，补上 snapshot jump 与 copy path。
- 决策：先实现 `Open snapshot`（仅 `DAILY_REPORT*`）与 `Copy path`（所有 artifact），保持后端 schema 和 Tauri bridge 不变。
- 原因：这是从“路径列表”走向“研究结果管理入口”的最高性价比第一步，同时复用现有 snapshot/date/scope 状态流，不需要立即引入新 schema 迁移。
- 影响：后续如果继续增强，可在此基础上再做 `compare previous`、first-class `scope` metadata、artifact open/reveal 等动作。
- 状态：进行中

## [2026-04-25] `Recent reports` 第二阶段优先补 `Open artifact`，不先做 capability/plugin 扩张

- 背景：第一阶段已经让 daily reports 可以回跳 matching snapshot，但 `DATA_HEALTH_REPORT` 等 artifact 仍然停留在“只能复制路径”的状态。
- 备选方案：
  - 直接引入新的 Tauri plugin / capability 体系做 artifact open/reveal。
  - 先加一个 app-local 原生命令，只允许打开 `reports/` 目录下的真实 artifact。
- 决策：先在 `src-tauri` 加入 repo-local 的 `open_report_artifact` 命令，并在 recent-reports slice 中接入 `Open artifact` 动作。
- 原因：这是最小侵入、最可控的下一步，不需要立刻把工作升级成 schema/API 或插件治理问题。
- 影响：`Recent reports` 现在对 daily reports 和 data-health reports 都有直接可用的 artifact 动作；后续若继续增强，再考虑 reveal/opener plugin 或 first-class metadata。
- 状态：进行中
