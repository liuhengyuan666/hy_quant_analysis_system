# ADR-068: Research Context + Reporting Layer v1

## 状态

Accepted

## 背景

当前系统的报告输出存在以下问题：

1. **语义重复**：`DashboardSnapshot`、`ResearchSnapshot`（CLI 内部）、`ResearchContext`（既有 crate）都在描述相同的市场状态、广度、轮动、信号等概念，但字段命名和结构不同。
2. **Consumer 分散**：CLI、GPT、Desktop、未来 API/PDF 各自消费不同来源的数据，导致同一信息被多次理解和转换。
3. **Presentation 与 Business 耦合**：`apps/cli/src/commands/research.rs` 和 `apps/cli/src/commands/audit.rs` 直接拼 Markdown/println!，新增输出格式需要修改命令逻辑。
4. **Production Surface 风险**：GPT/LLM 上下文当前从 `DashboardSnapshot`（Production Surface）构建，违反 ADR-066 的冻结原则。

## 目标

建立统一的研究语义层（Research Context）和展示管道（Reporting Layer），使 CLI/GPT/Desktop/API/PDF 共享同一套语义模型，同时保持 Production Surface 冻结。

## 最终方案

### 架构分层

```
Engine / Store
    ↓
research-context      # 研究语义层 Contract（Stable）
    ↓
reporting             # Presentation Contract（Stable）
    ↓
report-builder        # ReportDocument 组装（Internal）
    ↓
report-renderer       # Markdown / JSON / Text Formatter（Stable）
    ↓
CLI / Desktop / API / PDF
```

### Crate 职责

| Crate | 职责 | 依赖限制 |
|---|---|---|
| `crates/research-context` | 定义 `ResearchContext` 及 6 个 Summary：State/Breadth/Rotation/Signal/Divergence/Trust | 不依赖 `report-engine`、`reporting`、`report-builder`、`report-renderer` |
| `crates/reporting` | 定义 `ReportingSnapshot`、`ReportDocument`、`SectionKind`、`SectionContent`、`Formatter` trait | 依赖 `research-context`；不依赖 `report-builder`、`report-renderer` |
| `crates/report-builder` | 实现 `ReportBuilder` trait，组装 `ReportDocument` | 依赖 `reporting`、`research-context`；不依赖 `report-renderer` |
| `crates/report-renderer` | 实现 `MarkdownFormatter`/`JsonFormatter`/`TextFormatter`；保留旧版 `DashboardInsightComposer`/`DailyReportComposer` | 新 Formatter 仅依赖 `reporting`；旧 Composer 保留对 `report-engine`/`llm-context` 的遗留依赖，作为 backward-compatibility bridge |
| `crates/report-engine` | Production Surface：`DashboardSnapshot`、完整 Daily Report 渲染 | 冻结，V6 不修改 |
| `crates/app-service` | 从 Engine/Store 构建 `ResearchContext` 和 `ReportingSnapshot` | 可依赖 `research-context`、`reporting`、`report-builder`；CLI 调用 Renderer |

### Architecture Rules

> **Rule-01: ResearchContext is the canonical semantic model for all cross-consumer research outputs.**
>
> 任何 Engine 向 Consumer 提供 Market State / Breadth / Rotation / Signal / Divergence / Trust 等研究语义时，都应转换为 ResearchContext 中定义的 Summary 类型。

> **Rule-02: ResearchContext is consumer-neutral.**
>
> ResearchContext 的字段只能来源于研究语义，不能因 GPT、Desktop、Markdown、CLI 等 Consumer 需求而新增字段。

> **Rule-03: ResearchContext evolves conservatively; Presentation evolves rapidly.**
>
> ResearchContext 的变更必须保持向后兼容；ReportDocument、Section、Formatter 可以自由扩展。

> **Rule-04: ReportDocument represents a presentation model, not a CLI model.**
>
> ReportDocument 禁止出现 `ConsoleWidth`、`AnsiColor`、`TerminalStyle` 等 CLI/终端相关字段。它服务 CLI、Desktop、API、PDF、Email 等所有 Consumer。

> **Rule-05: Builders may accept additional domain inputs beyond ReportingSnapshot when required by the document type.**
>
> ReportingSnapshot 保持精简（只含 `generated_at` + `ResearchContext`）。Review/Timeline/Leader 等 Builder 可额外接受 History、Window、Ranking 等独立输入。

### ResearchContext 稳定性约束

