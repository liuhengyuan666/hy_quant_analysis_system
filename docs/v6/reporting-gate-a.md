# Architecture Review Gate A

> 日期：2026-07-03
> 目标：验证 V6 Reporting Layer 的架构假设，解决 Owner 冲突，决定是否进入 Phase B。

---

## 1. 审查输入

- `docs/v6/reporting-consumer-inventory.md`
- `docs/v6/reporting-output-inventory.md`
- `docs/v6/reporting-naming-convention.md`
- `docs/v6/reporting-boundary-inventory.md`

---

## 2. 检查清单

| 检查项 | 状态 | 说明 |
|---|---|---|
| Consumer 覆盖 | 通过 | CLI、GPT、Desktop、API、PDF、Email 已梳理 |
| Output 字段映射 | 通过 | 6 个核心 Summary 已识别 |
| Naming Convention | 通过 | Context / Section / Builder / Formatter 已统一 |
| Boundary 清晰 | 通过 | 各类型 Owner / Producer / Consumer / Lifecycle 已明确 |
| 既有冲突 | 已决策 | 见第 3 节 |
| 无循环依赖 | 通过 | 依赖方向已定义 |

---

## 3. 关键决策：既有 `research-context` 冲突

### 问题

当前 `crates/research-context` 从 `DashboardSnapshot` 构建，依赖 `report-engine`，与 V6 目标架构中 "`research-context` 不依赖 `report-engine`" 冲突。

### 决策

采用 **选项 A**：

> **将现有 `crates/research-context` 重命名为 `crates/llm-context`，新建目标 `crates/research-context` 作为统一研究语义层。**

理由：
1. 现有 crate 实际职责是为 LLM 提供上下文，`llm-context` 命名更准确。
2. 目标 `research-context` 是整个研究系统的语义层，名称不可替代。
3. `research-skills` 是主要消费者，改动范围可控。
4. 避免破坏 Production Surface（`report-engine`）。

### 影响

- `crates/research-context/Cargo.toml` 名称改为 `llm-context`。
- `crates/research-skills` 等依赖者更新引用。
- 新建 `crates/research-context` 承载 V6 语义层。

---

## 4. 通过条件

- 所有 Inventory 文档已完成。
- `research-context` 冲突已决策。
- 依赖方向无循环。

## 5. 结论

**Gate A 通过，批准进入 Phase B。**

Phase B 目标：建立 `ResearchContext → ReportingSnapshot → ReportDocument → Formatter` 第一条可运行链路，完成四个 crate 的基础骨架。

---

## 6. 下一步

1. 重命名既有 `crates/research-context` → `crates/llm-context`。
2. 新建 `crates/research-context`。
3. 新建 `crates/reporting`。
4. 新建 `crates/report-builder`。
5. 重命名 `crates/research-renderer` → `crates/report-renderer`。
6. 在 `app-service` 中增加 `build_research_context()` 和 `build_reporting_snapshot()`。
7. `cargo check` 验证。
