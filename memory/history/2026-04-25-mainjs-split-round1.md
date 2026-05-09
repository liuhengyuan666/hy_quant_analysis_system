# 2026-04-25 main.js 第一段拆分

## 变更内容

- 新增 `apps/desktop/frontend/src/lib/dashboard-utils.js`。
- 将 `main.js` 中的 formatting / normalization / markdown / tone 相关纯函数迁移到该模块。
- `main.js` 改为通过 import 使用这些工具函数，保留状态对象、异步加载、事件绑定与主渲染流程。
- 更新 `apps/desktop/frontend/AGENTS.md` 与 `memory/*`，反映新的前端结构事实。
- 根据 Oracle follow-up，为 `dashboard-utils.js` 增加返回值契约说明，明确大多数 helper 返回纯显示值，而 `renderMarkdownContent()` 返回 HTML 片段。

## 原因

- `main.js` 过大且职责过多，需要按低风险顺序逐步拆解。
- 纯工具函数不依赖状态流，是最容易率先抽离的部分。

## 影响

- `main.js` 的入口职责更清晰，阅读时能更快聚焦状态与交互主流程。
- 后续可以继续按 `snapshot / health / guides / render` 维度拆解前端，而不必重新处理这些基础工具函数。
