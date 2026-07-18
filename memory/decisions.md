> Historical decisions are in [decisions_archive.md](./decisions_archive.md)

## ADR-026: Memory 体系清理与状态同步

**Status:** Accepted

### Context
当前 Memory 体系存在三个问题：1) glossary 术语不完整（19个）；2) decisions 状态需要确认；3) archive 目录结构需要初始化。

### Decision
执行 Memory 体系清理：补充 12 个缺失术语（available_dates_ms、pipeline_diagnostics、refresh_pipeline、dashboard_bundle 等），确认 25 条决策状态，初始化 archive 目录结构。

**Tags:** memory, maintenance, documentation

## ADR-027: ClickHouse 日期查询性能优化（Oracle 复核修正）

**Status:** Accepted

### Context
Dashboard 加载性能瓶颈：`available_dates_ms` 耗时 24 秒。根因是 `fetch_dashboard_available_dates` 查询使用 IN 子句导致双表全扫描。

### Decision
实施两层优化：1) 重写主查询使用 JOIN 替代 IN 子句，避免双表全扫描；2) 在 AppContext 中添加 AvailableDatesCache 内存缓存（TTL 5分钟），数据刷新后自动清除。Oracle 复核后移除了 90 天限制，因为它会破坏历史日期查询（dashboard-snapshot --date 和 export-report --date）。

**Tags:** performance, clickhouse, caching, dashboard, oracle-reviewed

## ADR-028: rotation_missing 根因分析：历史窗口不足导致的预期行为

**Status:** Accepted

### Context
signal-engine 中 840 个 rotation_missing 条目需要排查原因。

### Decision
rotation_missing 是预期行为，不是 bug。根因是 rotation-engine 在计算 rs_20 时需要至少 20 天历史数据（index >= 20），导致每个标的的前 20 天无法生成 rotation 排名。22 个标的 × 20 天 = 440 基础缺失，加上数据缺口和标的状态变化导致总数达到 840。

**Tags:** rotation, signal, data-quality, expected-behavior

## ADR-029: HSAHP 暂时禁用决策

**Status:** Accepted

### Context
HSAHP（AH股溢价指数）数据源不可用：Eastmoney 从当前环境不可达，Tencent 无 K 线数据。当前 enabled: true 但 rows=0，产生 critical 状态告警。

### Decision
将 HSAHP 的 enabled 设置为 false。原因：1) 数据源短期内无法恢复；2) 消除 noise 和 critical 告警；3) HK scope 仍保留 HSCEI 和 HSTECH 两个标的。未来若找到替代数据源可重新启用。

**Tags:** HSAHP, data-source, HK-scope, disabled

## ADR-030: Turnover 存量回填待执行

**Status:** Accepted

### Context
P2 turnover 修复（commit 12b17bb）后，新拉取的腾讯日线包含 turnover，但存量 814 根 bar 仍缺失 turnover。需要通过 ingest-daily 回填。

### Decision
Turnover 存量回填命令为 `cargo run -p quant-cli -- ingest-daily --from 2023-01-01`。当前环境 Docker 未运行，需要用户手动启动 Docker Desktop 后执行。回填后 liquidity_proxy_score 计算将更准确。

**Tags:** turnover, backfill, data-quality, manual-execution

## ADR-031: HSAHP 数据失效根因分析

**Status:** Accepted

### Context
需要确认 HSAHP 数据失效的根本原因，以验证禁用决策是否正确。

### Decision
HSAHP 数据失效有两层原因：1) 当前环境无法访问 Eastmoney API（SSL/TLS 重协商失败）；2) 腾讯不提供 HSAHP 的 K 线数据（HSAHP 是衍生计算指数，非成分股指数）。测试了 hkHSAHP 和 hkHSHP 两种 Tencent symbol，均返回空数据。这验证了 ADR-029 禁用决策的正确性。

**Tags:** HSAHP, data-source, root-cause-analysis, eastmoney, tencent

## ADR-032: LLM 配置从 SQLite+Keyring 迁移到 TOML+Env

**Status:** Accepted

### Context
用户反馈 LLM 配置存储在 SQLite 和 OS Keyring 中不可见，体感差。需要透明、可编辑、可移植的配置方案。

### Decision
采用 TOML 文件 + 环境变量插值方案：1) 配置文件 config/llm.toml（gitignore），api_key 使用 ${VAR} 引用环境变量；2) 加载优先级 CLI > TOML > 默认值；3) 向后兼容：保留旧 CLI 命令，双写 SQLite+TOML；4) API Key 三级回退：TOML → Keyring → SQLite。

**Tags:** llm, config, toml, security, architecture

## ADR-033: LLM 配置从 SQLite+Keyring 迁移到 TOML+Env

**Status:** Accepted

### Context
用户反馈 LLM 配置存储在 SQLite 和 OS Keyring 中不可见，体感差。需要透明、可编辑、可移植的配置方案。ADR-032 ID 冲突修复。

### Decision
采用 TOML 文件 + 环境变量插值方案：1) 配置文件 config/llm.toml（gitignore），api_key 使用 ${VAR} 引用环境变量；2) 加载优先级 CLI > TOML > 默认值；3) 向后兼容：保留旧 CLI 命令，双写 SQLite+TOML；4) API Key 三级回退：TOML → Keyring → SQLite；5) temperature/max_tokens/seed 从 [llm.defaults] 读取并传递给 API。

**Tags:** llm, config, toml, security, architecture

## ADR-034: 前端改进方案：Vue 3 迁移 + 布局优化

**Status:** Accepted

### Context
当前前端为 Plain JS + Vite，1818 行 main.js，innerHTML 全量替换模式。用户要求三项改进：框架迁移、UI/UX 重设计、breadth-ma30 面板。Oracle 评审建议合并 Phase 1 到 Phase 2，避免 throwaway work。

### Decision
1. 采用 Vue 3 + Composition API 进行框架迁移
2. breadth-ma30 作为第一个 Vue 试点组件（非 Plain JS）
3. UI/UX 方向：保持暗色主题，优化布局，简约风格，最大化空间利用
4. CSS 策略：保留全局 CSS 用于布局/主题 token，Vue 组件仅消费 CSS 变量
5. 状态迁移：reactive() 保持当前 state 对象结构

**Tags:** frontend, vue3, migration, ui-ux, layout

## ADR-035: Vue 3 共享状态架构：reactive store + CSS 变量桥接

**Status:** Accepted

### Context
Phase 1 Oracle 复核发现三个关键问题：CSS 变量名不匹配、重复渲染、无状态协调。需要一个共享状态机制让 Plain JS 和 Vue 组件同步。

### Decision
1. 创建 src/store.js 导出 reactive() 对象作为共享状态
2. main.js 在状态变更时调用 sync*ToStore() 函数
3. Vue 组件通过 computed() 从 store 读取
4. styles.css 添加 CSS 变量桥接块映射 Vue 变量名到全局设计 token
5. BreadthPanel.vue 导入 dashboard-utils.js 工具函数而非重复实现

**Tags:** vue3, state-management, progressive-migration, css-bridge

## ADR-036: Vue Store 完整同步架构：10 属性全覆盖

**Status:** Accepted

### Context
Phase 2 Oracle 复核发现 store 只同步了 5 个属性，但 Vue 组件依赖 10+ 个属性。HealthStrip/DateSelector/RefreshProgress/StatusPanel 永远显示空状态。

