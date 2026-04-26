# 2026-04-25 main.js 第三段拆分（data health）

## 变更内容

- 新增 `apps/desktop/frontend/src/features/data-health.js`。
- 将 `main.js` 中 data-health 相关的缓存判断、摘要加载、导出流程、渲染与 data-health-specific event wiring 迁移到该 slice。
- `main.js` 改为通过 `createDataHealthSlice(...)` 调用 data-health 相关能力。
- 更新 `apps/desktop/frontend/AGENTS.md` 与 `memory/*`，反映新的 data-health 模块边界。
- 保留 `main.js` 中的 post-bundle stale-cache 检查，由主 dashboard 流程决定何时触发 `dataHealth.loadSummary()`。

## 原因

- 在纯工具层和 guides slice 抽离后，data-health 是下一个边界清晰、风险较低的 frontend area split 目标。

## 影响

- `main.js` 进一步聚焦于 dashboard 主流程与全局 render 调度。
- data-health 可以作为独立 slice 继续演进，而不必回到 `main.js` 与其余 dashboard 逻辑混在一起。
