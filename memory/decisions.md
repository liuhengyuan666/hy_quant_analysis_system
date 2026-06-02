## ADR-026: Memory 体系清理与状态同步

**Status:** active

### Context
当前 Memory 体系存在三个问题：1) glossary 术语不完整（19个）；2) decisions 状态需要确认；3) archive 目录结构需要初始化。

### Decision
执行 Memory 体系清理：补充 12 个缺失术语（available_dates_ms、pipeline_diagnostics、refresh_pipeline、dashboard_bundle 等），确认 25 条决策状态，初始化 archive 目录结构。

**Tags:** memory, maintenance, documentation

## ADR-027: ClickHouse 日期查询性能优化（Oracle 复核修正）

**Status:** active

### Context
Dashboard 加载性能瓶颈：`available_dates_ms` 耗时 24 秒。根因是 `fetch_dashboard_available_dates` 查询使用 IN 子句导致双表全扫描。

### Decision
实施两层优化：1) 重写主查询使用 JOIN 替代 IN 子句，避免双表全扫描；2) 在 AppContext 中添加 AvailableDatesCache 内存缓存（TTL 5分钟），数据刷新后自动清除。Oracle 复核后移除了 90 天限制，因为它会破坏历史日期查询（dashboard-snapshot --date 和 export-report --date）。

**Tags:** performance, clickhouse, caching, dashboard, oracle-reviewed

## ADR-028: rotation_missing 根因分析：历史窗口不足导致的预期行为

**Status:** active

### Context
signal-engine 中 840 个 rotation_missing 条目需要排查原因。

### Decision
rotation_missing 是预期行为，不是 bug。根因是 rotation-engine 在计算 rs_20 时需要至少 20 天历史数据（index >= 20），导致每个标的的前 20 天无法生成 rotation 排名。22 个标的 × 20 天 = 440 基础缺失，加上数据缺口和标的状态变化导致总数达到 840。

**Tags:** rotation, signal, data-quality, expected-behavior

## ADR-029: HSAHP 暂时禁用决策

**Status:** active

### Context
HSAHP（AH股溢价指数）数据源不可用：Eastmoney 从当前环境不可达，Tencent 无 K 线数据。当前 enabled: true 但 rows=0，产生 critical 状态告警。

### Decision
将 HSAHP 的 enabled 设置为 false。原因：1) 数据源短期内无法恢复；2) 消除 noise 和 critical 告警；3) HK scope 仍保留 HSCEI 和 HSTECH 两个标的。未来若找到替代数据源可重新启用。

**Tags:** HSAHP, data-source, HK-scope, disabled

## ADR-030: Turnover 存量回填待执行

**Status:** active

### Context
P2 turnover 修复（commit 12b17bb）后，新拉取的腾讯日线包含 turnover，但存量 814 根 bar 仍缺失 turnover。需要通过 ingest-daily 回填。

### Decision
Turnover 存量回填命令为 `cargo run -p quant-cli -- ingest-daily --from 2023-01-01`。当前环境 Docker 未运行，需要用户手动启动 Docker Desktop 后执行。回填后 liquidity_proxy_score 计算将更准确。

**Tags:** turnover, backfill, data-quality, manual-execution

## ADR-031: HSAHP 数据失效根因分析

**Status:** active

### Context
需要确认 HSAHP 数据失效的根本原因，以验证禁用决策是否正确。

### Decision
HSAHP 数据失效有两层原因：1) 当前环境无法访问 Eastmoney API（SSL/TLS 重协商失败）；2) 腾讯不提供 HSAHP 的 K 线数据（HSAHP 是衍生计算指数，非成分股指数）。测试了 hkHSAHP 和 hkHSHP 两种 Tencent symbol，均返回空数据。这验证了 ADR-029 禁用决策的正确性。

**Tags:** HSAHP, data-source, root-cause-analysis, eastmoney, tencent

## ADR-032: LLM 配置从 SQLite+Keyring 迁移到 TOML+Env

**Status:** active

### Context
用户反馈 LLM 配置存储在 SQLite 和 OS Keyring 中不可见，体感差。需要透明、可编辑、可移植的配置方案。

### Decision
采用 TOML 文件 + 环境变量插值方案：1) 配置文件 config/llm.toml（gitignore），api_key 使用 ${VAR} 引用环境变量；2) 加载优先级 CLI > TOML > 默认值；3) 向后兼容：保留旧 CLI 命令，双写 SQLite+TOML；4) API Key 三级回退：TOML → Keyring → SQLite。

**Tags:** llm, config, toml, security, architecture

## ADR-033: LLM 配置从 SQLite+Keyring 迁移到 TOML+Env

**Status:** active

### Context
用户反馈 LLM 配置存储在 SQLite 和 OS Keyring 中不可见，体感差。需要透明、可编辑、可移植的配置方案。ADR-032 ID 冲突修复。

### Decision
采用 TOML 文件 + 环境变量插值方案：1) 配置文件 config/llm.toml（gitignore），api_key 使用 ${VAR} 引用环境变量；2) 加载优先级 CLI > TOML > 默认值；3) 向后兼容：保留旧 CLI 命令，双写 SQLite+TOML；4) API Key 三级回退：TOML → Keyring → SQLite；5) temperature/max_tokens/seed 从 [llm.defaults] 读取并传递给 API。

