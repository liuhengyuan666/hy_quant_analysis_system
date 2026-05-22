# Decisions

## [2026-04-24] 采用 TOOLS.md 作为项目内 AI Agent 最高行为准则

- 背景：项目新增 `TOOLS.md`，需要统一 Agent 的探索、执行与记忆行为。
- 备选方案：
  - 继续沿用会话级临时约束。
  - 引入项目内持久化规则与 memory 机制。
- 决策：以 `TOOLS.md` 及其引用的 `runtime/memory.md` 作为后续协作中的最高行为准则。
- 原因：需要统一探索模式 / 执行模式切换、记忆读写规范与上下文优先级。
- 影响：后续工作前优先读取 `memory/`；关键节点需要维护项目记忆。
- 状态：superseded（2026-05-20：TOOLS.md 已删除，Agent 行为准则改为通过内置 skill 注入，不再依赖项目内文件）

## [2026-05-20] Agent 行为准则从项目内文件（TOOLS.md）转为内置 skill 注入

- 背景：`TOOLS.md` 作为项目内文件维护成本高，且 Agent 每次会话都需要读取；改为通过 opencode skill 机制内置注入后，行为准则与 Agent 运行时绑定，无需项目内文件。
- 备选方案：
  - 方案 A：保留 TOOLS.md，继续作为项目内最高行为准则。
  - 方案 B：删除 TOOLS.md，将行为准则内容迁移为 opencode 内置 skill。
- 决策：采用方案 B。删除 `TOOLS.md`，Agent 行为准则通过 skill 机制注入。
- 原因：
  - skill 注入比文件读取更可靠（不会因文件路径变动而失效）。
  - 减少项目根目录文件数量，降低维护噪音。
  - `memory/` 机制（context/decisions/structure/tech 等）继续保留作为项目级长期记忆，与 Agent skill 不冲突。
- 影响：
  - `TOOLS.md` 已删除。
  - `AGENTS.md` 中的 `TOOLS.md` 引用已移除。
  - `memory/decisions.md` 中 [2026-04-24] 决策标记为 superseded。
  - 后续 Agent 会话不再依赖 `TOOLS.md` 和 `runtime/memory.md` 文件路径。
- 状态：active

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

## [2026-04-26] 当 latest available date 被 signal 层卡住时，优先提示重跑 `compute-signals`

- 背景：排查 `export-report` 默认停在 `2026-04-09` 的问题时，确认根因并不在 export 逻辑，而是当时 `signal_snapshot` 仍停在旧日期。
- 备选方案：
  - 深挖并立即重构 signal/backend 流程。
  - 先增加明确的诊断提示，让用户在 `strategy_preference` 比 `signal_snapshot` 更新时直接知道该重跑哪一步。
- 决策：先在 `PipelineDateDiagnostics` 增加 `alerts`，并在桌面端/文档中明确提示：当 `strategy_preference` 比 `signal_snapshot` 更新时，优先重跑 `compute-signals`。
- 原因：当前最需要的是降低排障成本，而不是先做更大的 backend 改造。
- 影响：CLI、桌面端和文档会对 signal 落后问题给出同一条排障路径；后续若继续深挖 sequencing 问题，再考虑更深入修复。
- 状态：进行中

## [2026-04-26] `compute-signals` 与 refresh 对 signal-vs-strategy 落后关系 fail loud

- 背景：继续深挖后确认，4/9 的导出问题更像一次 sequencing / stale-input 问题，而不是 export 本身的 bug。
- 备选方案：
  - 只保留提示，让用户手工补跑 `compute-signals`。
  - 在 `compute-signals` 和 refresh 末尾加入一致性校验，让系统对 `strategy_preference` 新于 `signal_snapshot` 的状态直接失败。
- 决策：在 backend 增加 signal-vs-strategy 对齐校验 helper，并让 `compute-signals` 与桌面 refresh 在发现落后时 fail loud。
- 原因：当前系统已经允许通过提示降低排障成本，下一步最有价值的是阻止“看起来成功但默认日期仍然落后”的假成功态。
- 影响：后续如果 sequencing 问题再次出现，CLI 和桌面 refresh 会更早失败，而不是把 stale signal 继续传播到 dashboard/export 默认日期。
- 状态：进行中

## [2026-04-26] signal alignment guard 也覆盖 latest-day incompleteness

- 背景：进一步复核后确认，只检查 `strategy_latest > signal_latest` 还不够；如果 `signal_snapshot` 最新日期已经追平，但最新日 coverage 不完整，`dashboard_available` 同样可能被卡住。
- 备选方案：
  - 保持现有 date-lag guard，不处理 incomplete latest day。
  - 复用 `PipelineDateDiagnostics` 已有的 `is_complete` 信息，把 signal 最新日 coverage 不完整也纳入同一条 fail-loud 机制。
