# Technical Memory

## 技术栈

- Rust workspace：核心业务、计算与存储边界。
- Tauri：桌面端宿主。
- Plain JS + Vite：桌面前端。
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

## 当前工程现实

- 无正式 CI workflow。
- 主要验证手段为 `cargo check`、局部 `cargo test` 与实际 CLI 流程。
- `target/`、`reports/`、前端 `node_modules/`、前端 `dist/` 属于生成或运行时产物，不是源码事实来源。
- 桌面前端当前是 `Vite + Plain JS`，且已移除直接 `vue` 依赖。
- `apps/desktop/frontend/src/main.js` 已开始按“先纯工具层、后状态与视图层”的顺序渐进拆分；首批纯函数现位于 `apps/desktop/frontend/src/lib/dashboard-utils.js`。
- guide viewer 相关状态流、渲染和事件绑定现已集中在 `apps/desktop/frontend/src/features/usage-guides.js`。
- data-health 相关缓存判断、摘要加载、导出流程、渲染与按钮事件现已集中在 `apps/desktop/frontend/src/features/data-health.js`。
- environment layer 与 watchlist breadth 的 paired renderers 现已集中在 `apps/desktop/frontend/src/renderers/environment-breadth.js`。
- `Recent reports` 相关交互现已集中在 `apps/desktop/frontend/src/features/recent-reports.js`，当前最小可用能力是 `Open snapshot`、`Open artifact` 与 `Copy path`。
