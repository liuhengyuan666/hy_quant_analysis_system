# 2026-04-25 P0 语义一致性收口

## 变更内容

- 更新 `README.md`、`docs/系统架构与数据流.md`、`docs/分析使用手册.md`、`docs/功能模块与处理逻辑.md`、`docs/日常操作手册.md`，修正旧的 scope/global 语义表述。
- 调整 `docs/文档状态说明.md`，不再把 `docs/V2-Phase1-环境层详细技术设计.md` 当作当前实现最高优先级 truth source。
- 在 `docs/V2-Phase1-环境层详细技术设计.md` 顶部加入文档状态说明，明确它更适合作为 Phase 1 设计基线参考，而不是当前实现真相源。
- 更新桌面前端 `apps/desktop/frontend/src/main.js`，让 signal panel 与 trust summary 更直接展示 provenance / snapshot-match 语义。
- 根据 Oracle follow-up，再次更新 `README.md` 与 `docs/日常操作手册.md`，明确桌面端 `Refresh data` 是默认用户路径，CLI 全链路命令是显式工程/高级用户路径。

## 原因

- 当前实现已经比早期 Phase 1 叙述更进一步，但主参考文档和桌面展示仍在传播旧的用户心智模型。

## 影响

- 用户现在更容易区分：当前 scope 的环境解释、signal provenance、backtest 是否匹配当前 snapshot，以及 trust summary 的角色。
- 后续可以基于更一致的文档与 UI，再推进“一个主可信度入口 + 两个证据层”的进一步收敛。
