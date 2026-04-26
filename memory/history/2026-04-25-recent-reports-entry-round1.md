# 2026-04-25 Recent reports 第一阶段升级

## 变更内容

- 新增 `apps/desktop/frontend/src/features/recent-reports.js`。
- 将 `Recent reports` 从纯路径列表升级为可操作入口：
  - `DAILY_REPORT*` 支持 `Open snapshot`
  - 所有 artifact 支持 `Copy path`
- `main.js` 改为通过 recent-reports slice 渲染与绑定交互。
- 更新前端知识文档与 memory，记录 recent reports 已从“被动列表”进入“研究结果管理入口”的第一阶段。

## 原因

- 当前最缺的不是更多 report metadata，而是先让现有 artifact 列表变成真正可操作的研究入口。

## 影响

- 用户可以直接从 recent reports 回到对应的分析快照，而不必手动回忆 scope/date。
- artifact path 也从纯展示变成了可复制动作，为后续更强的结果管理铺路。
