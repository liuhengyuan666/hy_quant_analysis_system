# 2026-04-25 main.js 第四段拆分（environment + breadth renderers）

## 变更内容

- 新增 `apps/desktop/frontend/src/renderers/environment-breadth.js`。
- 将 `main.js` 中 `renderEnvironmentPanel()` 与 `renderWatchlistBreadthPanel()` 迁移到该模块。
- `main.js` 改为通过 `createEnvironmentBreadthRenderers(...)` 调用这组 paired renderers。
- 更新 `apps/desktop/frontend/AGENTS.md` 与 `memory/*`，反映新的 render 模块边界。
- 根据 Oracle follow-up，补齐 `main.js` 中传给既有 slice 的 helper imports，并修正 frontend AGENTS 里旧的 environment panel 定位。

## 原因

- 在前几轮拆掉 utils、guides、data-health 后，environment + breadth 是最清晰、最低风险的下一组 render seam。

## 影响

- `main.js` 中的 render 区进一步收缩，更聚焦于主 render 组合而不是所有 panel 的具体实现。
- environment layer 与 breadth proxy 现在以显式配对模块存在，更符合产品语义和后续维护方式。
