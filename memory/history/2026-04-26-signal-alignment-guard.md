# 2026-04-26 signal alignment guard

## 变更内容

- 为 `signal_snapshot` vs `strategy_preference` 增加 backend 对齐校验 helper。
- 当 `signal_snapshot` 落后于 `strategy_preference` 时，`compute-signals` 不再静默成功，而会直接返回错误。
- desktop refresh 末尾也会做同样的 end-state 一致性校验，避免出现“Refresh success 但 dashboard/export 默认日期仍落后”的假成功态。
- 增加了相关单元测试，并把这个判断方向写入 memory。
- 再补充 latest-day completeness 检查：即使 signal 最新日期已经追平 strategy，只要最新日 coverage 不完整，也会进入同一条 fail-loud 机制。

## 补充说明

- 这一步意味着当前 guard 已覆盖同一类问题的两种常见表现：
  1. signal 日期落后于 strategy
  2. signal 日期追平，但最新日覆盖不完整
- 最终又进一步统一了 guard 入口：`compute-signals` 与 refresh 都直接复用 scoped `PipelineDateDiagnostics.alerts`，refresh 也不再只检查 `GLOBAL` scope。

## 原因

- 仅靠提示用户重跑 `compute-signals` 仍然会让 stale 状态进入 dashboard/export 默认路径。

## 影响

- 如果 signal 层没有追上 strategy 层，系统会更早失败并暴露问题。
- 后续若继续做 deeper sequencing/refactor，可以在这一层 guard 之上继续推进，而不是重新回到纯分析状态。
