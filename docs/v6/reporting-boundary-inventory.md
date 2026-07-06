# Reporting Boundary Inventory

> 目标：明确 Reporting Layer 各核心类型的 Owner、Producer、Consumer 和生命周期，避免未来出现职责不清或循环依赖。

---

## 1. 类型边界总表

| Type | Owner | Producer | Consumer | Lifecycle |
|---|---|---|---|---|
| `ResearchContext` | `research-context` | `app-service` | `report-builder`, `research-skills` (未来), Desktop (未来), API (未来) | Stable |
| `MarketStateSummary` | `research-context` | `app-service` | `ResearchContext` | Stable |
| `BreadthSummary` | `research-context` | `app-service` | `ResearchContext` | Stable |
| `RotationSummary` | `research-context` | `app-service` | `ResearchContext` | Stable |
| `SignalSummary` | `research-context` | `app-service` | `ResearchContext` | Stable |
| `DivergenceSummary` | `research-context` | `app-service` | `ResearchContext` | Stable |
| `TrustSummary` | `research-context` | `app-service` | `ResearchContext` | Stable |
| `ReportingSnapshot` | `reporting` | `app-service` | `report-builder` | Internal |
| `ReportDocument` | `reporting` | `report-builder` | `report-renderer` | Stable |
| `ReportSection` | `reporting` | `report-builder` | `report-renderer` | Stable |
| `SectionKind` | `reporting` | `report-builder` | `report-renderer` | Stable |
| `SectionContent` | `reporting` | `report-builder` | `report-renderer` | Stable |
| `ReportLayout` | `reporting` | `CLI` / `Desktop` | `report-builder` | Stable |
| `Formatter` trait | `report-renderer` | `report-renderer` | CLI, Desktop, API | Stable |
| `DashboardSnapshot` | `report-engine` | `report-engine` | Dashboard, `export-report`, Desktop | Frozen |
| `TrustSummary` (existing in `report-engine`) | `report-engine` | `app-service` | `DashboardSnapshot` | Frozen |

---

## 2. Crate 边界

| Crate | 职责 | 允许依赖 | 禁止依赖 |
|---|---|---|---|
| `research-context` | 研究语义层 Contract | `core-domain`, `chrono`, `serde` | `report-engine`, `reporting`, `report-builder`, `report-renderer` |
| `reporting` | Presentation Contract | `research-context`, `core-domain`, `chrono`, `serde`, `serde_json` | `report-builder`, `report-renderer` |
| `report-builder` | 组装 `ReportDocument` | `reporting`, `research-context`, `core-domain` | `report-renderer` |
| `report-renderer` | 多后端 Formatter | `reporting`, `core-domain` | `research-context`, `report-engine` |
| `report-engine` | Production Surface 快照与渲染 | `core-domain`, `backtest-engine` | `research-context`, `reporting`, `report-builder`, `report-renderer` |
| `app-service` |  orchestration，构建 `ResearchContext` / `ReportingSnapshot` | `research-context`, `reporting`, `report-engine`, `market-store` 等 | `report-builder`, `report-renderer` |
| `apps/cli` | 参数解析、调用 Service、输出 Renderer 结果 | `app-service`, `report-builder`, `report-renderer` | `report-engine`（除 `export-report` 外） |

---

## 3. 既有冲突：research-context

### 3.1 冲突描述

当前代码库已存在 `crates/research-context`，其状态与 V6 目标架构冲突：

| 维度 | 当前 `research-context` | 目标 `research-context` |
|---|---|---|
| **Owner** | `research-context`（已存在） | `research-context`（同名） |
| **Producer** | 从 `DashboardSnapshot` 构建 | 从 Engine/Store 数据构建 |
| **依赖** | 依赖 `report-engine` | 不依赖 `report-engine` |
| **用途** | LLM 分析上下文 | 统一研究语义层 |
| **生命周期** | 当前未明确定义 | Stable |

### 3.2 影响

- 若直接改造现有 `research-context`，需要同时修改 `research-skills` 等消费者，风险较大。
- 若不处理，新 Reporting Layer 会依赖一个依赖 `report-engine` 的类型，破坏依赖方向。

### 3.3 决策选项

| 选项 | 做法 | 优点 | 缺点 |
|---|---|---|---|
| **A：重命名既有 crate** | 将现有 `research-context` 重命名为 `llm-context` 或 `research-llm-context`，新建目标 `research-context` | 名称最准确，目标架构清晰 | 需要修改 `research-skills` 等引用 |
| **B：新 crate 用不同名** | 目标语义层用 `research-model` / `analysis-context` / `market-context` | 不影响现有代码 | 名称不够直观，长期可能混淆 |
| **C：重构既有 crate** | 将现有 `research-context` 改造为目标语义层，LLM 功能迁移到 `research-skills` | 保留 crate 名称 | 改动面最大，需要同时协调多个消费者 |

### 3.4 推荐方案

**选项 A**：重命名现有 `research-context` 为 `llm-context`，新建目标 `research-context`。

理由：
1. 现有 crate 的实际职责是为 LLM 提供上下文，命名为 `llm-context` 更准确。
2. 目标 `research-context` 是整个研究系统的语义层，名称不可替代。
3. `research-skills` 是现有 crate 的主要消费者，改动范围可控。

---

## 4. 依赖方向验证

### 4.1 目标依赖图

```
Engine / Store
    ↓
research-context      (Stable, no report-engine dep)
    ↓
reporting             (Stable)
    ↓
report-builder        (Internal)
    ↓
report-renderer       (Stable)
    ↓
CLI / Desktop / API
```

### 4.2 禁止的依赖

以下依赖必须禁止：

- `research-context` → `report-engine`
- `research-context` → `reporting`
- `research-context` → `report-builder`
- `research-context` → `report-renderer`
- `reporting` → `report-builder`
- `reporting` → `report-renderer`
- `report-builder` → `report-renderer`
- `report-engine` → `research-context` / `reporting` / `report-builder` / `report-renderer`

---

## 5. Gate A 检查清单

- [ ] Consumer Inventory 证明 CLI 和 GPT 是主要消费者。
- [ ] Output Inventory 识别出 6 个核心 Summary：State / Breadth / Rotation / Signal / Divergence / Trust。
- [ ] Naming Convention 覆盖 Context / Section / Builder / Formatter。
- [ ] Boundary Inventory 明确各类型 Owner / Producer / Consumer / Lifecycle。
- [ ] 不存在职责不清或 Owner 冲突的核心类型（`research-context` 冲突已决策）。
- [ ] 依赖方向图无循环依赖。

---

## 6. Gate A 结论

| 检查项 | 状态 | 说明 |
|---|---|---|
| Consumer 覆盖 | 通过 | CLI、GPT、Desktop、API、PDF、Email 已梳理 |
| Output 字段映射 | 通过 | 6 个核心 Summary 已识别 |
| Naming Convention | 通过 | 已统一 |
| Boundary 清晰 | **待决策** | 既有 `research-context` 冲突需选方案 |
| 无循环依赖 | 通过 | 依赖方向已定义 |

**Gate A 通过条件**：选择 `research-context` 冲突处理方案（推荐选项 A），并更新本 Inventory。