### Decision
1. store.js 扩展到 10 个响应式属性：snapshot, status, selectedScope, selectedReportDate, availableDates, loading, error, exporting, exportResult, refreshStatus
2. 每个属性有对应的 update*ToStore() 函数
3. main.js 在所有状态变更点调用同步函数
4. BreadthPanel 标准化为从 store 读取（与其他面板一致）
5. App.vue 连接所有组件事件处理器

**Tags:** vue3, store, state-sync, phase2-fix

## ADR-037: Phase 3 布局优化：CSS Grid + 简约风格 + 侧边面板

**Status:** Accepted

### Context
Phase 3 目标是优化布局、统一视觉风格、提升交互体验。需要实现响应式网格、统一间距系统、简化边框阴影、优化排版层级、将信号详情弹窗转为侧边面板。

### Decision
1. App.vue 使用 CSS Grid 实现 3 列/2 列布局，响应式断点 1080px/720px
2. 统一间距系统：所有组件使用 --space-* 变量（4px 基准）
3. 统一排版：使用 --font-size-label/meta/body 变量
4. SignalDetailModal 从居中弹窗转为右侧滑入面板（400px 宽）
5. 添加 Vue 过渡动画：fade（通知/骨架屏）、slide（信号详情面板）
6. 保持暗色主题，简化边框使用 --panel-border 变量

**Tags:** vue3, layout, css-grid, responsive, transitions, phase3

## ADR-038: 滚动锁定统一管理：App.vue 层级 watcher

**Status:** Accepted

### Context
Oracle 复核发现滚动锁定存在两个问题：1) SignalDetailModal 和 main.js 都 toggle 同一个 class，存在竞争；2) v-if 销毁组件时 watcher 无法清理 class。

### Decision
1. 移除 SignalDetailModal 中的 watcher
2. 在 App.vue 添加 watch(selectedSignal) 统一管理 body scroll lock
3. main.js 的旧路径保留作为 plain JS 后备
4. 当 selectedSignal 从 truthy 变为 falsy 时，watcher 正确移除 class

**Tags:** vue3, scroll-lock, lifecycle, phase3-fix

## ADR-039: 事件桥接架构：Vue 组件回调 main.js 数据加载

**Status:** Accepted

### Context
Oracle 复核发现三个关键问题：1) 视觉重复（Plain JS 和 Vue 同时渲染所有面板）；2) 状态分裂（Vue 事件不回调 main.js）；3) Store 同步缺失（syncLoadingToStore 从未调用）。

### Decision
1. 从 main.js commitRender() 移除已迁移面板的渲染（保留 hero、usage guides、recent reports、data health）
2. 在 store.js 添加事件桥接函数（loadDashboard、loadSelectedSnapshot、startRefresh 等）
3. main.js 通过 initEventBridge() 注册实际实现
4. App.vue 导入桥接函数，在事件处理器中调用
5. 添加缺失的 syncLoadingToStore/syncErrorToStore 调用
6. JS 包大小从 200KB 降至 158KB

**Tags:** vue3, event-bridge, state-sync, critical-fix

## ADR-040: 前端布局优化：宽度对齐 + 面板重组

**Status:** Accepted

### Context
用户反馈布局问题：1) 左右两侧空白太多；2) Recent Reports 面板细长导致右侧空白；3) Vue 和 Plain JS 容器宽度不对齐。

### Decision
1. 统一宽度公式：#app 和 #vue-app 都使用 width: min(calc(100% - 4rem), 88rem)
2. Top Rotation 独占一行，限制高度 400px
3. Signals + Backtest 并排显示
4. Recent Reports 只显示最近 3 个，多的用模态框展示
5. Recent Reports 和 Data Health 各自独占一行
6. 移除 main.js 中已迁移面板的事件绑定

**Tags:** vue3, layout, responsive, width-alignment

## ADR-041: Frontend i18n: vue-i18n@11 + Vue migration first

**Status:** Accepted

### Context
User wants Chinese/English language switching in the Tauri desktop app. Current frontend has ~260-280 unique translatable strings across 22 files, with a Vue 3 + plain JS hybrid architecture. 3 feature slices (recent-reports, data-health, usage-guides) and the hero section remain plain JS. No existing i18n infrastructure.

### Decision
1. Use vue-i18n@11 with @intlify/unplugin-vue-i18n (Composition API mode). 2. Default language: Chinese (zh). 3. Language toggle: top-right corner of header. 4. No persistence for now. 5. Backend text remains English (mixed-language UI acceptable). 6. Complete Vue migration BEFORE i18n to avoid dual-i18n patterns. 7. Phase 0: delete ~700 lines dead code, extract hero to DashboardHero.vue, migrate 3 plain JS slices to Vue. 8. Eliminate commitRender() after full Vue migration. 9. Use nested JSON keys by domain (trustSummary.*, dataHealth.*, etc.). 10. Thread locale through dashboard-utils.js formatters.

**Tags:** i18n, vue3, frontend, tauri, migration

## ADR-042: Frontend i18n Phase 1 complete: vue-i18n@11 + all Vue components migrated

**Status:** Accepted

### Context
Phase 1 of i18n implementation completed. All 20 Vue components use useI18n/t(). Locale files (zh.json, en.json) with ~280 keys each. LanguageToggle in top-right corner. Default language Chinese.

### Decision
1. vue-i18n@11 with @intlify/unplugin-vue-i18n. 2. Composition API mode (legacy: false). 3. Default locale: zh. 4. Fallback locale: en. 5. Domain-nested key structure. 6. LanguageToggle component in App.vue header. 7. No persistence yet (Oracle identified as must-fix). 8. Deferred: dashboard-utils.js fallbacks, date/number formatting, backend text.

**Tags:** i18n, vue3, frontend, completed

## ADR-043: Frontend i18n Phase 2 complete: dashboard-utils.js locale-aware formatting

**Status:** Accepted

### Context
Phase 2 of i18n implementation completed. All 11 format functions in dashboard-utils.js now use locale-aware Intl formatters and i18n fallback strings.

### Decision
1. Import i18n instance directly in dashboard-utils.js. 2. Use getLocale() helper for Intl.DateTimeFormat/NumberFormat locale parameter. 3. Use t() helper for fallback strings. 4. Added utils.* and reportTypes.* keys to locale files. 5. All format functions (formatDate, formatDateTime, formatNumber, formatInteger, formatCurrency, formatDeltaPoints, formatCanonicalAdjustment, formatDateRange, formatReportType, formatFallbackState, getErrorMessage) now locale-aware.

**Tags:** i18n, vue3, frontend, completed

## ADR-044: Frontend i18n Phase 3 complete: main.js export messages + dead code cleanup

**Status:** Accepted

