# Platform Evolution v6

> **Status:** Active
> **Scope:** V6 Semantic Foundation → V6 Consumer Migration → V6 Consumer Expansion（后续阶段）
>
> 本文档描述 Pipeline → Platform 的演进。V1~V5 解决「能分析什么」，V6 解决「如何表达分析并让不同消费者共享语义」。

---

## 架构大图

```text
                Engines / Stores
                        │
                        ▼
        Canonical Semantic Model
              (ResearchContext)
                        │
                        ▼
         Presentation Contract
             (ReportDocument)
                        │
        ┌─────────┬─────────┬─────────┐
        ▼         ▼         ▼         ▼
      CLI       Desktop    API      GPT / PDF / Email
```

### 分层含义

| 层 | 代表 | 职责 | 演进速度 |
|---|---|---|---|
| Engines / Stores | `macro-engine`, `rotation-engine`, `signal-engine`, `market-store` | 计算与存储原始研究数据 | 中等 |
| Canonical Semantic Model | `crates/research-context` | 定义跨消费者共享的研究语义 | 慢 |
| Presentation Contract | `crates/reporting` | 定义 `ReportDocument`、`Section`、`Formatter` trait | 中等 |
| Builders | `crates/report-builder` | 按文档类型组装 `ReportDocument` | 中等 |
| Renderers | `crates/report-renderer` | Markdown / JSON / Text / 未来 HTML | 快 |
| Consumers | CLI, Desktop, API, GPT, PDF, Email | 面向最终用户的交付面 | 快 |

---

## 里程碑

### M1: Semantic Foundation（V6）✅

- 建立 `ResearchContext` 作为 Canonical Semantic Model。
- 建立 `ReportDocument` 作为 Presentation Contract。
- 完成 `ResearchContext → ReportDocument → Formatter` 的可运行链路。
- 保持 Production Surface 冻结（`DashboardSnapshot`、`export-report` 不变）。

### M2: Consumer Migration（V6）⏳

- 将 `research *` 和 `audit *` CLI 命令迁移到 Reporting Pipeline。
- Builder 按文档类型组织：`ResearchReportBuilder`、`ReviewReportBuilder`、`AuditReportBuilder`。
- 验收标准：Markdown 输出与现有 CLI 等价，不同 Formatter 对同一 `ResearchContext` 语义等价。

### M3: Consumer Expansion（V6 后续阶段）🔒

- Timeline / Leader / Executive Summary 等 Explainability 能力。
- Desktop Panel / Card / Tree / Table 渲染。
- API JSON 输出、PDF/Email Renderer。
- 所有这些能力都在稳定的 `ResearchContext` 基础上增量开发，不再触及核心契约。

---

## 核心规则

1. **ResearchContext is the canonical semantic model.**
2. **ResearchContext is consumer-neutral.**
3. **ResearchContext evolves conservatively; Presentation evolves rapidly.**
4. **ReportDocument represents a presentation model, not a CLI model.**
5. **Builders may accept additional domain inputs beyond ReportingSnapshot when required by the document type.**
6. **Production Surface stays frozen unless explicitly defrosted by a new ADR.**

---

## 为什么这是未来三年的路线图

在 V6 Semantic Foundation 之前，每个 Consumer（CLI、GPT、Desktop）都直接或间接依赖 `DashboardSnapshot` 或原始 Engine 输出。这导致：

- 同一研究语义被多次解释。
- 新增 Consumer 时需要重新理解 Engine 结构。
- Production Surface 不断被污染。

V6 通过引入 `ResearchContext` 把「研究语义」与「表达方式」解耦：

- Engines 只负责计算。
- `ResearchContext` 只负责定义研究语义。
- Presentation 只负责把语义渲染成各种格式。

这意味着 V6 及以后新增任何 Consumer（API、PDF、Email、新的 Desktop 视图），都不需要再改动 Engine 或核心语义层，只需要新增 Formatter 或 Builder Profile。

---

## 相关文档

- `docs/v6/adr-068-research-context-reporting-layer.md`
- `docs/v6/research-context-contract.md`
- `docs/v6/reporting-consumer-inventory.md`
- `docs/v6/reporting-output-inventory.md`
- `docs/v6/reporting-boundary-inventory.md`
- `docs/v6/reporting-gate-a.md`
