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

/// Render the shared adversarial hypothesis-background section (ADR-112).
///
/// Semantics: this is a HYPOTHESIS BACKGROUND, not a conclusion. Downstream
/// personas are instructed to verify or refute it against system data.
///
/// Level:
/// - `"full"`     → inject the complete `analysis_text`
/// - `"standard"` → inject `analysis_text` (TODO: TASK-215 apply max_chars truncation)
/// - `"compact"`  → inject the truncated `summary` (~400 chars)
/// - anything else (including `"none"`, which callers should have filtered
///   out already) → degrades to compact
pub fn adversarial_context_section(record: &LlmAnalysisRecord, level: &str) -> String {
    let body = match level {
        "full" | "standard" => {
            // TODO TASK-215: standard level should apply max_chars truncation
            record.analysis_text.as_str()
        }
        _ => record.summary.as_str(), // "compact" and unknown fallback
    };
    format!(
        "## 市场博弈假设背景（{}）\n\n> 注意：以下内容为前置博弈分析产生的**假设性背景**，描述市场可能的博弈结构。\n> 它不是事实证据，不是结论，不得作为你本次判断的依据来源。\n> 你的职责是结合系统数据验证或反驳其中的假设。\n\n{}",
        record.report_date, body
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

    #[test]
    fn adversarial_section_is_hypothesis_not_conclusion() {
        let record = LlmAnalysisRecord {
            scope: "GLOBAL".into(),
            action: "adversarial".into(),
            persona_label: "市场博弈视角".into(),
            report_date: "2026-07-24".into(),
            created_at: "2026-07-24T15:00:00Z".into(),
            summary: "摘要内容".into(),
            analysis_text: "完整的博弈分析全文".into(),
        };
        let section = adversarial_context_section(&record, "full");
        assert!(section.contains("假设性背景"));
        assert!(section.contains("验证或反驳"));
        assert!(section.contains("不是结论"));
        assert!(section.contains("2026-07-24"));
        assert!(section.contains("完整的博弈分析全文"));
        assert!(!section.contains("摘要内容"));
    }

    #[test]
    fn adversarial_section_compact_level_uses_summary() {
        let record = LlmAnalysisRecord {
            scope: "GLOBAL".into(),
            action: "adversarial".into(),
            persona_label: "市场博弈视角".into(),
            report_date: "2026-07-24".into(),
            created_at: "2026-07-24T15:00:00Z".into(),
            summary: "摘要内容".into(),
            analysis_text: "完整的博弈分析全文".into(),
        };
        let section = adversarial_context_section(&record, "compact");
        assert!(section.contains("摘要内容"));
        assert!(!section.contains("完整的博弈分析全文"));
        // unknown level degrades to compact
        let degraded = adversarial_context_section(&record, "none");
        assert!(degraded.contains("摘要内容"));
    }

    #[test]
    fn adversarial_section_standard_level_uses_analysis_text() {
        let record = LlmAnalysisRecord {
            scope: "GLOBAL".into(),
            action: "adversarial".into(),
            persona_label: "市场博弈视角".into(),
            report_date: "2026-07-24".into(),
            created_at: "2026-07-24T15:00:00Z".into(),
            summary: "摘要内容".into(),
            analysis_text: "完整的博弈分析全文".into(),
        };
        let section = adversarial_context_section(&record, "standard");
        assert!(section.contains("完整的博弈分析全文"));
        assert!(!section.contains("摘要内容"));
    }
}
