# 2026-04-25 main.js 第二段拆分（usage guides）

## 变更内容

- 新增 `apps/desktop/frontend/src/features/usage-guides.js`。
- 将 `main.js` 中 guide viewer 相关的加载、开关、渲染与 guide-specific event binding 迁移到该 slice。
- `main.js` 改为通过 `createUsageGuidesSlice(...)` 调用 guide 相关能力。
- 更新 `apps/desktop/frontend/AGENTS.md` 与 `memory/*`，反映新的 guide 模块边界。

## 原因

- 在纯工具层拆分后，usage guides 已是最适合继续从 `main.js` 抽离的低风险 area。

## 影响

- `main.js` 中与 guide viewer 相关的散点减少，更聚焦于全局渲染与主数据流。
- guide viewer 可以作为独立 slice 继续演进，而不必回到 `main.js` 与其他 dashboard 逻辑混在一起。
