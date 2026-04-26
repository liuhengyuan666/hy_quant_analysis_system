# 2026-04-26 refresh stage control 第一阶段

## 变更内容

- 在 `apps/desktop/src-tauri/src/lib.rs` 为 refresh 增加 `start_stage` / `retry_from_stage` 支持。
- desktop refresh 现在支持：
  - `Retry failed stage`
  - 从 `ingest / indicators / macro / rotation / strategy / signals / backtests` 开始继续跑
- 前端增加 refresh stage selector 和 failed-stage retry 按钮。
- 更新前端知识文档、用户手册和 memory，反映新的 refresh control 范围。
- 根据最终 Oracle follow-up，最终一致性校验失败现在会被明确标成非阶段性 `Refresh consistency validation failed`，不再错误提供 `Retry failed stage`。

## 原因

- 当前最常见的恢复场景并不需要完整的 job-state / cancel/resume 系统，而是能从后续阶段继续跑、避免整条链重来。

## 影响

- refresh 已从“单一按钮”升级成“默认完整刷新 + 轻量阶段控制”。
- 后续如果继续做更重的 stage control，可以在这个 suffix-run 语义上继续扩展，而不必回到零开始设计。
