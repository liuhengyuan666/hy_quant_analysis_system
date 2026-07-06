# Reporting Naming Convention

> 目标：统一 V6 Reporting Layer 中 Context、Section、Builder、Formatter 的命名规则，避免同一概念出现多个名称。

---

## 1. 总体原则

1. **使用完整英文单词，避免缩写**（行业通用缩写除外，如 `JSON`、`HTML`、`SRD`）。
2. **名词化命名**：类型/结构体用名词，行为/函数用动词。
3. **避免歧义**：一个概念只对应一个名称。
4. **层次清晰**：从语义层到表现层，命名体现层级关系。

---

## 2. Research Context 命名

### 2.1 主类型

| 名称 | 含义 |
|---|---|
| `ResearchContext` | 研究语义聚合（顶层） |
| `MarketStateSummary` | 市场状态摘要 |
| `BreadthSummary` | 市场广度摘要 |
| `RotationSummary` | 轮动结构摘要 |
| `SignalSummary` | 最终信号摘要 |
| `DivergenceSummary` | Signal-State 背离摘要 |
| `TrustSummary` | 数据可信度摘要 |

### 2.2 字段命名

| 概念 | 字段名 | 说明 |
|---|---|---|
| 状态标签 | `label` | 如 RiskOn / Neutral / RiskOff |
| 趋势评分 | `trend_score` | 0-100 |
| 流动性评分 | `liquidity_score` | 0-100 |
| 风险评分 | `risk_score` | 0-100 |
| 置信度 | `confidence` | 0.0-1.0 |
| 广度百分比 | `breadth_pct` | 0.0-100.0 |
| 5 日变化 | `delta_5d` | 原始变化值 |
| 5 日均线 | `sma5` | 简单移动平均 |
| 排名靠前 | `top` | Vec<T> |
| 排名靠后 | `bottom` | Vec<T> |
| 轮动状态 | `rotation_state` | Broad / Concentrated / Divergent |
| 领涨稳定性 | `leadership_stability` | 0.0-1.0 |
| 看多数量 | `bullish_count` | usize |
| 强烈买入数量 | `strong_buy_count` | usize |
| 平均得分 | `average_score` | f64 |
| 背离持续天数 | `divergence_duration` | i64 |

### 2.3 禁止字段

`ResearchContext` 中禁止出现以下展示相关字段：

- `markdown`
- `summary_text`
- `display_title`
- `rendered_output`
- `section_title`
- `icon`
- `color`

---

## 3. Reporting Contract 命名

### 3.1 主类型

| 名称 | 含义 |
|---|---|
| `ReportingSnapshot` | Reporting 输入快照（包含 `ResearchContext`） |
| `ReportDocument` | 展示层文档模型 |
| `ReportSection` | 文档中的一个章节 |
| `ReportMetadata` | 文档元数据 |
| `ReportLayout` | 文档布局枚举 |
| `SectionKind` | 章节语义类型 |
| `SectionContent` | 章节内容类型 |
| `TableData` | 表格数据 |
| `Metric` | 指标数据 |

### 3.2 Layout 枚举

| 名称 | 含义 |
|---|---|
| `Detail` | 完整详情 |
| `Review` | 区间回顾 |
| `Executive` | 高管摘要（V6 后续阶段） |
| `Timeline` | 时间线（V6 后续阶段） |

### 3.3 SectionKind 枚举

| 名称 | 含义 |
|---|---|
| `Trend` | 趋势/状态 |
| `Breadth` | 市场广度 |
| `Rotation` | 轮动结构 |
| `Stretch` | 市场拉伸 |
| `Analytics` | 条件分析 |
| `Review` | 区间综述 |
| `Executive` | 高管摘要（V6 后续阶段） |
| `Timeline` | 时间线（V6 后续阶段） |
| `Leader` | 领涨统计（V6 后续阶段） |

### 3.4 SectionContent 枚举

| 名称 | 含义 |
|---|---|
| `Markdown(String)` | Markdown 文本 |
| `Table(TableData)` | 表格 |
| `Metrics(Vec<Metric>)` | 指标列表（V6 后续阶段） |
| `Timeline(Vec<TimelinePoint>)` | 时间线（V6 后续阶段） |

---

## 4. Builder 命名

### 4.1 Trait 与实现

| 名称 | 含义 |
|---|---|
| `ReportBuilder` | Builder trait |
| `ResearchBuilder` | 研究报告 Builder |
| `AuditBuilder` | 审计报告 Builder |
| `DailyBuilder` | 日报 Builder（V6 后续阶段） |
| `WeeklyBuilder` | 周报 Builder（V6 后续阶段） |

### 4.2 方法命名

```rust
pub trait ReportBuilder {
    fn build(&self, snapshot: &ReportingSnapshot, layout: ReportLayout) -> Result<ReportDocument>;
}
```

- Builder 必须是**无状态**的。
- 方法名统一用 `build`。

---

## 5. Formatter 命名

### 5.1 Trait

| 名称 | 含义 |
|---|---|
| `Formatter` | 格式化器 trait |

### 5.2 实现

| 名称 | 含义 |
|---|---|
| `MarkdownFormatter` | Markdown 输出 |
| `JsonFormatter` | JSON 输出 |
| `TextFormatter` | 终端纯文本输出 |
| `HtmlFormatter` | HTML 输出（V6 后续阶段预留） |
| `PdfFormatter` | PDF 输出（V6 后续阶段预留） |

### 5.3 方法命名

```rust
pub trait Formatter {
    fn render_document(&mut self, doc: &ReportDocument);
    fn render_section(&mut self, section: &ReportSection);
    fn render_markdown(&mut self, content: &str);
    fn render_table(&mut self, table: &TableData);
    fn finalize(self) -> String;
}
```

---

## 6. Crate 命名

| 名称 | 职责 | 生命周期 |
|---|---|---|
| `crates/research-context` | 研究语义层 Contract | Stable |
| `crates/reporting` | Reporting Contract | Stable |
| `crates/report-builder` | ReportDocument 组装 | Internal |
| `crates/report-renderer` | 多后端 Formatter | Stable |
| `crates/report-engine` | DashboardSnapshot / Daily Report | Frozen |

---

## 7. 冲突处理

若新字段/类型与既有命名冲突：

1. 优先使用本规范中的名称。
2. 对既有代码使用类型别名或迁移适配器，不直接修改 Frozen 的 Production Surface。
3. 在 Boundary Inventory 中记录 Owner 和 Lifecycle。
