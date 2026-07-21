//! LLM analysis conversation history (RV1 Phase 3).
//!
//! Records are stored under `workspace/llm-history/{scope}/{action}/{date}.json`.
//!
//! ADR-106 boundary: a previous record injected into a later prompt must be
//! labeled as PREVIOUS INTERPRETATION (前次解读), never as evidence. The LLM's
//! own past outputs are background context, not facts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAnalysisRecord {
    pub scope: String,
    pub action: String,
    pub persona_label: String,
    pub report_date: String,
    pub created_at: String,
    /// Truncated excerpt of the analysis for continuity injection.
    pub summary: String,
    pub analysis_text: String,
}

fn history_dir(project_root: &Path, scope: &str, action: &str) -> PathBuf {
    project_root
        .join("workspace")
        .join("llm-history")
        .join(scope.to_lowercase())
        .join(action)
}

/// Build a short summary from a full analysis text.
/// Keeps the first ~400 chars on a char boundary.
pub fn make_summary(text: &str) -> String {
    const LIMIT: usize = 400;
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= LIMIT {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

/// Persist a record. Writes `{date}.json` under the scope/action directory.
pub fn save_record(project_root: &Path, record: &LlmAnalysisRecord) -> Result<PathBuf> {
    let dir = history_dir(project_root, &record.scope, &record.action);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;
    let path = dir.join(format!("{}.json", record.report_date));
    let json = serde_json::to_string_pretty(record)?;
    std::fs::write(&path, json)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

/// Load the most recent record for (scope, action), if any.
/// "Most recent" = lexicographically greatest `{date}.json` filename.
pub fn latest_record(
    project_root: &Path,
    scope: &str,
    action: &str,
) -> Option<LlmAnalysisRecord> {
    let dir = history_dir(project_root, scope, action);
    let mut dates: Vec<String> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.strip_suffix(".json").map(|stem| stem.to_string())
        })
        .collect();
    dates.sort();
    let latest = dates.pop()?;
    let path = dir.join(format!("{}.json", latest));
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Render the previous interpretation section for prompt injection.
/// The label explicitly marks it as background, not evidence (ADR-106).
pub fn previous_interpretation_section(record: &LlmAnalysisRecord) -> String {
    format!(
        "## 前次解读（{}，{}）\n\n> 注意：以下内容是前次 LLM 解读，仅作连续性背景参考，不是事实证据，不得作为你本次判断的依据来源。\n\n{}",
        record.report_date, record.persona_label, record.summary
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_truncates_on_char_boundary() {
        let text: String = "好".repeat(500);
        let summary = make_summary(&text);
        assert!(summary.ends_with('…'));
        assert!(summary.chars().count() <= 401);
    }

    #[test]
    fn summary_keeps_short_text_intact() {
        let text = "短线偏强，但价值端回避。";
        assert_eq!(make_summary(text), text);
    }

    #[test]
    fn previous_section_is_labeled_not_evidence() {
        let record = LlmAnalysisRecord {
            scope: "GLOBAL".into(),
            action: "market_story".into(),
            persona_label: "市场叙事".into(),
            report_date: "2026-07-20".into(),
            created_at: "2026-07-20T15:00:00Z".into(),
            summary: "前次摘要".into(),
            analysis_text: "全文".into(),
        };
        let section = previous_interpretation_section(&record);
        assert!(section.contains("前次解读"));
        assert!(section.contains("不是事实证据"));
    }
}
