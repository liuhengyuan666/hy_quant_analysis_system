# 2026-05-04 手动同步流水线优化方案

## 背景

- 用户按 README 中的 CLI 手动同步 / 计算链路执行时，出现了“需要跑 2-3 次才把默认最新日期推进到较新日期”的体验问题。
- 分析确认，这更像是：
  - 分步工程命令
  - 与最终 latest-date gate 强门控
  - 之间的抽象错位，而不是单点函数 bug。

## 本次结论

- 当前默认最新日期不是看 `daily_bar`，而是看某个日期是否同时满足：
  - `signal_snapshot` scoped coverage 足够
  - `rotation_rank` scoped coverage 足够
  - 存在 `market_regime`
  - 存在 `environment_snapshot`
- 当前 signal fail-loud / diagnostics alerts 已经降低了最危险的假成功态，但还没有把整个 CLI 路径抽象成“一次完整刷新动作”。

## 文档化后的优先级建议

1. 第一优先级：新增 CLI 聚合命令（如 `refresh-all` / `sync-and-compute`）
2. 第二优先级：在命令末尾直接输出 latest-date gate 是否推进以及阻塞原因
3. 第三优先级：补 `explain-latest-gate` 之类的专用解释命令
4. 中长期：run_id / staging / promote 模型

## 产出

- 新增活跃设计文档：`docs/手动同步流水线优化方案.md`
- 更新：`docs/文档状态说明.md`
- 更新：`memory/decisions.md`
