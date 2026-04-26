# 2026-04-25 Recent reports 第二阶段升级

## 变更内容

- 在 `apps/desktop/src-tauri/src/lib.rs` 新增 `open_report_artifact` 命令，并限制只能打开 `reports/` 目录下的真实 artifact 文件。
- 在 `apps/desktop/frontend/src/features/recent-reports.js` 接入 `Open artifact` 动作。
- 更新前端知识文档、README、操作手册和 memory，反映 `Recent reports` 已支持 snapshot jump + artifact open + path copy。

## 原因

- 第一阶段已经让 recent reports 成为轻量研究入口，但对 `DATA_HEALTH_REPORT` 等非 snapshot 型 artifact 仍然缺少直接可用动作。

## 影响

- `Recent reports` 现在对 daily reports 和 data-health reports 都具备直接可操作价值。
- 下一步如果继续增强，可以优先考虑 reveal in folder 或 compare previous，而不是急着做 schema/API 重构。
