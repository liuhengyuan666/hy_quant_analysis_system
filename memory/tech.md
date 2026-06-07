# Technical Memory

## 技术栈

- Rust workspace：核心业务、计算与存储边界。
- Tauri：桌面端宿主。
- Vite + Vue 3 + Composition API：桌面前端（vue-i18n@11 国际化）。
- ClickHouse：分析型时序数据存储。
- SQLite：本地轻状态。

## 当前架构原则

- `app-service` 负责编排。
- 各 engine crate 负责计算逻辑。
- `market-store` 是数据库访问边界。
- CLI 与桌面桥接层保持薄封装，主要调用 `app-service`。

## 数据与语义约束

- 日线口径统一为前复权：Eastmoney `fqt=1`，Tencent `qfq`。
- Dashboard / report 使用持久化快照与 as-of 日期语义。
- `GLOBAL / CN / HK` 已是显式 scope 语义。
- `TrustSummary.data_health` 已改为 `Option<DataHealthSummary>`（2026-05-20 Dashboard 性能优化），热路径不再阻塞于外部 HTTP 请求。
- `core-domain` 新增 `TradingCalendar` 模块（`src/calendar.rs`），基于 `config/calendars/*.json` 提供 CN/HK 休市日判断。

## 当前工程现实

- 无正式 CI workflow。
- 主要验证手段为 `cargo check`、局部 `cargo test` 与实际 CLI 流程。
- `target/`、`reports/`、前端 `node_modules/`、前端 `dist/` 属于生成或运行时产物，不是源码事实来源。
- 桌面前端已完成 Vue 3 迁移，使用 Vite + Vue 3 + Composition API + vue-i18n@11。Plain JS（main.js）与 Vue 组件共存，通过 reactive store 桥接状态。
- `apps/desktop/frontend/src/main.js` 保留根状态、命令调度与全局渲染；纯工具函数位于 `apps/desktop/frontend/src/lib/dashboard-utils.js`（已全部 locale-aware）。
- 前端国际化使用 vue-i18n@11，默认中文，支持中英切换。i18n 基础设施位于 `i18n.js`，语言文件位于 `locales/zh.json` 和 `locales/en.json`。
- `LanguageToggle.vue` 提供右上角语言切换按钮。
- 20+ Vue 组件已迁移完成，覆盖所有 dashboard 面板（trust、signals、backtest、rotation、breadth、environment、data-health、recent-reports、usage-guides 等）。
- `features/*.js` 与 `renderers/*.js` 已作为 dead code 删除（Phase 0），功能全部迁移至 Vue 组件。
