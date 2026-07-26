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
    /// Provenance of the record (ADR-113/114): "on-demand" for the interactive
    /// analyze path, "market-refresh" for the async prewarm. `None` for records
    /// written before this field existed.
    #[serde(default)]
    pub source: Option<String>,
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

/// Result of rendering the shared adversarial section (ADR-114).
///
/// Provenance: `original_chars` is the body character count before any
/// ContentPolicy truncation, `final_chars` the body character count actually
/// injected (including the truncation marker when truncated), and
/// `truncated` reports whether the ContentPolicy fired.
#[derive(Debug, Clone)]
pub struct AdversarialSectionResult {
    pub section: String,
    pub original_chars: usize,
    pub final_chars: usize,
    pub truncated: bool,
}

/// Truncate `text` at a paragraph boundary within `cap` characters (ADR-114).
///
/// Semantics:
/// - Text fitting within `cap` is returned unchanged (`truncated == false`).
/// - Otherwise whole paragraphs (split on blank lines) are accumulated while
///   they fit within `cap`; a sentence is never cut mid-way.
/// - Fallback: if the FIRST paragraph alone exceeds `cap`, hard-cut at `cap`
///   on a char boundary.
/// - When truncated, the marker `\n\n…（已截断）` is appended to the body.
fn truncate_at_paragraph_boundary(text: &str, cap: usize) -> (String, bool) {
    let total = text.chars().count();
    if total <= cap {
        return (text.to_string(), false);
    }

    // Paragraphs are separated by blank lines (double newline).
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut body = String::new();
    let mut used = 0usize;
    let mut first = true;
    for para in &paragraphs {
        let para_len = para.chars().count();
        let sep_len = if first { 0 } else { 2 }; // re-join with "\n\n"
        if used + sep_len + para_len > cap {
            if first {
                // Single giant paragraph fallback: hard cut at a char boundary.
                body = para.chars().take(cap).collect();
            }
            break;
        }
        if !first {
            body.push_str("\n\n");
        }
        body.push_str(para);
        used += sep_len + para_len;
        first = false;
    }

    body.push_str("\n\n…（已截断）");
    (body, true)
}