- 决策：让 `pipeline_date_alerts()` 同时在“signal 最新日不完整”时发出同类告警，并让 `compute-signals` / refresh 末尾一致性校验同样对此失败。
- 原因：这是和原始问题同一族的错误状态，继续只提示日期滞后会留下明显漏网场景。
- 影响：当前 guard 不仅能防住 signal 日期落后，也能防住 signal 最新日 rows 不完整导致的假成功态。
- 状态：进行中

## [2026-04-26] signal guard 最终统一到 scoped diagnostics alerts

- 背景：进一步复核后发现，只在 `compute-signals` 中做 date-based helper 校验、并只在 refresh 末尾检查 `GLOBAL` scope，还会留下 CN/HK 或 latest-day incompleteness 的漏网情况。
- 备选方案：
  - 保持 `compute-signals` 的局部 helper 校验，refresh 继续只看 Global。
  - 直接复用 `PipelineDateDiagnostics.alerts` 作为 signal guard 的单一事实来源，并让 refresh 检查 `GLOBAL / CN / HK` 三个 scope。
- 决策：让 `compute-signals` 与 refresh 一致地复用 scoped diagnostics alerts；refresh 末尾不再只校验 `GLOBAL`，而校验全部标准 scope。
- 原因：这样可以避免 guard 逻辑在多个地方分叉，减少“日期问题修了但 completeness 或局部 scope 还漏掉”的情况。
- 影响：当前 signal guard 的判断标准已统一到同一套 diagnostics 语义上，后续若再增强，只需要在 diagnostics/alerts 一处扩展。
- 状态：进行中

## [2026-04-26] refresh stage control 先做 suffix-run，不做 stop-at-stage 或持久化 job model

- 背景：在 trust 和 signal guard 都收口后，下一步确认方向是补 `retry failed stage / partial rerun`，但不引入更重的 job-state 或 staging redesign。
- 备选方案：
  - 直接设计复杂的 cancel/resume/stop-at-stage 流程。
  - 先在 Tauri refresh coordinator 层支持 `Retry failed stage` 和 `Run from stage`，并保持后端阶段方法不拆。
- 决策：先实现 suffix-run 语义：用户只能从某个阶段开始并一直跑到结尾；失败后可直接 `Retry failed stage`。当前阶段名集合包含 `ingest`、`indicators`、`macro`、`rotation`、`strategy`、`signals`、`backtests`。
- 原因：这是最小侵入、最不容易破坏现有桌面默认路径的实现，同时能覆盖最常见的恢复场景。
- 影响：当前 refresh 已从“单一按钮”升级成“默认完整刷新 + 轻量阶段控制”。后续若继续增强，再考虑 cancel/resume 或独立 job-state。
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

## [2026-04-26] `/init-deep` 先补边界最强的 AGENTS 层级，而不是平均铺满所有目录

- 背景：trust / recent reports / signal guard / refresh stage control 连续落地后，现有 root / desktop / crates guidance 已明显落后，而 `src-tauri`、`core-domain`、`macro-engine` 又缺少就近约束。
- 备选方案：
  - 继续只维护 root 层 AGENTS。
  - 为更多子目录平均生成 AGENTS。
  - 优先刷新 root / crates / apps/desktop / apps/desktop/frontend，并补齐 `apps/desktop/src-tauri`、`crates/core-domain`、`crates/macro-engine`。
- 决策：本轮 `/init-deep` 采用第三种，先把最容易发生语义漂移、又最常被后续 agent 直接触达的边界刷新到位。
- 原因：这些位置承载了默认用户路径、跨层责任边界、共享 contract 和纯计算语义，是后续最容易因为旧心智而走偏的地方。
- 影响：未来修改 desktop refresh、trust / recent reports、shared DTO 或 macro regime 逻辑时，应先读最近一层 AGENTS，而不是只依赖 root 层印象。
- 状态：完成

## [2026-05-04] 手动同步链路优化先做方案文档，不先直接改实现

- 背景：当前 README 暴露的是一组 CLI 分步命令，但 `dashboard/export` 的默认最新日期由 `signal_snapshot + rotation_rank + market_regime + environment_snapshot` 的最终资格门控共同决定。用户在手动同步时出现了“需要连续跑 2-3 次才推进到较新日期”的体验问题。
- 备选方案：
  - 直接跳到实现，新增 CLI 聚合命令或重构流水线。
  - 先把问题成因、短期优化点和中长期方向收敛成文档，再决定实现顺序。