### Context
Phase 3 of i18n implementation completed. main.js export messages now use t(). Dead code files removed (features/*.js, renderers/environment-breadth.js). Backend-originated strings (trust.headline/message/notes) documented as requiring Rust backend changes.

### Decision
1. main.js export messages use t('export.*') keys. 2. Added t() helper in main.js using i18n.global.t(). 3. Added export.* and refresh.cancellingAfterStage keys to locale files. 4. Deleted dead code: features/recent-reports.js, features/data-health.js, features/usage-guides.js, renderers/environment-breadth.js. 5. Backend-originated strings (trust.headline, trust.message, trust.notes) require Rust backend changes - documented as limitation.

**Tags:** i18n, vue3, frontend, completed

## ADR-045: i18n key mismatch fixes: 26+ keys corrected across DataHealthPanel and RecentReportsPanel

**Status:** Accepted

### Context
Oracle review found 26+ key mismatches between Vue component code and locale files. These would cause runtime broken translations (raw key names displayed instead of translated text).

### Decision
1. Fixed 22 key mismatches in DataHealthPanel.vue. 2. Fixed 5 key mismatches in RecentReportsPanel.vue. 3. Fixed 3 parameter name mismatches (gaps/jumps/healthStatusMeta). 4. Fixed hardcoded strings in main.js and BreadthPanel.vue. 5. Added dashboardSnapshot key to locale files.

**Tags:** i18n, bugfix, critical

## ADR-046: 前端布局精细调整（2026-05-31）

**Status:** Accepted

### Context
用户在实际使用中发现 Vue 3 迁移后的前端存在若干布局问题：1) `#app` 遗留空 div 导致顶部一整页空白；2) DateSelector 宽度不足，两个 select 未并排；3) TimeContext 与 Regime/Breadth 同列导致高度不均；4) Backtest 与 Signals 并列使信号区域拥挤；5) LanguageToggle 占据 DateSelector 同行空间；6) Hero 右侧 action 区宽度不足，select 选项和按钮文字显示不全。

### Decision
1. 移除 `index.html` 中遗留的空 `#app` div 及其 CSS，消除顶部空白
2. DateSelector 内两个 select 横向 flex 排列，`.header-top` 让 DateSelector 独占整行宽度
3. TimeContext 从 3 列行移出，独占一行（4 个 MetricCard 全宽展示）
4. Regime + Breadth 改为 2 列行
5. BacktestPanel 和 SignalsPanel 拆分为独立行，Signals 内部 buy/sale 保持半宽
6. LanguageToggle 从 `.header-top` 移至 DashboardHero 的 Help/Usage 卡片内，与 Guide viewer pill 并排
7. `.hero__actions` 宽度从 22rem 加宽至 26rem，使 select 选项（5 个汉字）和按钮文字（单行）均可完整显示
8. 刷新按钮文字固定为 "刷新数据"，不再随下拉选项变化

**Tags:** frontend, vue3, layout, ui-ux, responsive

## ADR-047: Signal/Rotation 中文名称显示 + Schema-Evolution 重构（2026-06-01）

**Status:** Accepted

### Context
用户要求：1) 信号栈中买入/防御组的每个模块显示对应中文名称；2) 轮动排行榜因篇幅限制采用悬浮样式显示中文名称。实施过程中引发 schema-evolution 问题：在 `RotationRankSnapshot` / `SignalSnapshot` 上新增 `name` 字段导致旧 ClickHouse JSON 行反序列化崩溃。

### Decision
1. **前端显示**：SignalsPanel 中 `top`/`bullish`/`defensive` 信号卡均在代码旁显示中文名；RotationPanel 中鼠标悬浮在代码单元格上显示中文名 tooltip
2. **架构重构**：将 `name` 从 `RotationRankSnapshot` 和 `SignalSnapshot` 移除，改为 `DashboardSnapshot.symbol_names: HashMap<String, String>`，由 `app-service` 从 universe 配置一次性填充
3. **Schema-evolution 政策**：所有从 ClickHouse JSON 反序列化的 DTO 字段必须携带 `#[serde(default)]`，或在 fetch 函数中手动 remap。已在 `market-store/AGENTS.md` 正式文档化
4. **Markdown 报告同步**：`render_markdown_report` 的信号行现在也输出 `symbol (name)` 格式

**Tags:** frontend, backend, schema-evolution, i18n, ui-ux, oracle-reviewed

## ADR-048: 前端 LLM 智能分析面板集成（LLM Desktop Integration）

**Status:** Accepted

### Context
项目后端已具备完整的 research-skills LLM 引擎（OpenAI provider、Skill Registry、Agent Profile、Reasoning Graph），CLI 已支持 analyze-with-llm 和 analyze --skill 命令，但桌面端前端没有任何 LLM 交互入口。需要在 Dashboard 中增加一键触发 AI 分析、查看结构化结果的完整链路。

### Decision
1. 在 Dashboard Hero 下方的 header-top 区域放置 LlmAnalysisTrigger 组件（Agent 下拉选择 + AI 分析按钮）。2. 点击分析后从右侧滑出 LlmAnalysisPanel（520px，与 SignalDetailModal 同模式），含 4 个 Tab：分析结论 / Regime 研判 / 执行详情 / 风险提示。3. Tauri 层新增 3 个命令：get_llm_status、list_agent_profiles、list_skills；复用已有的 analyze_with_skill。4. Store 扩展 7 个新属性（llmAnalysis、llmLoading、llmError、llmConfig、selectedAgent、availableAgents、showLlmPanel），遵循现有 reactive store 模式。5. 对 PlaceholderProvider 返回的占位数据添加 placeholder: true 标识，前端渲染黄色警告横幅，避免误导操作者。6. 定义类型化 DTO（LlmStatus、AgentProfileSummary、SkillSummary）替代 serde_json::Value，保持前后端契约一致。

**Tags:** frontend, llm, tauri, vue3, architecture, oracle-reviewed

## ADR-049: LLM Desktop Integration Phase 2 — Skill Selector, Real Provider, Config UI, Export

**Status:** Accepted

### Context
Following ADR-048 (Phase 1 MVP), the LLM desktop integration needed enhancement: Skill selection, real OpenAI provider wiring, frontend configuration UI, markdown export, XSS protection, typed DTOs, and event bridge unification. Oracle review (D1-D6) identified path safety, placeholder handling, markdown bugs, XSS, type safety, and dead code issues — all fixed in Phase 2.

### Decision
1. Skill Selector: LlmAnalysisTrigger.vue adds second dropdown populated from list_skills Tauri command; labels use skill.description || skill.name for readability; selectedSkill stored in reactive store.
2. Real OpenAI Provider: analyze_with_skill conditionally uses OpenAiProvider::from_config(&config, &key) when config + API key present; falls back to PlaceholderProvider when missing; placeholder: true flag in response JSON triggers yellow warning banner in UI.
3. LLM Config Frontend UI: LlmAnalysisPanel.vue empty-state shows three-field form (base_url, model, api_key) with Save button; calls set_llm_config + set_llm_api_key Tauri commands; refreshes llmConfig status after save; form pre-populates from existing config via watch(immediate: true); apiKey never pre-filled for security.
4. Markdown Export: LlmAnalysisPanel.vue footer adds "Export Markdown" button; calls export_llm_analysis Tauri command which writes reports/llm-analysis-{scope}-{date}.md and registers via market_store::insert_report_snapshot("LLM_ANALYSIS", ...).
5. XSS Sanitization: renderMarkdown uses three-layer defense — escape raw HTML entities first, then convert markdown syntax to HTML, then strip dangerous tags (script/iframe/object/embed/form/event handlers/javascript:).
6. Typed DTOs: LlmStatus, AgentProfileSummary, SkillSummary defined in core-domain/src/lib.rs; Tauri commands return these strong types instead of serde_json::Value; inline struct definitions removed from src-tauri/src/lib.rs.
7. Event Bridge Unification: analyzeWithLlm added to initEventBridge in store.js/main.js; App.vue imports bridgeAnalyzeWithLlm instead of direct llmApi; consistent with refresh/export patterns.
8. Agent Profile Path Safety: analyze_with_skill Tauri command uses StorageConfig::project_root()?.join("research/agents") instead of relative path, preventing CWD-dependent failures in packaged builds.
9. i18n: 35+ new keys added to zh.json and en.json covering skill/config/export actions.

**Tags:** frontend, llm, tauri, vue3, architecture, oracle-reviewed, phase2

## ADR-050: Research Insight First — Ground Truth before Narrative, Insight before JSON

**Status:** Accepted

### Context
两份复核建议 converged：Doc 1 指出前端暴露内部状态而非研究结论；Doc 2 指出 Ground Truth 验证是 P0。需要同时解决 '系统是否正确' 和 '用户是否看得懂'。

### Decision
1. Ground Truth Validation (Wave 7) 为 P0，利用已存在的 research-validation 基础设施接线历史数据验证。2. Insight Composer (Wave 8) 为 P1，用确定性规则将 DashboardSnapshot 映射为 ResearchInsight（headline/summary/implications/recommendations），重构前端为四层：Insight → Metrics → Evidence → Raw。3. Daily Report Composer (Wave 9) 为 P2，只做聚合不做创作。4. Narrative Layer (Wave 10) 推迟到 GT macro_f1 > 0.65。

**Tags:** v4, insight, ground-truth, ui, roadmap

## ADR-051: From Research Insight to Portfolio Decision

**Status:** Accepted

### Context
Ground Truth validation has been run for 2023-02 to 2026-06 across GLOBAL/CN/HK scopes. Results show regime prediction Macro F1 = 0.18 (GLOBAL/CN) and 0.27 (HK), far below the 0.65 threshold required for downstream trust. Root cause identified: macro-engine regime thresholds are intuitively set (risk_off triggered by trend<40 OR risk<40) causing massive risk_off bias (~53% predictions vs ~0.1% actual).

### Decision
1. PAUSE all downstream work (Insight enhancements, Daily Report expansion, Narrative Layer, Allocation Layer) until regime Macro F1 > 0.65. 2. Next 1-2 sprints focus exclusively on regime calibration: analyze threshold sensitivity, test alternative classification rules, validate against GT, iterate. 3. Allocation Layer (Portfolio Decision) remains the target architecture but is gated by regime accuracy. 4. Only after GT passes will we expand verifiable Skills and build Allocation Layer.

**Tags:** v4, ground-truth, regime-calibration, allocation, portfolio, adr-051

## ADR-052: Regime Classification Is Not Future Return Classification

**Status:** Accepted

### Context
Ground Truth validation revealed extreme class imbalance (neutral 94.8%, risk_on 5.1%, risk_off 0.1%) because the current GT uses 20-day forward returns with ±8% thresholds. This creates a label for 'extreme return events' rather than 'market state'. A correct risk_on regime classification might only produce +3% future return, which would be labeled 'neutral' by GT. Optimizing thresholds to fit this GT would cause the model to overfit to the wrong objective.

### Decision
1. Regime Ground Truth must NOT be constructed purely from future returns. 2. GT must reflect market STATE (trend, breadth, liquidity, volatility posture) not future OUTCOME. 3. Before any threshold calibration, audit the GT label system for class balance, transition coherence, and duration persistence. 4. If GT is flawed, redesign labels using ResearchContext dimensions or external validated regime indicators. 5. Threshold calibration only begins after GT passes audit.

**Tags:** v4, ground-truth, regime, classification, adr-052

## ADR-053: Ground Truth Must Be Independent From Predictor

**Status:** Accepted

### Context
Wave 7 regime audit proved that the Observation Source (macro-engine scores) is the root cause of low F1, not the persistence filter or state machine. Current architecture has GT and Predictor sharing the same Macro Engine intermediate layer, violating the ML principle that Label and Feature must be independent.

### Decision
1. GT and Predictor must use completely independent data paths. GT chain: OHLCV/Volume/Indicators → Market-State Extractor → MarketStateObservation → GT Regime Generator → Ground Truth. Predictor chain: Macro/Dollar/Rates/Sentiment → Research Context → Regime Engine → Predicted Regime. 2. MarketStateObservation must be a semantic observation layer (TrendObservation, LiquidityObservation, VolatilityObservation, BreadthObservation) not raw indicator dump. 3. BreadthObservation must be Option<BreadthObservation> — never fake breadth when underlying data is unavailable. 4. Introduce Candidate State → Persistence Filter two-phase state machine. 5. Persistence Score is a hard gate: avg_episode > 20d, median > 15d, churn < 5%, stability > 0.9. 6. Add Coverage Score: imbalance_ratio < 5 to prevent single-state dominance. 7. Old Wave 7.1-7.4 tasks are superseded; new tasks 7.3B-7.4 replace them.

**Tags:** v4, ground-truth, regime, architecture, gt-predictor-separation

## ADR-055: Regime Classification Must Be Scope-Aware Factor-Dominant

**Status:** Accepted

### Context
TASK-026 Macro Factor Alignment Audit revealed CN and HK have completely different optimal regime factors. CN: Trend is best (Trend-Only alignment=0.527 vs baseline=0.353). HK: Trend is completely broken (F1=0.00), Risk is best (Risk-Only alignment=0.285 vs baseline=0.073). Current production uses identical logic for both markets.

### Decision
Implement scope-aware factor-dominant regime logic: CN uses Trend-Dominant (RiskOff=trend<40, RiskOn=trend>=60), HK uses Risk-Dominant (RiskOff=risk<40, RiskOn=risk>=55). Remove Liquidity from RiskOff trigger as it has negligible DD20 predictive power (F1<0.10 in both markets).

**Tags:** regime, factor-dominant, scope-aware, macro-engine, cn, hk

## ADR-056: Regime Must Separate State Classification From Economic Prediction

**Status:** Accepted

### Context
TASK-025 through TASK-028A established that single-layer regime cannot simultaneously serve state classification and economic prediction. HK shows critical divergence: Risk is alignment-best but Liquidity is economic-best. TASK-027 proves optimizing for Alignment degrades Economic Separation (33.4 to 8.4).

### Decision
Adopt dual-layer architecture: MarketStateRegime (State Layer) for alignment/trend/drawdown detection, and EconomicStateRegime (Economic Layer) for forward return separation. Both layers coexist independently. Freeze ADR-055 and TASK-004 until architecture is accepted.

**Tags:** regime, architecture, dual-layer, state-classification, economic-prediction, adr-056

## ADR-058: Persistence Simplification — confirmation_days = 1

**Status:** Accepted

### Context
TASK-034C (Episode Survival Audit) reveals that raw regime episodes are far shorter than assumed. CN median episode = 2.0 days, HK median = 3.0 days. P95 is only 15.2d (CN) and 25.3d (HK). This means `confirmation_days=10` exceeds the typical state lifetime — it is not filtering noise, it is systematically destroying state classification by swallowing regimes before they can be confirmed.

TASK-034B reveals the mechanics: `apply_persistence` counts streak from 1, not 0, making `days=1` mechanically identical to `days=0`. Any regime with `duration < confirmation_days` is entirely swallowed. At 10d, CN swallows 86% of all episodes (274 days, 51.7% of data), HK swallows 72% (140 days, 27.2%).

TASK-034 shows the economic consequence: Sharpe drops -84% at 10d. But the primary argument for change is **state classification integrity**, not economic performance. A regime system that swallows 86% of episodes cannot correctly describe market states.

### Decision
Change production `confirmation_days` from **10 to 1**.

Rationale:
1. **Episode survival proves 10d is absurd**: CN median=2d, HK median=3d. 10d is 3-5x typical state lifetime.
2. **0d and 1d are mechanically identical** in current implementation (streak starts at 1)
3. **1d is more defensible to team** than 0d: "at least one close confirmation" vs "instant flip"
4. **2d+ proven destructive to state classification**: swallows the majority of episodes
5. **Primary goal is correct state description**, not Sharpe maximization

### Evidence (State Classification Perspective)
- CN: median episode=2.0d, p95=15.2d, 30.1% are 1-day flips
- HK: median episode=3.0d, p95=25.3d, 24.3% are 1-day flips
- At 10d: CN survival rate=14.2%, HK survival rate=28.4%
- At 2d: CN survival rate=69.9%, HK survival rate=75.7%
- 10d swallows 86% (CN) and 72% (HK) of all regime episodes

### Evidence (Economic Perspective — Secondary)
- CN 0-1d: Sharpe=1.36 vs 10d: Sharpe=0.22
- HK 0-1d: Sharpe=1.14 vs 10d: Sharpe=0.18

### Scope
Only affects `confirmation_days` in macro-engine persistence filter. Does NOT change threshold logic, factor weights, or regime classification rules.

**Tags:** regime, persistence, confirmation-days, macro-engine, production-change, state-classification, episode-survival, adr-058

## ADR-059: HK Anchor Symbol Fix — HSI → HSCEI

**Status:** Accepted

### Context
Wave 8 Phase 2 revealed HK Alignment=0.007 at 1d, appearing "broken". Score Distribution Audit discovered HK `trend_score` was CONSTANTLY 50.0 (min=50, max=50, std=0) across all 515 days. Investigation revealed `app-service/src/lib.rs:2122` hardcoded "HSI" as HK anchor, but the database has NO HSI bars. `fetch_daily_bars` returned empty, causing `trend_score` to always default to 50.0 via `unwrap_or(50.0)`.

### Decision
Change HK anchor symbol from **"HSI" to "HSCEI"** in `app-service/src/lib.rs:2122`.

### Impact
All HK Wave 7.5 experiments were run with broken `trend_score`. Re-running Wave 8 with fixed trend_score (computed from actual HSCEI bars) shows:
- HK Alignment: **0.286** (outperforms CN's 0.252)
- HK Sharpe: **1.53**, CAGR: **22.96%**
- HK is NOT broken. The "HK failure" conclusion was entirely a data ingestion bug.

### Consequences
- **ADR-057 (HK Liquidity Dominant) is no longer needed.** HK does not need a separate Liquidity-Dominant regime.
- **All HK Wave 7.5 conclusions must be re-evaluated** in light of the fixed data.
- **Production refresh APPROVED for both CN and HK** with `confirmation_days=1`.

**Tags:** hk, data-ingestion, bug-fix, hsi, hscei, anchor-symbol, adr-059

## ADR-060: Regime Ground Truth Definition

**Status:** Accepted

### Context
TASK-035B (Ground Truth Audit) revealed a fundamental mismatch:
- **Regime predicts**: 45.7% RiskOff days (CN @ 1d)
- **Actual drawdowns >20%**: 0% of days
- **Regime makes money**: Sharpe=1.90 (CN), Sharpe=1.53 (HK)
- **Alignment is low**: 0.252 (CN), 0.286 (HK) — far below 0.75 gate

The current Ground Truth definition uses:
- RiskOff: drawdown > 20% from recent high
- RiskOn: close > MA20 && MA20 > MA60

But the regime is designed to detect **macro factor states** (trend/risk/liquidity scores), not **technical price patterns**.

### Problem
The Alignment metric compares "macro state predictions" against "technical pattern ground truth". This is comparing apples to oranges.

A regime can be economically valuable (high Sharpe) while having low Alignment, because it captures market regimes that are NOT simply "drawdown" or "uptrend".

### Decision
**ACCEPTED.** Launch Wave 9 to formally redefine Ground Truth and Alignment.

The regime's purpose is to classify **macro environment states** that precede distinct forward return distributions, NOT to match technical price patterns.

Ground Truth should reflect what the regime is actually designed to detect:
- **RiskOff**: Forward return distribution skewed negative (e.g., 20-day forward return < -5th percentile)
- **RiskOn**: Forward return distribution skewed positive (e.g., 20-day forward return > 75th percentile)
- **Neutral**: Everything else

This is a **conceptual pivot** from "pattern matching" to "return distribution prediction".

### Wave 9 Deliverables
1. **TASK-060A**: Forward Return Ground Truth Definition
- Define lookback/forward windows
- Define percentile thresholds for RiskOff/RiskOn/Neutral
- Document rationale

2. **TASK-060B**: Forward Return Ground Truth Audit
- Compute new Ground Truth labels for CN/HK historical data
- Compare with current technical-pattern labels
- Measure overlap and divergence

3. **TASK-060C**: Alignment Metric Redesign
- Redesign Alignment to compare regime predictions vs forward-return Ground Truth
- Evaluate new Alignment on CN/HK
- Re-assess 0.75 gate appropriateness

4. **TASK-060D**: Information Score Validation
- Confirm Information score remains valid under new Ground Truth
- Information measures predictive power, independent of Ground Truth definition

### Success Criteria
- New Ground Truth labels show meaningful separation in forward returns
- Regime predictions achieve higher Alignment against new Ground Truth
- Economic metrics (Sharpe, CAGR) remain strong or improve
- Information score stays high (>0.9)

### Impact
- **Alignment Gate**: May need to be recalibrated. If new Alignment is still <0.75 but economic metrics are strong, the gate itself may be the problem.
- **TASK-004**: Remains FROZEN until Wave 9 completes. Threshold calibration depends on valid Ground Truth.
- **ADR-056 Dual-Layer**: Remains valid. State Layer + Economic Layer separation is orthogonal to Ground Truth definition.

**Tags:** regime, ground-truth, alignment, metric-design, state-classification, adr-060, wave-9

## ADR-061: State Layer Semantic Contract

**Status:** Accepted

### Context
Wave 10: State Truth Discovery revealed RiskOff is a lagging indicator (crisis confirmer), not predictive. State Layer should not be evaluated against Forward Return.

### Decision
State Layer is DESCRIPTIVE. Answers 'What is the current market state?' Contract: RiskOn=Trend Follower, Neutral=Consolidation Detector, RiskOff=Crisis Confirmer. Frozen v1.0.

**Tags:** state-layer, semantic-contract, descriptive, frozen

## ADR-062: Three-Layer Evaluation Framework

**Status:** Accepted

### Context
Wave 11: State Layer was being evaluated with wrong metrics (Alignment vs Forward Return). Need separate evaluation for each layer.

### Decision
Three independent evaluation frameworks: State Layer (Coverage, Stability, Persistence, Descriptive Return/Volatility Profile), Economic Layer (Information Gain vs Forward Return), Allocation Layer (Sharpe, CAGR, Max DD). Alignment Gate > 0.75 abandoned for State Layer.

**Tags:** evaluation-framework, three-layer, metrics

## ADR-063: 3-State Economic Taxonomy

**Status:** Accepted

### Context
Economic Layer v2 requires stable taxonomy. After TASK-080A-F (feature inventory, orthogonality, predictive audit, taxonomy discovery, Fed Funds fix), 3-state optimal.

### Decision
Economic Layer uses 3 states: Favorable (37.4%), Neutral (40.3%), Unfavorable (22.4%). Variance ratio 0.843. Fed Funds uses 252d Z-score with ±3 capping. Ready for 90-day Shadow Production.

**Tags:** economic-layer, taxonomy, three-state, shadow-production

## ADR-064: FRED Configuration: TOML + Toggle Switch

**Status:** Accepted

### Context
FRED API key currently hardcoded in app-service/src/lib.rs. Need configurable storage + on/off toggle for macro data fetching.

### Decision
Migrate FRED API key from hardcoded string to TOML config file (config/fred.toml) with environment variable interpolation, and add an enabled/disabled toggle to control macro data fetching.

**Tags:** fred, config, toml, security, macro-data

## ADR-065: State Layer v1.0 Freeze — Shadow Production Entry

**Status:** Accepted

### Context
审计数据：620个交易日 (2024-01-01 ~ 2026-06-16)。DeRisk 50.3% (312天), risk>60 占 41.8%, stress>70 占 33.1%, trend<55 仅占 19.7%。NoTrade(fallback) 10.8% (67天) 为唯一待观察指标。

### Decision
冻结 State Layer v1.0 所有阈值和状态转移逻辑。仅允许实现层 BUG FIX（如 DeRisk 回测映射、数据源错误）。禁止任何行为优化（调阈值、加保护条款、state_score 分类）。进入 90 天 Shadow Production 观察期。

**Tags:** state-layer, shadow-production, freeze, v1.0, audit, task-090a

## ADR-066: Research Surface Governance Model — Production vs Research Surface Separation

**Status:** Accepted

### Context
Shadow Production phase requires minimal-variable observation. Risk identified: 'UI change trap' where display changes silently alter LLM input behavior via research-context builder.

### Decision
Establish two distinct surfaces. Production Surface (frozen): dashboard_snapshot, daily_report, research_context, trust_summary. Research Surface (open): rotation-ranking, state-audit, signal-divergence-audit, risk-breakdown, factor-inspection. New Research Surface tools must NOT enter DashboardSnapshot, ResearchContext, or Markdown Daily Report. First approved tool: rotation-ranking CLI.

**Tags:** shadow-production, governance, research-surface, adr-065, llm-context, oracle-reviewed

## ADR-067: Explainability Layer Governance Boundary — No New Scores, Only Explanations

**Status:** Accepted

### Context
Audit of TASK-092 confirmed Explainability Layer prevents future optimization traps by replacing 'guess → change' with 'observe → understand'. Critical constraint: Explainability Layer must NEVER generate new composite scores, confidence metrics, or decision signals.

### Decision
Explainability Layer is allowed to: expose existing scores, display attribution breakdowns, show strategy composition, reveal state context. Explainability Layer is FORBIDDEN to: generate new scores, create confidence metrics, rank explanations, modify thresholds. Future Divergence Sample Library will track StrongBuy+DE_RISK patterns using only existing scores.

**Tags:** explainability, governance, shadow-production, adr-065, divergence-sample

## ADR-068: Research Context + Reporting Layer v1

**Status:** Accepted

### Context
CLI/GPT/Desktop/future API-PDF-Email consumers depended on different data sources, causing repeated semantic interpretation and Presentation-Business coupling. Production Surface was being polluted.

### Decision
Establish unified ResearchContext as canonical semantic model and Reporting Layer pipeline. ResearchContext is consumer-neutral and evolves conservatively; ReportDocument/Section/Formatter form Presentation Contract; ReportInput carries document payload only; ResearchDataset stays inside app-service; reusable research computation lives in core-domain::research; Production Surface remains frozen. V6 Reporting Platform is frozen; future Consumer Expansion builds on it.

**Tags:** reporting, research-context, architecture, canonical-model, consumer-neutrality, v6

## ADR-069: Reporting Layer Architecture Invariants

**Status:** Accepted

### Context
ADR-068 established the layered model, but the non-negotiable boundary rules needed to be explicit to prevent future PRs from accidentally violating layer separation.

### Decision
Adopt 10 Architecture Invariants covering: ResearchContext consumer-neutrality, ResearchDataset boundary, ReportInput payload-only, Formatter no-domain-computation, core-domain no report-builder dependency, ReportingSnapshot as sole metadata carrier, CLI no raw SQL for research data, concrete ReportInput in builders, domain helpers in core-domain::research, and Production Surface frozen. Full rules in docs/architecture-invariants.md; violations default to do-not-merge.

**Tags:** reporting, architecture, invariants, v6

## ADR-070: Market Evolution Semantic Layer

**Status:** Accepted

### Context
V6 has completed the Observation Layer (Environment, Signal, Stretch, Rotation). The system can describe what the market is doing, but lacks semantic capabilities to answer where the market is evolving. After convergence discussions, V7 should not be a feature list of new reports, but an extension of shared Market Evolution semantics.

### Decision
Adopt a four-layer Market Evolution Semantic Layer for V7: (1) Observation Layer remains frozen from V6; (2) Market Evolution Layer adds Confirmation and Recovery; (3) Historical Evidence Layer adds Market Fingerprint Engine, Historical Analogues, and Outcome Profile; (4) Research Synthesis Layer adds Consensus. Transition is not an independent ResearchContext field; it belongs inside ConsensusSummary. Rotation does not become a standalone module; instead RotationSummary is upgraded with leadership_transition, rotation_acceleration, and theme_dispersion. Historical Analogues must not expose raw similarity percentages to users; use rank or qualitative levels. Any new semantic capability added to ResearchContext must be a cross-consumer shared independent market concept, not a report-specific field.

**Tags:** v7, research-layer, market-evolution, semantic-layer, research-context, reporting

## ADR-071: Market Fingerprint as Canonical Historical Feature Representation

**Status:** Accepted

### Context
ADR-070 established the four-layer Market Evolution Semantic Layer. V7.2 was originally planned as a single phase implementing Historical Analogues (KNN). During detailed design, the user and agent concluded that the MarketFingerprint contract should be frozen before similarity algorithms are introduced, to avoid premature algorithmic coupling in the semantic model.

### Decision
MarketFingerprint is the canonical historical feature representation derived from ResearchContext. Similarity algorithms (Cosine, Euclidean, Mahalanobis, DTW, etc.), normalization strategies, and ranking/OutcomeProfile consumers are all consumers of MarketFingerprint, not part of its definition. V7.2 is therefore split into V7.2A (Market Fingerprint Foundation: MarketFingerprint + MarketFingerprintBuilder contract) and V7.2B (Similarity Engine + OutcomeProfile + CLI). No CLI command is exposed in V7.2A. Future similarity algorithm changes must not modify MarketFingerprint without a new ADR.

**Tags:** v7, market-fingerprint, historical-evidence, canonical-representation, adr-070

## ADR-072: V7.2B Evidence Retrieval Engine Design

**Status:** Accepted

### Context
V7.2A established MarketFingerprint as a canonical historical feature representation. V7.2B introduces similarity search, normalization, distance metrics, and outcome profiling. Because this is the only algorithmic part of V7, its boundaries must be frozen before implementation to avoid coupling algorithms into the semantic layer or letting the engine drift into prediction/interpretation.

### Decision
V7.2B is defined as an Evidence Retrieval Engine, not a prediction engine. The architecture is: ResearchContext → MarketFingerprintBuilder → MarketFingerprint → Normalizer → DistanceMetric (trait, replaceable) → SimilarityMatcher → HistoricalMatch → OutcomeProfiler → SearchResult. Key boundaries: (1) Fingerprint Engine does not predict; (2) Matcher does not interpret; (3) DistanceMetric is a trait so Cosine/Euclidean/WeightedCosine/Mahalanobis can be swapped without changing Fingerprint; (4) Normalizer is independent from DistanceMetric; (5) Similarity is exposed as rank or levels (Very High/High/Moderate/Weak), not raw distance percentages; (6) OutcomeProfile is an independent object; (7) SearchResult includes metadata (searched_days, filtered_days, average_distance) plus matches. find_cluster() API is frozen but may be unimplemented in V7.2B.

**Tags:** v7, market-fingerprint, evidence-retrieval, similarity-engine, adr-070, adr-071

## ADR-076: V7.3.1 Consensus Stabilization (Hotfix)

**Status:** Accepted

### Context
Oracle Review (V7.3) identified maintainability gaps: no version on ConsensusSummary, hard-coded weights/thresholds, dead branch, missing integration test. User accepted these as P0 fixes, rejecting CosineDistance decoupling as premature.

### Decision
Make Consensus configurable via ConsensusConfig, version ConsensusSummary, extract Calibration Baseline Version constant, remove dead branch in classify_bias, and add an app-service integration test for the Consensus vertical slice.

**Tags:** v7, consensus, research-synthesis, v7-3-1, hotfix, oracle-reviewed

## ADR-077: Research Platform 1.0 Freeze

**Status:** Accepted

### Context
V7 has reached a watershed. All four Research layers are implemented and stabilized. The platform crosses from active architecture construction into stable semantic contract.

### Decision
Officially freeze Research Platform 1.0: Observation (V6), Market Evolution (V7.1), Historical Evidence (V7.2), Research Synthesis (V7.3). Future additions are Research Content Evolution feeding into existing frozen layers; any Semantic Architecture change requires a new ADR and explicit un-freeze approval.

**Tags:** v7, research-platform, freeze, semantic-architecture

## ADR-078: Research Attribution Layer (Failure Attribution / Regime Attribution)

**Status:** Accepted

### Context
Historical Replay (2026-07-09) revealed SRD-strong forward returns are Regime-dependent: 2023 Q2 / 2024 H1 negative, 2025 H2 84.6% positive. This is a Model Bias / Systematic Bias, not a random bug. Shadow Production needs to move from observation to explanation.

### Decision
Propose an additive Research Attribution Layer on top of Research Platform 1.0 to explain why the same signal/condition performs differently across market regimes. It covers Macro, Breadth, Liquidity, Theme, Crowding, and Volatility attribution. It is read-only and does not modify frozen State/Signal/Execution layers.

**Tags:** v7, research-attribution, failure-attribution, regime-attribution, shadow-production, adr-077, task-093

## ADR-079: Research Snapshot

**Status:** Accepted

### Context
V8 needs a durable snapshot format that captures a research computation context (Observation/Evolution/Evidence/Synthesis) without embedding Evidence data. Snapshots must be reproducible and reference Evidence by identity.

### Decision
Introduce ResearchSnapshot as a Research Asset with a unique RA-XXXXXX id, AssetKind::Snapshot, carrying dataset_hash and config_hash. It references Evidence via EvidenceRef { id, version } rather than embedding. The snapshot lives in workspace/snapshots/ and is indexed in workspace/registry/snapshot-index.json.

**Tags:** v8, research-snapshot, research-asset, evidence-ref, workspace, reproducibility

## ADR-080: Research Asset Lifecycle

**Status:** Accepted

### Context
V8 introduces multiple Research Asset types (Evidence, Snapshot, and future Knowledge/Validation/Hypothesis). Each needs a consistent, auditable lifecycle so consumers know whether an asset is draft, verified, published, superseded, or archived.

### Decision
All Research Assets share a unified lifecycle: Draft -> Verified -> Published -> Superseded -> Archived. Transitions are managed by WorkspaceManager. Draft assets are produced by computation/replay; Verified requires hash/dependency audit; Published is stable for downstream consumption; Superseded means replaced by a newer version; Archived means expired but retained for history.

**Tags:** v8, research-asset, lifecycle, workspace, draft, verified, published, superseded, archived

## ADR-081: Research Asset Identity

**Status:** Accepted

### Context
V8 produces durable assets of different kinds (Evidence, Snapshot, etc.). They need a single, uniform identity scheme so references, indexes, and lifecycles are consistent across the workspace.

### Decision
All Research Assets use a single identity format RA-XXXXXX (6 uppercase alphanumeric characters). Asset type is encoded in metadata via AssetKind (Evidence, Snapshot, etc.). Existing EV- and SN- prefixed assets are grandfathered but new assets no longer use those prefixes.

**Tags:** v8, research-asset, identity, registry, ra-xxxxxx, asset-kind

## ADR-082: Execution Platform Architecture

**Status:** Accepted

### Context
V5 Execution Layer remains a rule-based Pattern Library that does not consume the V6/V7 Research Context. Execution decisions are made via short-circuit if-else rules, StrategyState is used as a hard gate, and the layer is isolated from the Research Platform. A new Execution Platform is needed as a V8 downstream consumer that unifies with the existing Research Pipeline philosophy.

### Decision
Introduce an Execution Platform as a V8 downstream consumer with a layered pipeline: Quote → Feature → Observation → Evidence → Assessment → Decision → Replay. Execution consumes ResearchContext via an ExecutionMarketView projection, not by owning it. StrategyState becomes a weighted evidence contributor rather than a hard gate. LLM participates only in the Explanation layer after Decision, never in Decision itself. The platform outputs verifiable facts, not investment advice.

**Tags:** v8, execution-platform, architecture-freeze, pipeline, evidence, llm-boundary

## ADR-083: Execution Evidence Model

**Status:** Accepted

### Context
Execution Platform needs a unified semantic language to reason about intraday market behavior, research context, and strategy state. The system currently uses different terms across layers (Observation, Reason, Insight) which fragments the consumer interface.

### Decision
Use Evidence as the unified unit across Research, Execution, and Review layers. An Evidence carries kind, confidence, direction, source, and a typed payload (not serde_json::Value). Intraday Observations are converted into Evidence before assessment. EvidenceKind is semantic (e.g., TrendParticipation, MomentumExpansion, DistributionRisk). EvidencePayload is a typed enum capturing structured metadata per kind. Formatters derive human-readable text from Evidence kind + payload, not from a pre-computed reason string.

**Tags:** v8, execution-platform, evidence, semantic-model

## ADR-084: LLM Boundary in Execution Platform

**Status:** Accepted

### Context
As LLM capabilities are integrated into the desktop and reporting surfaces, there is a risk that LLM starts to be treated as a decision maker or signal generator. The boundary between deterministic execution and LLM-assisted explanation must be explicitly defined.

### Decision
Within the Execution Platform, LLM responsibilities are strictly limited to: Explain, Summarize, Compare, Highlight, and Recommend Reading. LLM MUST NOT perform signal generation, strategy decision making, risk evaluation, or execution state determination. All execution states must originate from ExecutionDecision. LLM consumes ExecutionExplanation produced by report-engine, not raw engine internals. This boundary applies across Research, Reporting, and Execution consumers.

**Tags:** v8, execution-platform, llm, boundary

## ADR-090: V8 Execution Platform Phase 1 Complete: Enter Research Calibration

**Status:** Accepted

### Context
V8 Execution Platform validation results: Golden Suite 80% pass, 9,083 candidate discovery records showing 98.62% Wait, 1.38% BuyNow, 0% Reduce. Root cause is State/Prior evidence overweight, not threshold.

### Decision
Phase 1 (Architecture/Engineering) is complete. Enter Research Calibration phase. 2A: Execution Analytics, 2B: Research Asset Accumulation, 2C: Calibration, 2D: Bayesian Assessment, 2E: ML Ranking. Immediate priorities: wire real market_regime_label, add Execution Analytics, expand Golden Suite Reduce cases, accumulate ≥100-300 records, defer HK repair.

**Tags:** v8, execution, calibration, research-asset, analytics

## ADR-091: V8 Execution Platform: Architecture Frozen, Enter Research Calibration Phase 2A (with Exit Criteria)

**Status:** Accepted

### Context
Refinement of ADR-090. V8 Execution Platform validation results show bottleneck shifted from Architecture to Data. Architecture is frozen. Need a Stage Transition ADR with Entry/Exit Criteria.

### Decision
V8 Execution Platform Architecture is frozen. Future work shifts from architecture optimization to research calibration unless objective evidence indicates architectural deficiencies. Phase 2A: (1) wire real market_regime_label and freeze ExecutionEvent, (2) implement Execution Statistics module, (3) incremental validation 100 -> 1000 -> 9000+, (4) expand Golden Suite only after Evidence Frequency analysis, (5) defer HK repair until CN Calibration done. Entry Criteria (met): Execution Pipeline closed-loop, Replay closed-loop, ExecutionEvent v2.1 frozen, DTOs frozen, Validation CLI complete, Golden Suite established, Discovery Dataset > 9000 records, V5 intact. Exit Criteria (to end Phase 2A): Research Asset >= 300, Execution Statistics Report delivered for 3 rounds, First Calibration Proposal formed, Evidence Frequency Baseline established.

**Tags:** v8, execution, stage-transition, calibration, research-asset, statistics

## ADR-092: Phase 2A Execution Plan: Exit Criteria and Incremental Statistics

**Status:** Accepted

### Context
Refinement of ADR-091 (Accepted). V8 Execution Platform has entered Phase 2A. User feedback clarifies: Phase 2A is a stage-transition ADR, not a general architecture decision, and requires explicit Exit Criteria and a 6-step sub-phase plan.

### Decision
Phase 2A is split into 6 sub-steps: 2A-1 wire real market_regime_label and freeze ExecutionEvent; 2A-2 build Execution Statistics module (not 'Analytics'); 2A-3 run 100-record validation; 2A-4 run 1,000-record validation; 2A-5 run 9,000+ full validation; 2A-6 produce Calibration Proposal. Golden Suite expansion should wait until Evidence Frequency analysis shows why Reduce decisions are absent.

**Tags:** v8, execution, phase-2a, calibration, statistics, exit-criteria

## ADR-093: Execution Statistics Contract Freeze (Phase 2A-2)

**Status:** Accepted

### Context
V8 Execution Platform Phase 2A-2. Architecture Gate added before implementation.

### Decision
Execution Statistics contract is frozen to six outputs: EvidenceFrequency, EvidencePairMatrix, DecisionDistribution, PriorDistribution, AssessmentHistograms, OutcomeMatrix. Output is ExecutionStatistics domain object; Formatter handles JSON/Markdown. Sample strategy is Representative -> Expanded -> Full (no hardcoded numbers). No correlation, feature importance, SHAP, ML, or calibration conclusions in Phase 2A-2.

**Tags:** v8, execution, statistics, phase-2a, contract-freeze

## ADR-094: Phase 2A-3/4: Evidence Trace and Root Cause Review Before Calibration

**Status:** Accepted

### Context
After completing 2A-2 Execution Statistics, full CN dataset shows Reduce=0.00% and RiskExpansion=0.74%. User correctly argues we cannot yet conclude ObservationEngine is too conservative; we need an Evidence Trace/Funnel to determine where each EvidenceKind dies (Observation → Evidence → Assessment → Decision).

### Decision
Add Phase 2A-3 Evidence Trace Analysis and 2A-4 Root Cause Review before any Calibration. Implement an EvidenceTrace/EvidenceFunnel module that counts, per EvidenceKind, how many observations survive each pipeline stage. Do NOT modify ObservationEngine, EvidenceBuilder, Assessment, or Decision until the funnel identifies the failing layer.

**Tags:** v8, execution, evidence-trace, phase-2a, root-cause, calibration

## ADR-095: Phase 2A-4 Decision Path Review: Distribution Coverage + Decision Margin

**Status:** Accepted

### Context
After 2A-3 Evidence Trace found that Reduce=0 is caused by two paths: Distribution Observation=0 and RiskExpansion reaching Assessment but not Reduce. User insists on not modifying code yet; instead perform two focused reviews before any Calibration.

### Decision
Phase 2A-4 is renamed to Decision Path Review with two sub-reviews: 2A-4A Distribution Coverage Review (analyze feature percentiles and which days should trigger Distribution observation) and 2A-4B Decision Margin Review (analyze dominant_direction histogram and Assessment→Decision mapping for RiskExpansion records). No code modification until both reviews complete.

**Tags:** v8, execution, decision-path-review, distribution-coverage, decision-margin, calibration

## ADR-096: Decision Gate Analysis: Why Bearish Assessments Do Not Become Reduce

**Status:** Accepted

### Context
2A-4 Decision Margin Review found that 152 records have dominant_direction < -0.3 but all result in Wait. User wants to identify the exact gate in DecisionEngine that blocks Reduce: risk, confidence, or consensus. volume_ma20 fix is deferred.

### Decision
Phase 2A-4.5 (or 2A-4B) is Decision Gate Analysis. It will enumerate every Reduce candidate (dominant_direction < reduce_threshold) and report which DecisionEngine gate blocks it: RiskCritical, RiskHigh, ConfidenceTooLow, or ConsensusTooLow. Per-record detail included. volume_ma20 remains unfixed for now.

**Tags:** v8, execution, decision-gate, confidence, consensus, risk, reduce

## ADR-097: Risk Semantics Review: Entry Risk vs Holding Risk

**Status:** Accepted

### Context
Decision Gate Analysis found 54 RiskHigh records block Reduce and 98 ConfidenceTooLow records block Reduce. User wants to understand if RiskLevel::High is a Domain Modeling issue: it currently means 'do not trade' but in bearish contexts it should mean 'exit position'. Need to analyze evidence composition, decision context, and future outcomes of RiskHigh records.

### Decision
Phase 2A-4C is Risk Semantics Review. It will analyze RiskLevel::High records to determine: (1) which evidences compose High risk, (2) the distribution of direction/confidence/consensus for High risk records, (3) the forward outcome of RiskHigh+Wait records to validate whether waiting was harmful, and (4) a proposed semantic mapping of EntryRisk vs HoldingRisk. No code changes to RiskLevel or DecisionEngine.

**Tags:** v8, execution, risk-semantics, domain-modeling, entry-risk, holding-risk, reduce
