use reporting::{Formatter, ReportDocument, ReportSection, SectionContent};
use serde_json::{json, Value};

pub struct JsonFormatter {
    sections: Vec<Value>,
    metadata: Option<Value>,
}

impl JsonFormatter {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            metadata: None,
        }
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for JsonFormatter {
    fn render_document(&mut self, doc: &ReportDocument) {
        self.metadata = Some(json!({
            "title": doc.metadata.title,
            "scope": doc.metadata.scope,
            "date": doc.metadata.date.to_string(),
            "generated_at": doc.metadata.generated_at.to_rfc3339(),
            "layout": format!("{:?}", doc.layout),
        }));
    }

    fn render_section(&mut self, section: &ReportSection) {
        let content = match &section.content {
            SectionContent::Markdown(m) => json!({ "type": "markdown", "value": m }),
            SectionContent::Table(t) => json!({
                "type": "table",
                "headers": t.headers,
                "rows": t.rows,
            }),
            SectionContent::Json(v) => json!({ "type": "json", "value": v }),
        };

        self.sections.push(json!({
            "kind": format!("{:?}", section.kind),
            "title": section.title,
            "content": content,
        }));
    }

    fn render_markdown(&mut self, _content: &str) {}
    fn render_table(&mut self, _table: &reporting::TableData) {}
    fn render_json(&mut self, _value: &Value) {}

    fn finalize(self) -> String {
        serde_json::to_string_pretty(&json!({
            "metadata": self.metadata,
            "sections": self.sections,
        }))
        .unwrap_or_default()
    }
}