- 决策：先新增 `docs/手动同步流水线优化方案.md`，把当前问题定义、latest-date gate 逻辑、短期优先方案（CLI 聚合命令 + end-state explanation）和中长期 staging / promote 方向写清楚。
- 原因：这个问题不是单点 bug，而是工程分步命令与产品级最终资格门控之间的抽象错位；先文档化可以避免后续直接进入实现时方向发散。
- 影响：后续如果推进实现，优先级应是 `refresh-all / sync-and-compute` 聚合命令与 latest-gate explanation，而不是立刻做重型 run_id / staging 改造。
- 状态：完成

## [2026-05-07] 采用 Trading-Aware Partial Coverage + 静态 JSON 日历解决 CN/HK 跨市场休市差异

- 背景：CN 与 HK 的法定节假日存在差异，导致 GLOBAL scope 的 dashboard 门控在 HK 休市、CN 开市时被卡住，无法推进到 CN 的最新交易日。数据健康检查也会把 HK 休市期间的 gap 误判为数据缺失。
- 备选方案：
  - 选项 A：在 `config/` 中维护静态 JSON 日历，系统自建 `TradingCalendar` 模块。
  - 选项 B：引入外部交易日历 crate（如 `chrono-tz` + `holidays`）。
  - 选项 C：让门控直接跳过缺失数据的 symbol（不区分休市 vs 缺失）。
  - 选项 D：在 ingestion 层做按市场拆分、按交易日请求。
- 决策：采用选项 A（静态 JSON 日历）+ 选项 C 的变体（Trading-Aware Partial Coverage）。
  - `core-domain` 新增 `TradingCalendar` 模块，`config/calendars/*.json` 维护 CN/HK 休市日。
  - 门控逻辑改为：只检查“该日期期望交易”的 symbol 是否有数据，休市 symbol 不计入 `expected_count`。
  - `TrustSummary` 增加 `non_trading_count` 提示，让用户知道哪些市场因休市被排除。
  - `analyze_gap_metrics` 过滤全休市期间的 gap，避免误告警。
- 原因：
  - 外部 crate 通常偏重欧美市场，CN/HK 覆盖不完整且会增加依赖。
  - 静态 JSON 足够低频研究场景使用，且容易人工校对和补录。
  - Partial Coverage 既解决了门控被卡住的问题，又不会把数据缺失误判为正常休市。
- 影响：
  - GLOBAL dashboard 在 CN 开市+HK 休市时不再被阻塞。
  - 系统新增了跨市场交易日历概念，后续如果有更多市场（如 US）需要纳入，只需扩展 JSON 和 `Market` 枚举。
  - `app-service` 的 `AppContext` 新增了 `calendar` 字段，所有门控/诊断/信任逻辑都需要考虑交易日历。
- 状态：完成

## [2026-05-07] signal-engine 增加 data-starved warning，避免静默 fallback 掩盖数据缺失

- 背景：`build_signal_snapshots` 在缺失 `market_regime` 或 `rotation_rank` 时，静默使用 50.0 / 40.0 的 fallback 默认值。这会导致用户无法区分一个信号是真实计算得出的，还是在缺失关键输入的情况下生成的。
- 备选方案：
  - 保持静默 fallback，仅在文档中说明。
  - 修改 `build_signal_snapshots` 返回 data-starved 统计，并在 `compute_signals` 中 fail loud 或显式 warning。
- 决策：选择方案 B。`build_signal_snapshots` 现在返回 `(Vec<SignalSnapshot>, SignalBuildStats)`，统计 `regime_missing` 和 `rotation_missing`。`compute_signals` 在发现缺失时打印 warning 并将信息写入 `SignalSummary`。
- 原因：数据缺失与休市不同，休市已被 TradingCalendar 过滤，剩下的缺失是真正的数据问题，不应该静默掩盖。
- 影响：
  - CLI 运行 `compute-signals` 时，如果有缺失 regime/rotation 会直接打印 warning。
  - `SignalSummary` 新增了 `data_starved_count` 和 `data_starved_warning` 字段，下游可以进一步展示或告警。
- 状态：完成

## [2026-05-08] 默认 `export-report` 在 latest gate 落后时 fail-loud，不再静默导出旧日报

- 背景：复核日常 CLI 分步链路时，实测出现 `daily_bar` 已到 `2026-05-07`，但 `dashboard_available` 仍停在 `2026-04-30` 的状态；此时默认 `export-report` 会按旧的 dashboard latest 静默导出旧日报。
- 备选方案：
  - 保持当前行为，让用户事后通过 `pipeline-dates` 发现报告日期落后。
  - 在默认导出前检查 latest gate，若 dashboard latest 落后于 freshest market date 则直接失败，并输出 gate alerts。
