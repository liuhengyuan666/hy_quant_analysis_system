# 2026-04-26 signal 落后提示增强

## 变更内容

- 扩展 `PipelineDateDiagnostics`，新增 `alerts` 字段。
- 当 `strategy_preference` 最新日期领先于 `signal_snapshot` 时，后端会生成明确提示：先重跑 `compute-signals`，再信任 dashboard/export 默认日期。
- 桌面端 `Pipeline freshness` 面板现在会直接展示这类提示。
- 更新 `README.md` 与 `docs/日常操作手册.md`，把这一排障路径写成显式建议。

## 原因

- 当前系统的 latest available analysis date 强依赖 `signal_snapshot`，用户否则只能从原始 stage 日期表里自己推断 signal 落后问题。

## 影响

- 当 signal 落后导致 dashboard/export 默认日期被卡住时，CLI JSON、桌面端和文档都会更直接提示下一步动作。