- ResearchContext 只聚合研究语义 Summary，不放入原始数据（OHLCV、Symbol 列表、DataFrame 等）。
- 演进优先采用 additive changes，避免修改已有字段语义。
- 不兼容调整通过新增 Summary 或版本迁移处理，不直接破坏 Consumer。
- Stable ≠ immutable：可以演进，但需保持向后兼容。
- ReportingSnapshot 保持精简：只允许 `generated_at` 和 `ResearchContext`，不允许 Timeline、History、Theme、Chart、Attachments 等字段进入。

### Formatter 与 Builder 边界

- **ReportBuilder** 是唯一决定报告中出现哪些 Section 的组件。
- **Formatter** 只负责 `Document → Section → Content` 的渲染，不参与业务选择。
- ResearchContext 中禁止出现任何 Formatter 相关字段（如 `markdown`、`summary_text`、`display_title`）。
- Builder 按文档类型组织（`ResearchReportBuilder`、`ReviewReportBuilder`、`AuditReportBuilder`），不按 CLI 命令组织。

## 未采纳方案（Rejected Alternatives）

### 1. 直接让 `ReportBuilder` 消费 Engine 输出

**原因未采纳**：Engine 输出是原始计算结果，不同 Consumer 需要不同的语义聚合。如果 Builder 直接消费 Engine，每个 Builder 都需要重复理解 Engine 结构，且新增 Consumer 时需要新增 Builder-Engine 映射。

### 2. 让 `ReportDocument` 持有 `ReportingSnapshot`

**原因未采纳**：`ReportDocument` 是 Pure Presentation Model，不应包含 Domain Model。持有 Snapshot 会导致 PDF/HTML 等 Consumer 被迫理解 Snapshot 结构，且未来 Snapshot 演进时 Document 也会受影响。

### 3. 使用 trait object 的 `ReportContext`

**原因未采纳**：本系统不是插件架构，Context 数量增长缓慢且均由 Engine 输出。使用 `enum` 更简单、类型安全，且无需 `Any`/`TypeId`/`Arc<dyn>` 等运行时开销。

### 4. 在 V6 加入 Executive Summary、Timeline、Leader

**原因未采纳**：
- Executive Summary 属于 Explainability/Interpretation，不是纯 Presentation，应延后到 V6 后续阶段。
- Timeline 需要 History Snapshot 稳定，依赖 Reporting Layer 先运行一段时间。
- Leader 只是 Rotation/Signal 的 Presentation 统计，可在 V6 后续阶段作为新 Section 接入，无需新增评分。

### 5. 将 `ReportingSnapshot` / `ReportDocument` 放入 `report-engine`

**原因未采纳**：`report-engine` 是 Production Surface 的生成器，职责是生成数据。`ReportDocument` 是 Presentation Contract，放入 `report-engine` 会导致该 crate 同时承担 Engine 和 Presentation 职责，长期会越来越胖。

### 6. 保留既有 `crates/research-context` 不变，新建其他名称的语义层 crate

**原因未采纳**：既有 `research-context` 实际职责是为 LLM 提供上下文（从 `DashboardSnapshot` 构建），命名为 `llm-context` 更准确。研究系统的统一语义层应使用 `research-context` 这个名称。

## V6 边界

**做**：
- 建立 `research-context`、`reporting`、`report-builder`、`report-renderer` 四个 crate。
- 完成一条可运行的 `ResearchContext → ReportingSnapshot → ReportDocument → Formatter` 演示链路。
- 在 `app-service` 中增加 `demo_reporting_pipeline` 和 `build_research_context_from_dashboard`。
- 将既有 `research-context` 重命名为 `llm-context`，并更新所有引用。
- 将既有 `research-renderer` 重命名为 `report-renderer`，并更新所有引用。

**不做**：
- 不迁移任何 CLI 业务命令到 Reporting Layer。
- 不修改 `export-report` / `DashboardSnapshot`（Production Surface 冻结）。
- 不新增 Executive Summary、Timeline、Leader 等分析/解释能力。
- 不实现 PDF/HTML/Email Renderer。

## 验证

- `cargo check`：全 workspace 通过。
- `cargo test -p research-context -p reporting -p report-builder -p report-renderer -p app-service`：通过。
- CLI 输出保持不变（`export-report`、`research-context` 等命令未修改行为）。

## 演进路径

- **V6 Consumer Migration**：迁移 `research *` / `audit *` CLI 命令到 Reporting Layer。
- **V6 后续阶段**：在稳定 `ResearchContext` 基础上，新增 Timeline、Leader Summary、Executive Summary 等 Consumer/Explainability 能力。

## 相关文档

- `docs/v6/reporting-consumer-inventory.md`
- `docs/v6/reporting-output-inventory.md`
- `docs/v6/reporting-naming-convention.md`
- `docs/v6/reporting-boundary-inventory.md`
- `docs/v6/reporting-gate-a.md`