- 决策：采用方案 B。`export_report_with_scope` 在 `report_date.is_none()` 时先调用 `explain_latest_gate(scope)`；若 `latest_gate_advanced == Some(false)`，拒绝默认导出并提示运行缺失 pipeline stage 或显式传 `--date` 导出历史报告。
- 原因：默认导出代表“当前最新研究快照”，不应在上游 stage 未推进时生成看似成功的旧日报；显式 `--date` 仍然保留历史回看能力。
- 影响：
  - 默认 `cargo run -p quant-cli -- export-report` 不再静默产出旧日期报告。
  - 用户需要先根据 `explain-latest-gate` / `pipeline-dates` 补跑缺失阶段，或明确使用 `--date` 表示导出历史日报。
- 状态：完成

## [2026-05-08] 规划文档审查与精简

- 背景：v2-roadmap、设计规划-v2、README 及多个 AGENTS.md 中关于 strategy/signal/backtest "仍使用 GLOBAL regime" 的表述与代码实现严重脱节。
- 发现：
  - `compute_strategy_preferences` 已遍历 `[Global, CN, Hk]` 并写入 scoped rows
  - `signal-engine` 已按 `regime_basis_scope` 查找 regime
  - `run_backtest` 已接受 `scope` 参数
  - `TrustSummary` / `report-engine` / frontend 已完整展示 provenance
  - Phase 2 checklist（P0-A, P0-B, P1）实际已完成约 85%，但 roadmap 仍标记为"未做"
- 决策：
  - 修正所有文档中的过时 GLOBAL-only 表述
  - 更新 v2-roadmap Phase 2 checklist 以反映真实完成状态
  - 为部分过时的设计文档（V2-Phase1 详细设计、MA30-V1 实施计划、流水线优化方案）添加实现状态标注
  - 不删除任何文档，只做标注和修正
- 原因：文档与代码不一致会导致用户和维护者持续被误导，削弱 Phase 2 已完成的用户价值。
- 影响：后续开发者阅读规划文档时，能准确判断哪些功能已实现、哪些仍待推进。
- 状态：已完成

---

## [2026-05-10] V3 功能（sync-and-export / CLI progress / LLM integration）代码审查完成

- 背景：commit `37f2ae5` 完成了 V3 规划中的 sync-and-export、CLI 进度输出、LLM 集成等功能。由于网络阻塞，`cargo check --workspace` 未能在提交前运行，代码仅通过 rustfmt/LSP 验证。
- 审查发现：
  1. **编译阻断**：`apps/cli/src/main.rs` 缺少 `use anyhow::Context;` 导入，`.context()` 调用会导致编译失败。
  2. **架构风险**：`analyze_report_with_llm` 在同步函数内新建 `tokio::runtime::Runtime`，若未来被 async 上下文调用会 panic。
  3. **死配置**：`LlmConfig.timeout_secs` 被持久化但从未应用到 HTTP 客户端，LLM 调用可能无限挂死。
  4. **密钥管理缺陷**：keyring 成功时向 SQLite 写入空字符串，若后续 keyring 不可用则 fallback 返回空字符串，导致密钥"丢失"。
  5. **明文存储**：SQLite `credential_store` 以纯文本存储 API key，存在安全风险。
  6. **进度回退**：桌面端 `refresh_pipeline` 传入 `None` progress callback，可能丢失前端进度事件。
  7. **进度不一致**：`ingest_daily` 未接入 progress callback，7 个阶段中仅第 1 阶段无法细粒度报告进度。
  8. **其他**：`wiremock` 误放在 workspace 依赖、部分单元测试仅验证字符串常量、计划文件文件计数与实际不符等。
- 决策：
  - 将审查报告写入 `docs/V3-代码审查报告-2026-05-10.md` 作为项目真相源。
  - 在 `cargo check` 恢复后，优先修复编译阻断项（P0）和密钥管理缺陷（P1）。
  - `timeout_secs` 未接入和 Runtime 反模式需在 LLM 功能正式启用前修复。
  - 桌面端进度回调回退需在下次桌面端发布前修复。
- 原因：V3 功能在 happy path 上设计正确，但存在编译、异步、安全和 UX 层面的明确缺陷，需要在正式使用前修复。
- 影响：后续若继续扩展 LLM 功能（多 provider、streaming、前端集成），应以本次审查发现的边界为约束，避免同类问题重复出现。
- 状态：审查完成，待修复
## [2026-05-20] Oracle 数据质量复核报告 — P5：修复 `fetch_market_regimes` GLOBAL-only 过滤导致 CN/HK scope 信号永远 missing regime

