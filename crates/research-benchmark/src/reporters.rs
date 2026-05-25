use super::metrics::BenchmarkMetrics;

/// Generate benchmark reports
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate a markdown report from metrics
    pub fn to_markdown(metrics: &BenchmarkMetrics) -> String {
        format!(
            "# Benchmark Report: {}\n\n\
             | Metric | Value |\n\
             |--------|-------|\n\
             | Consistency | {:.1}% |\n\
             | Hallucination Score | {:.1}% |\n\
             | Schema Pass Rate | {:.1}% |\n\
             | Semantic Validity | {:.1}% |\n\
             | Latency | {}ms |\n\
             | Token Cost | {} |\n\
             | Runs | {} |\n",
            metrics.skill_name,
            metrics.consistency * 100.0,
            metrics.hallucination_score * 100.0,
            metrics.schema_pass_rate * 100.0,
            metrics.semantic_validity * 100.0,
            metrics.latency_ms,
            metrics.token_cost,
            metrics.runs
        )
    }
}
