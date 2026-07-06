use reporting::{Formatter, ReportDocument, ReportSection, SectionContent, TableData};

pub struct TextFormatter {
    output: String,
    indent: usize,
}

impl TextFormatter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for TextFormatter {
    fn render_document(&mut self, doc: &ReportDocument) {
        self.output.push_str(&format!("{}\n", doc.metadata.title));
        self.output.push_str(&format!(
            "Scope: {} | Date: {}\n",
            doc.metadata.scope, doc.metadata.date
        ));
        self.output.push_str("-".repeat(40).as_str());
        self.output.push('\n');
    }

    fn render_section(&mut self, section: &ReportSection) {
        self.output.push_str(&format!("[{}] {}\n", format!("{:?}", section.kind), section.title));
        match &section.content {
            SectionContent::Markdown(m) => self.render_markdown(m),
            SectionContent::Table(t) => self.render_table(t),
            SectionContent::Json(v) => self.render_json(v),
        }
        self.output.push('\n');
    }

    fn render_markdown(&mut self, content: &str) {
        for line in content.lines() {
            self.output.push_str(&" ".repeat(self.indent));
            self.output.push_str(line);
            self.output.push('\n');
        }
    }

    fn render_table(&mut self, table: &TableData) {
        if table.headers.is_empty() {
            return;
        }
        self.output.push_str(&table.headers.join(" | "));
        self.output.push('\n');
        for row in &table.rows {
            self.output.push_str(&row.join(" | "));
            self.output.push('\n');
        }
    }

    fn render_json(&mut self, value: &serde_json::Value) {
        if let Ok(s) = serde_json::to_string(value) {
            self.output.push_str(&s);
            self.output.push('\n');
        }
    }

    fn finalize(self) -> String {
        self.output
    }
}
