# 2026-04-24 建立 memory 目录

## 变更内容

- 新建 `memory/` 目录及基础文件：`product.md`、`tech.md`、`structure.md`、`glossary.md`、`decisions.md`、`context.md`。
- 以当前已确认的 README、TOOLS、runtime/memory、AGENTS 事实初始化项目记忆。

## 原因

- 项目刚引入新的 Agent 规范，需要把记忆系统落地为可持续维护的外部状态。

## 影响

- 后续每轮工作可以先读 `memory/` 获得当前阶段、决策与结构事实。
- 后续探索/执行结果可逐步沉淀到 `memory/`，减少重复判断。
