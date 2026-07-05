use reporting::{Formatter, ReportDocument, ReportSection, SectionContent, TableData};

pub struct MarkdownFormatter {
    output: String,
}

impl MarkdownFormatter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for MarkdownFormatter {
    fn render_document(&mut self, doc: &ReportDocument) {
        self.output.push_str(&format!("# {}\n\n", doc.metadata.title));
        self.output.push_str(&format!(
            "Scope: {} | Date: {}\n\n",
            doc.metadata.scope, doc.metadata.date
        ));
    }

    fn render_section(&mut self, section: &ReportSection) {
        self.output.push_str(&format!("## {}\n\n", section.title));
        match &section.content {
            SectionContent::Markdown(m) => self.render_markdown(m),
            SectionContent::Table(t) => self.render_table(t),
            SectionContent::Json(v) => self.render_json(v),
        }
        self.output.push('\n');
    }

    fn render_markdown(&mut self, content: &str) {
        self.output.push_str(content);
        self.output.push('\n');
    }

    fn render_table(&mut self, table: &TableData) {
        if table.headers.is_empty() || table.rows.is_empty() {
            return;
        }

        self.output.push_str("| ");
        self.output.push_str(&table.headers.join(" | "));
        self.output.push_str(" |\n");

        self.output.push_str("| ");
        for _ in &table.headers {
            self.output.push_str("--- | ");
        }
        self.output.push('\n');

        for row in &table.rows {
            self.output.push_str("| ");
            self.output.push_str(&row.join(" | "));
            self.output.push_str(" |\n");
        }
    }

    fn render_json(&mut self, value: &serde_json::Value) {
        if let Ok(s) = serde_json::to_string_pretty(value) {
            self.output.push_str("```json\n");
            self.output.push_str(&s);
            self.output.push_str("\n```\n");
        }
    }

    fn finalize(self) -> String {
        self.output
    }
}
