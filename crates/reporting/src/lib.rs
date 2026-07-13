use chrono::{DateTime, NaiveDate, Utc};
use research_context::ResearchContext;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Reporting 输入快照。
///
/// 只包含 generated_at 和 ResearchContext，不重复 scope/date。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingSnapshot {
    pub generated_at: DateTime<Utc>,
    pub research: ResearchContext,
}

/// 展示层文档模型（纯 Presentation Model，不持有 Snapshot）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDocument {
    pub layout: ReportLayout,
    pub sections: Vec<ReportSection>,
    pub metadata: ReportMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub title: String,
    pub generated_at: DateTime<Utc>,
    pub scope: String,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReportLayout {
    Detail,
    Summary,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub kind: SectionKind,
    pub title: String,
    pub content: SectionContent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SectionKind {
    Trend,
    Breadth,
    Rotation,
    Signal,
    Divergence,
    Stretch,
    Confirmation,
    Recovery,
    Analytics,
    Review,
    Observation,
    Interpretation,
    Risk,
    Evidence,
    Consensus,
    Explanation,
    Disclaimer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionContent {
    Markdown(String),
    Table(TableData),
    Json(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Formatter trait：只负责渲染，不参与业务选择。
pub trait Formatter {
    fn render_document(&mut self, doc: &ReportDocument);
    fn render_section(&mut self, section: &ReportSection);

    fn render_markdown(&mut self, _content: &str) {}
    fn render_table(&mut self, _table: &TableData) {}
    fn render_json(&mut self, _value: &Value) {}

    fn finalize(self) -> String;
}