**Tags:** llm, config, toml, security, architecture

## ADR-034: 前端改进方案：Vue 3 迁移 + 布局优化

**Status:** active

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

**Status:** active

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

**Status:** active

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

**Status:** active

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

**Status:** active

### Context
Oracle 复核发现滚动锁定存在两个问题：1) SignalDetailModal 和 main.js 都 toggle 同一个 class，存在竞争；2) v-if 销毁组件时 watcher 无法清理 class。

### Decision
1. 移除 SignalDetailModal 中的 watcher
2. 在 App.vue 添加 watch(selectedSignal) 统一管理 body scroll lock
3. main.js 的旧路径保留作为 plain JS 后备
4. 当 selectedSignal 从 truthy 变为 falsy 时，watcher 正确移除 class

**Tags:** vue3, scroll-lock, lifecycle, phase3-fix

## ADR-039: 事件桥接架构：Vue 组件回调 main.js 数据加载

**Status:** active

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

**Status:** active

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

**Status:** active

### Context
User wants Chinese/English language switching in the Tauri desktop app. Current frontend has ~260-280 unique translatable strings across 22 files, with a Vue 3 + plain JS hybrid architecture. 3 feature slices (recent-reports, data-health, usage-guides) and the hero section remain plain JS. No existing i18n infrastructure.

### Decision
1. Use vue-i18n@11 with @intlify/unplugin-vue-i18n (Composition API mode). 2. Default language: Chinese (zh). 3. Language toggle: top-right corner of header. 4. No persistence for now. 5. Backend text remains English (mixed-language UI acceptable). 6. Complete Vue migration BEFORE i18n to avoid dual-i18n patterns. 7. Phase 0: delete ~700 lines dead code, extract hero to DashboardHero.vue, migrate 3 plain JS slices to Vue. 8. Eliminate commitRender() after full Vue migration. 9. Use nested JSON keys by domain (trustSummary.*, dataHealth.*, etc.). 10. Thread locale through dashboard-utils.js formatters.

**Tags:** i18n, vue3, frontend, tauri, migration

## ADR-042: Frontend i18n Phase 1 complete: vue-i18n@11 + all Vue components migrated

**Status:** active

### Context
Phase 1 of i18n implementation completed. All 20 Vue components use useI18n/t(). Locale files (zh.json, en.json) with ~280 keys each. LanguageToggle in top-right corner. Default language Chinese.

### Decision
1. vue-i18n@11 with @intlify/unplugin-vue-i18n. 2. Composition API mode (legacy: false). 3. Default locale: zh. 4. Fallback locale: en. 5. Domain-nested key structure. 6. LanguageToggle component in App.vue header. 7. No persistence yet (Oracle identified as must-fix). 8. Deferred: dashboard-utils.js fallbacks, date/number formatting, backend text.

**Tags:** i18n, vue3, frontend, completed

## ADR-043: Frontend i18n Phase 2 complete: dashboard-utils.js locale-aware formatting

**Status:** active

### Context
Phase 2 of i18n implementation completed. All 11 format functions in dashboard-utils.js now use locale-aware Intl formatters and i18n fallback strings.

### Decision
1. Import i18n instance directly in dashboard-utils.js. 2. Use getLocale() helper for Intl.DateTimeFormat/NumberFormat locale parameter. 3. Use t() helper for fallback strings. 4. Added utils.* and reportTypes.* keys to locale files. 5. All format functions (formatDate, formatDateTime, formatNumber, formatInteger, formatCurrency, formatDeltaPoints, formatCanonicalAdjustment, formatDateRange, formatReportType, formatFallbackState, getErrorMessage) now locale-aware.

**Tags:** i18n, vue3, frontend, completed

## ADR-044: Frontend i18n Phase 3 complete: main.js export messages + dead code cleanup

**Status:** active

### Context
Phase 3 of i18n implementation completed. main.js export messages now use t(). Dead code files removed (features/*.js, renderers/environment-breadth.js). Backend-originated strings (trust.headline/message/notes) documented as requiring Rust backend changes.

### Decision
1. main.js export messages use t('export.*') keys. 2. Added t() helper in main.js using i18n.global.t(). 3. Added export.* and refresh.cancellingAfterStage keys to locale files. 4. Deleted dead code: features/recent-reports.js, features/data-health.js, features/usage-guides.js, renderers/environment-breadth.js. 5. Backend-originated strings (trust.headline, trust.message, trust.notes) require Rust backend changes - documented as limitation.

**Tags:** i18n, vue3, frontend, completed

## ADR-045: i18n key mismatch fixes: 26+ keys corrected across DataHealthPanel and RecentReportsPanel

**Status:** active

### Context
Oracle review found 26+ key mismatches between Vue component code and locale files. These would cause runtime broken translations (raw key names displayed instead of translated text).

### Decision
1. Fixed 22 key mismatches in DataHealthPanel.vue. 2. Fixed 5 key mismatches in RecentReportsPanel.vue. 3. Fixed 3 parameter name mismatches (gaps/jumps/healthStatusMeta). 4. Fixed hardcoded strings in main.js and BreadthPanel.vue. 5. Added dashboardSnapshot key to locale files.

**Tags:** i18n, bugfix, critical

## ADR-046: 前端布局精细调整（2026-05-31）

**Status:** active

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

**Status:** active

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