/// Render the shared adversarial hypothesis-background section (ADR-112/114).
///
/// Semantics: this is a HYPOTHESIS BACKGROUND, not a conclusion. Downstream
/// personas are instructed to verify or refute it against system data.
///
/// InjectionLevel (content granularity) and ContentPolicy (size protection)
/// are independent knobs (ADR-114):
/// - `"full"`     → `analysis_text` capped at `full_max_chars` (generous hard
///   ceiling, pure protection)
/// - `"standard"` → `analysis_text` capped at `max_chars` (the policy knob)
/// - `"compact"`  → `summary` as-is (already short; no policy needed)
/// - anything else (including `"none"`, which callers should have filtered
///   out already) → degrades to compact
///
/// Truncation always happens at paragraph boundaries, never mid-sentence,
/// and the section header records a truncation note when it fires.
pub fn adversarial_context_section(
    record: &LlmAnalysisRecord,
    level: &str,
    max_chars: usize,
    full_max_chars: usize,
) -> AdversarialSectionResult {
    let (body, original_chars, truncated) = match level {
        "full" => {
            let original = record.analysis_text.chars().count();
            let (body, truncated) =
                truncate_at_paragraph_boundary(&record.analysis_text, full_max_chars);
            (body, original, truncated)
        }
        "standard" => {
            let original = record.analysis_text.chars().count();
            let (body, truncated) =
                truncate_at_paragraph_boundary(&record.analysis_text, max_chars);
            (body, original, truncated)
        }
        _ => {
            // "compact" and unknown fallback: summary is already short.
            let summary = record.summary.clone();
            let len = summary.chars().count();
            (summary, len, false)
        }
    };
    let final_chars = body.chars().count();
    let truncation_note = if truncated {
        format!(
            "\n> （注：本背景已从 {} 字截断至 {} 字）",
            original_chars, final_chars
        )
    } else {
        String::new()
    };
    let section = format!(
        "## 市场博弈假设背景（{}）\n\n> 注意：以下内容为前置博弈分析产生的**假设性背景**，描述市场可能的博弈结构。\n> 它不是事实证据，不是结论，不得作为你本次判断的依据来源。\n> 你的职责是结合系统数据验证或反驳其中的假设。{}\n\n{}",
        record.report_date, truncation_note, body
    );
    AdversarialSectionResult {
        section,
        original_chars,
        final_chars,
        truncated,
    }
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
            source: None,
        };
        let section = previous_interpretation_section(&record);
        assert!(section.contains("前次解读"));
        assert!(section.contains("不是事实证据"));
    }

    fn adversarial_test_record(analysis_text: &str) -> LlmAnalysisRecord {
        LlmAnalysisRecord {
            scope: "GLOBAL".into(),
            action: "adversarial".into(),
            persona_label: "市场博弈视角".into(),
            report_date: "2026-07-24".into(),
            created_at: "2026-07-24T15:00:00Z".into(),
            summary: "摘要内容".into(),
            analysis_text: analysis_text.to_string(),
            source: None,
        }
    }

    #[test]
    fn adversarial_section_is_hypothesis_not_conclusion() {
        let record = adversarial_test_record("完整的博弈分析全文");
        let result = adversarial_context_section(&record, "full", 4000, 12000);
        let section = &result.section;
        assert!(section.contains("假设性背景"));
        assert!(section.contains("验证或反驳"));
        assert!(section.contains("不是结论"));
        assert!(section.contains("2026-07-24"));
        assert!(section.contains("完整的博弈分析全文"));
        assert!(!section.contains("摘要内容"));
        assert!(!result.truncated);
    }

    #[test]
    fn adversarial_section_compact_level_uses_summary() {
        let record = adversarial_test_record("完整的博弈分析全文");
        let result = adversarial_context_section(&record, "compact", 4000, 12000);
        assert!(result.section.contains("摘要内容"));
        assert!(!result.section.contains("完整的博弈分析全文"));
        // unknown level degrades to compact
        let degraded = adversarial_context_section(&record, "none", 4000, 12000);
        assert!(degraded.section.contains("摘要内容"));
    }

    #[test]
    fn adversarial_section_standard_level_uses_analysis_text() {
        let record = adversarial_test_record("完整的博弈分析全文");
        let result = adversarial_context_section(&record, "standard", 4000, 12000);
        assert!(result.section.contains("完整的博弈分析全文"));
        assert!(!result.section.contains("摘要内容"));
    }

    #[test]
    fn truncation_passthrough_when_text_fits() {
        let text = "第一段。\n\n第二段。";
        let record = adversarial_test_record(text);
        let result = adversarial_context_section(&record, "standard", 4000, 12000);
        assert!(!result.truncated);
        assert_eq!(result.original_chars, result.final_chars);
        assert_eq!(result.original_chars, text.chars().count());
        assert!(result.section.contains(text));
        assert!(!result.section.contains("已截断"));
    }

    #[test]
    fn truncation_keeps_whole_paragraphs_only() {
        // 3 paragraphs of 10 chars each; cap fits exactly 2 (+ "\n\n").
        let p1 = "甲".repeat(10);
        let p2 = "乙".repeat(10);
        let p3 = "丙".repeat(10);
        let text = format!("{}\n\n{}\n\n{}", p1, p2, p3);
        let record = adversarial_test_record(&text);
        // cap = 22 → p1 (10) + "\n\n" (2) + p2 (10) fits; p3 would need +12 more.
        let result = adversarial_context_section(&record, "standard", 22, 12000);
        assert!(result.truncated);
        assert!(result.section.contains(&p1));
        assert!(result.section.contains(&p2));
        assert!(!result.section.contains(&p3));
        assert!(result.section.contains("…（已截断）"));
        assert_eq!(result.original_chars, 34);
    }

    #[test]
    fn truncation_hard_cuts_single_giant_paragraph() {
        // One paragraph with no blank lines, exceeding the cap.
        let text = "长".repeat(5000);
        let record = adversarial_test_record(&text);
        let result = adversarial_context_section(&record, "standard", 4000, 12000);
        assert!(result.truncated);
        // Body before the marker is exactly the cap.
        assert!(result.section.contains(&"长".repeat(4000)));
        assert!(!result.section.contains(&"长".repeat(4001)));
        assert!(result.section.contains("…（已截断）"));
        assert_eq!(result.original_chars, 5000);
    }

    #[test]
    fn truncation_header_note_appears_iff_truncated() {
        let long = adversarial_test_record(&"字".repeat(5000));
        let truncated = adversarial_context_section(&long, "standard", 4000, 12000);
        assert!(truncated.truncated);
        assert!(truncated.section.contains("本背景已从 5000 字截断至"));

        let short = adversarial_test_record("短小文本");
        let passthrough = adversarial_context_section(&short, "standard", 4000, 12000);
        assert!(!passthrough.truncated);
        assert!(!passthrough.section.contains("本背景已从"));
    }

    #[test]
    fn compact_level_ignores_content_policy() {
        // Even with a tiny cap, compact injects the summary untouched.
        let record = adversarial_test_record(&"长".repeat(5000));
        let result = adversarial_context_section(&record, "compact", 2, 3);
        assert!(!result.truncated);
        assert!(result.section.contains("摘要内容"));
        assert_eq!(result.original_chars, result.final_chars);
    }

    #[test]
    fn full_level_uses_full_max_chars() {
        // Text between max_chars and full_max_chars: truncated at standard,
        // passthrough at full.
        let text = "全".repeat(8000);
        let record = adversarial_test_record(&text);
        let standard = adversarial_context_section(&record, "standard", 4000, 12000);
        assert!(standard.truncated);
        let full = adversarial_context_section(&record, "full", 4000, 12000);
        assert!(!full.truncated);
        assert_eq!(full.final_chars, 8000);
        // And full's own ceiling still fires when exceeded.
        let huge = adversarial_test_record(&"巨".repeat(20000));
        let full_capped = adversarial_context_section(&huge, "full", 4000, 12000);
        assert!(full_capped.truncated);
        assert!(full_capped.section.contains("…（已截断）"));
    }

    #[test]
    fn old_record_without_source_deserializes() {
        // Records written before ADR-113/114 lack the `source` field.
        let json = r#"{
            "scope": "GLOBAL",
            "action": "adversarial",
            "persona_label": "市场博弈视角",
            "report_date": "2026-07-24",
            "created_at": "2026-07-24T15:00:00Z",
            "summary": "摘要",
            "analysis_text": "全文"
        }"#;
        let record: LlmAnalysisRecord = serde_json::from_str(json).expect("old record must deserialize");
        assert_eq!(record.source, None);
        assert_eq!(record.report_date, "2026-07-24");
    }

    #[test]
    fn record_with_source_round_trips() {
        let record = LlmAnalysisRecord {
            scope: "GLOBAL".into(),
            action: "adversarial".into(),
            persona_label: "市场博弈视角".into(),
            report_date: "2026-07-24".into(),
            created_at: "2026-07-24T15:00:00Z".into(),
            summary: "摘要".into(),
            analysis_text: "全文".into(),
            source: Some("market-refresh".into()),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"source\":\"market-refresh\""));
        let restored: LlmAnalysisRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.source.as_deref(), Some("market-refresh"));
    }
}