- 背景：Oracle 复核报告标注 `regime_missing=17,195`（50.2% 信号）根因为宏观因子历史缺口（P0）。P0 执行 `compute-macro --from 2020` 补全历史后，`compute-signals` 重跑结果 `regime_missing` 仍为 17,195，未改善。深入排查发现 `market-store::fetch_market_regimes` 硬编码 `WHERE market = 'GLOBAL'`，只加载 GLOBAL scope 的 regime 行。而 `signal-engine::build_signal_snapshots` 按 `(date, scope)` 做 exact lookup，CN/HK scoped 策略偏好永远无法匹配到 regime，全部 fallback 50.0。
- 备选方案：
  - 方案 A：修改 `fetch_market_regimes` 移除 GLOBAL 过滤，返回全 scope regime。
  - 方案 B：在信号引擎中改为 scope-agnostic lookup（fallback to GLOBAL regime for CN/HK signals）。
- 决策：采用方案 A，从 SQL 查询中移除 `WHERE market = 'GLOBAL'`，与 `build_market_regimes` 已生成 `GLOBAL/CN/HK` 三 scope 的实现对齐。
- 原因：方案 A 是最小改动（1 行 SQL），且与上游已落地的 per-scope regime 计算语义一致；方案 B 会模糊 scope 语义，与 Phase 2 的设计方向冲突。
- 影响：
  - `regime_missing` 从 17,195 降至 152（-99.1%），`data_starved` 从 52.6% 降至 2.9%。
  - 信号质量大幅提升，CN/HK scope 信号现在使用正确的 scoped regime 而非 fallback。
  - `fetch_market_regimes` 返回值数量从仅 GLOBAL 扩展到三 scope，下游消费者（dashboard/report/export）已在 scope-aware 路径上使用 scoped fetch（`fetch_latest_market_regime_on_or_before`），不受影响。
- 状态：完成

## [2026-05-20] Oracle 数据质量复核报告 — P2：Tencent fallback 解析 turnover 字段

- 背景：`fetch_tencent_daily_bars` 硬编码 `turnover: None`，所有通过 Tencent fallback 获取的 bar 缺失 turnover。当前环境 Eastmoney 主源全部不可达，全部 22 标的走 Tencent fallback，导致 `liquidity_proxy_score` 中的 `turnover_coverage_pct` 系统性为 0。
- 决策：将 `turnover: None` 改为 `turnover: row.get(6).and_then(|v| v.parse().ok())`，安全解析腾讯 K 线接口第 7 列（成交额）。若列不存在或解析失败，退化为 `None`，不影响现有流程。
- 影响：
  - 代码已修复并通过 `cargo check`。存量 ClickHouse 数据仍需 `ingest-daily` 回填才能反映 turnover。
  - 新拉取的腾讯日线将包含 turnover，`liquidity_proxy_score` 计算更准确。
- 状态：完成（代码侧），存量数据回填待执行

## [2026-05-20] Oracle 数据质量复核报告 — P4：注册制板块指数（科创/创业板）跳变阈值差异化

- 背景：`analyze_jump_metrics` 对所有 Index 使用统一 12% 阈值，导致科创50/100、创业板指/50 等注册制板块（20% 涨跌停制度）的真实极端行情被误报为"可疑大波动日"。
- 决策：增加 `REGISTRATION_BOARD_INDICES` 常量集合（`["000688", "000698", "399006", "399673"]`），对匹配到的 Index 使用 22% 阈值，其余 Index 保持 12%，ETF 保持 15%。
- 影响：
  - 科创50/100/创业板指/50 的 `suspicious_jump_count` 从若干 → 0，噪音消除。
  - 阈值硬编码 symbol 列表在 universe 扩展时需同步维护。后续可考虑将 `volatility_regime` 元数据放入 `universe.json`。
- 状态：完成

## [2026-05-20] Oracle 数据质量复核报告 — P0：宏观因子历史回填

- 背景：`macro_snapshot` 表仅覆盖 2025-04-14 之后，导致 2023-2024 年无 regime/environment/strategy_state。根源是此前 `compute-macro` 的 `--from` 参数过晚。
- 决策：执行 `compute-macro --from 2020-01-01 --to 2026-05-19` 完成历史回填。
- 影响：
  - 生成 7,149 macro 行、各 2,442 regime/environment/strategy_state 行（覆盖 GLOBAL/CN/HK）。
  - 配合 P5（scope 过滤修复），信号从 2023 年起拥有完整 regime 数据。
- 状态：完成
