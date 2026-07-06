pub mod json;
pub mod markdown;
pub mod text;

pub use json::JsonFormatter;
pub use markdown::MarkdownFormatter;
pub use text::TextFormatter;

use anyhow::Result;
use reporting::{Formatter, ReportDocument};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Render a ReportDocument using the provided Formatter.
pub fn render<F: Formatter>(formatter: &mut F, doc: &ReportDocument) {
    formatter.render_document(doc);
    for section in &doc.sections {
        formatter.render_section(section);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use reporting::{ReportDocument, ReportLayout, ReportMetadata, ReportSection, SectionContent, SectionKind};

    fn dummy_document() -> ReportDocument {
        ReportDocument {
            layout: ReportLayout::Detail,
            sections: vec![
                ReportSection {
                    kind: SectionKind::Trend,
                    title: "Trend".to_string(),
                    content: SectionContent::Markdown("State: risk_on".to_string()),
                },
                ReportSection {
                    kind: SectionKind::Breadth,
                    title: "Breadth".to_string(),
                    content: SectionContent::Markdown("breadth_pct: 65.0".to_string()),
                },
            ],
            metadata: ReportMetadata {
                title: "Demo Report".to_string(),
                generated_at: Utc::now(),
                scope: "GLOBAL".to_string(),
                date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            },
        }
    }

    #[test]
    fn markdown_formatter_renders() {
        let doc = dummy_document();
        let mut fmt = MarkdownFormatter::new();
        render(&mut fmt, &doc);
        let output = fmt.finalize();
        assert!(output.contains("Demo Report"));
        assert!(output.contains("Trend"));
        assert!(output.contains("risk_on"));
    }

    #[test]
    fn json_formatter_renders() {
        let doc = dummy_document();
        let mut fmt = JsonFormatter::new();
        render(&mut fmt, &doc);
        let output = fmt.finalize();
        assert!(output.contains("Demo Report"));
        assert!(output.contains("Trend"));
    }

    #[test]
    fn text_formatter_renders() {
        let doc = dummy_document();
        let mut fmt = TextFormatter::new();
        render(&mut fmt, &doc);
        let output = fmt.finalize();
        assert!(output.contains("Demo Report"));
        assert!(output.contains("Trend"));
    }
}

/// Unified summary across all analysis skills.
/// This is the Machine Layer output before any human-facing rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSummary {
    pub regime: RegimeState,
    pub confidence: f64,
    pub breadth_condition: llm_context::BreadthCondition,
    pub risk_level: RiskLevel,
    pub rotation: Option<RotationSummary>,
    pub liquidity: Option<LiquiditySummary>,
    pub key_drivers: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSummary {
    pub rotation_detected: bool,
    pub rotation_type: String,
    pub leading_sectors: Vec<String>,
    pub lagging_sectors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquiditySummary {
    pub shock_detected: bool,
    pub pressure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegimeState {
    RiskOn,
    Neutral,
    RiskOff,
    DeRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Composes structured ResearchSummary from raw skill JSON outputs.
pub trait ResearchComposer {
    fn compose(&self, skill_name: &str, skill_result: &Value) -> Result<ResearchSummary>;
}

/// Registry that routes skill names to their composers.
pub struct ComposerRegistry {
    composers: Vec<Box<dyn ResearchComposer>>,
}

impl ComposerRegistry {
    pub fn new() -> Self {
        Self { composers: Vec::new() }
    }

    pub fn register(&mut self, composer: Box<dyn ResearchComposer>) {
        self.composers.push(composer);
    }

    pub fn compose(&self, skill_name: &str, skill_result: &Value) -> Result<ResearchSummary> {
        for composer in &self.composers {
            match composer.compose(skill_name, skill_result) {
                Ok(summary) => return Ok(summary),
                Err(_) => continue,
            }
        }
        anyhow::bail!("No composer registered for skill: {}", skill_name)
    }
}

impl Default for ComposerRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(MarketRegimeComposer));
        registry.register(Box::new(SectorRotationComposer));
        registry
    }
}

// ------------------------------------------------------------------
// Market-Regime Composer
// ------------------------------------------------------------------

pub struct MarketRegimeComposer;

impl ResearchComposer for MarketRegimeComposer {
    fn compose(&self, skill_name: &str, skill_result: &Value) -> Result<ResearchSummary> {
        if skill_name != "market-regime-reasoning" {
            anyhow::bail!("MarketRegimeComposer only handles market-regime-reasoning");
        }

        let regime_state = skill_result
            .get("regime_state")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing regime_state"))?;

        let regime = match regime_state {
            "risk_on" => RegimeState::RiskOn,
            "neutral" => RegimeState::Neutral,
            "risk_off" => RegimeState::RiskOff,
            "de_risk" => RegimeState::DeRisk,
            _ => anyhow::bail!("unknown regime_state: {}", regime_state),
        };

        let confidence = skill_result
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let risk_level = skill_result
            .get("risk_assessment")
            .and_then(|v| v.get("level"))
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "low" => RiskLevel::Low,
                "medium" => RiskLevel::Medium,
                "high" => RiskLevel::High,
                "critical" => RiskLevel::Critical,
                _ => RiskLevel::Medium,
            })
            .unwrap_or(RiskLevel::Medium);

        let key_drivers: Vec<String> = skill_result
            .get("key_drivers")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let recommendations: Vec<String> = skill_result
            .get("risk_assessment")
            .and_then(|v| v.get("recommendation"))
            .and_then(|v| v.as_str())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default();

        // Breadth condition is not directly in the skill output; placeholder for now
        let breadth_condition = llm_context::BreadthCondition::Strong;

        Ok(ResearchSummary {
            regime,
            confidence,
            breadth_condition,
            risk_level,
            rotation: None,
            liquidity: None,
            key_drivers,
            recommendations,
        })
    }
}

// ------------------------------------------------------------------
// Sector-Rotation Composer
// ------------------------------------------------------------------

pub struct SectorRotationComposer;

impl ResearchComposer for SectorRotationComposer {
    fn compose(&self, skill_name: &str, skill_result: &Value) -> Result<ResearchSummary> {
        if skill_name != "sector-rotation" {
            anyhow::bail!("SectorRotationComposer only handles sector-rotation");
        }

        let rotation_detected = skill_result
            .get("rotation_detected")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let rotation_type = skill_result
            .get("rotation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("no_rotation")
            .to_string();

        let leading_sectors: Vec<String> = skill_result
            .get("leading_sectors")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("sector").and_then(|s| s.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let lagging_sectors: Vec<String> = skill_result
            .get("lagging_sectors")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("sector").and_then(|s| s.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let confidence = skill_result
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let recommendations: Vec<String> = skill_result
            .get("recommendation")
            .and_then(|v| v.get("action"))
            .and_then(|v| v.as_str())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default();

        let key_drivers = vec![rotation_type.clone()];

        let rotation = Some(RotationSummary {
            rotation_detected,
            rotation_type,
            leading_sectors,
            lagging_sectors,
        });

        Ok(ResearchSummary {
            regime: RegimeState::Neutral, // sector-rotation doesn't output regime
            confidence,
            breadth_condition: llm_context::BreadthCondition::Strong,
            risk_level: RiskLevel::Medium,
            rotation,
            liquidity: None,
            key_drivers,
            recommendations,
        })
    }
}

// ===================================================================
// Research Insight — human-facing conclusion layer (Wave 8)
// ===================================================================

/// Human-facing research insight derived deterministically from a dashboard snapshot.
/// This is the "Insight First" payload: headline, summary, implications, recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchInsight {
    pub headline: String,
    pub summary: String,
    pub implications: Vec<String>,
    pub recommendations: Vec<String>,
    pub confidence: f64,
    pub regime_transition: Option<String>,
}

/// Deterministic composer that maps DashboardSnapshot → ResearchInsight.
/// All rules are heuristic and explicit; no LLM generation occurs here.
pub struct DashboardInsightComposer;

impl DashboardInsightComposer {
    pub fn compose(snapshot: &report_engine::DashboardSnapshot) -> ResearchInsight {
        let trust_level = snapshot
            .trust_summary
            .as_ref()
            .map(|t| t.level.as_str())
            .unwrap_or("review");

        let regime_label = snapshot.regime_label.as_str();
        let regime_stale_days = snapshot.regime_stale_days;
        let risk_score = snapshot.risk_score;

        // ---------- Headline ----------
        let headline = if trust_level == "degraded" {
            "数据可信度受限，建议谨慎参考".to_string()
        } else if regime_stale_days > 5 {
            "宏观数据滞后，市场判断进入观察阶段".to_string()
        } else {
            match regime_label {
                "risk_on" => "市场处于风险偏好状态".to_string(),
                "neutral" => "市场进入观察阶段".to_string(),
                "risk_off" => "市场风险偏好下降".to_string(),
                "de_risk" => "建议降低风险暴露".to_string(),
                _ => "市场状态待确认".to_string(),
            }
        };

        // ---------- Summary ----------
        let mut summary_parts = Vec::new();
        summary_parts.push(format!(
            "系统判断当前市场状态为 {}。",
            Self::regime_display_name(regime_label)
        ));

        if regime_stale_days > 0 {
            summary_parts.push(format!(
                "宏观数据已滞后 {} 天，此前判断的确认度下降。",
                regime_stale_days
            ));
        }

        if let Some(env) = &snapshot.environment {
            summary_parts.push(format!(
                "市场广度维持在 {:.1}%（{}）。",
                env.breadth_pct,
                Self::breadth_state_display(&env.breadth_state)
            ));
        }

        if risk_score > 70.0 {
            summary_parts.push("风险评分偏高，需关注潜在下行。".to_string());
        } else if risk_score < 40.0 {
            summary_parts.push("风险评分较低，环境相对温和。".to_string());
        }

        let summary = summary_parts.join("");

        // ---------- Implications ----------
        let mut implications = Vec::new();

        if let Some(env) = &snapshot.environment {
            if env.breadth_pct < 30.0 {
                implications.push("市场广度收缩，参与标的减少。".to_string());
            } else if env.breadth_pct > 60.0 {
                implications.push("市场广度良好，多数标的处于强势状态。".to_string());
            }
            if let Some(delta) = env.breadth_5d_delta {
                if delta < -5.0 {
                    implications.push("广度近 5 日出现回落，动能减弱。".to_string());
                } else if delta > 5.0 {
                    implications.push("广度近 5 日改善，动能增强。".to_string());
                }
            }
        }

        if regime_stale_days > 3 {
            implications.push("宏观确认度下降，新入场信号可靠性降低。".to_string());
        }

        if trust_level == "review" {
            implications.push("数据健康存在警告，建议下钻复核后再决策。".to_string());
        } else if trust_level == "degraded" {
            implications.push("数据可信度显著下降，当前结论仅供观察。".to_string());
        }

        if snapshot.bullish_signals.is_empty() && !snapshot.top_signals.is_empty() {
            implications.push("当前缺乏明确进攻信号，以防守观察为主。".to_string());
        }

        // ---------- Recommendations ----------
        let mut recommendations = Vec::new();

        if let Some(strategy) = &snapshot.strategy_state {
            let position = strategy.recommended_position_pct;
            if position == 0.0 {
                recommendations.push("保持空仓或极低仓位观望。".to_string());
            } else if position <= 30.0 {
                recommendations.push("控制仓位，以试探性布局为主。".to_string());
            } else if position <= 60.0 {
                recommendations.push("保持中等仓位，等待趋势进一步确认。".to_string());
            } else if position < 100.0 {
                recommendations.push("可维持较高仓位，但避免追涨。".to_string());
            } else {
                recommendations.push("趋势明确时可满仓操作，仍须设置止损。".to_string());
            }
        } else {
            // Fallback when no strategy state
            match regime_label {
                "risk_on" => recommendations.push("趋势向好时可积极参与，注意轮动节奏。".to_string()),
                "neutral" => recommendations.push("保持仓位，暂停新增高风险暴露。".to_string()),
                "risk_off" | "de_risk" => {
                    recommendations.push("降低仓位，优先防御性配置。".to_string())
                }
                _ => recommendations.push("观望为主，等待信号明朗。".to_string()),
            }
        }

        if regime_stale_days > 3 {
            recommendations.push("等待新的宏观数据确认后再做重大调仓。".to_string());
        }

        if trust_level == "review" || trust_level == "degraded" {
            recommendations.push("导出报告前请先检查数据健康详情。".to_string());
        }

        // ---------- Confidence ----------
        let base_confidence = snapshot
            .environment
            .as_ref()
            .map(|e| e.environment_score / 100.0)
            .unwrap_or(0.5)
            .clamp(0.2, 1.0);

        let confidence = if trust_level == "degraded" {
            base_confidence * 0.5
        } else if trust_level == "review" {
            base_confidence * 0.8
        } else if regime_stale_days > 3 {
            base_confidence * 0.85
        } else {
            base_confidence
        };

        // ---------- Regime transition ----------
        let regime_transition = if regime_stale_days > 0 {
            Some(format!(
                "宏观数据滞后 {} 天，当前 regime 基于 {} 数据",
                regime_stale_days, snapshot.regime_as_of_date
            ))
        } else {
            None
        };

        ResearchInsight {
            headline,
            summary,
            implications,
            recommendations,
            confidence,
            regime_transition,
        }
    }

    fn regime_display_name(label: &str) -> &'static str {
        match label {
            "risk_on" => "Risk-On（风险偏好）",
            "neutral" => "Neutral（观察阶段）",
            "risk_off" => "Risk-Off（风险回避）",
            "de_risk" => "De-Risk（降仓防御）",
            _ => "未知状态",
        }
    }

    fn breadth_state_display(state: &str) -> String {
        match state.as_ref() {
            "near_local_high" => "接近局部高点".to_string(),
            "near_local_low" => "接近局部低点".to_string(),
            "expanding" => "扩张中".to_string(),
            "contracting" => "收缩中".to_string(),
            _ => state.to_string(),
        }
    }
}

// ===================================================================
// Daily Report Composer — concise aggregation layer (Wave 9)
// ===================================================================

/// Composes a concise daily report from DashboardSnapshot + optional ResearchInsight.
/// Output is ~20 lines vs the full ~200-line deterministic markdown report.
pub struct DailyReportComposer;

impl DailyReportComposer {
    pub fn compose_markdown(
        snapshot: &report_engine::DashboardSnapshot,
        insight: Option<&ResearchInsight>,
    ) -> String {
        let mut md = String::new();

        // Title
        md.push_str(&format!(
            "# Daily Quant Report (Concise)\n\nScope: {} | Date: {}\n\n",
            snapshot.scope, snapshot.report_date
        ));

        // Insight layer (if available)
        if let Some(i) = insight {
            md.push_str("## Market Conclusion\n\n");
            md.push_str(&format!("**{}**\n\n", i.headline));
            md.push_str(&format!("{}\n\n", i.summary));

            if !i.recommendations.is_empty() {
                md.push_str("**Recommendations:**\n");
                for rec in &i.recommendations {
                    md.push_str(&format!("- {}\n", rec));
                }
                md.push('\n');
            }
        }

        // Key metrics table
        md.push_str("## Key Metrics\n\n");
        md.push_str(&format!("| Regime | Confidence | Breadth | Risk |\n"));
        md.push_str(&format!("|--------|------------|---------|------|\n"));

        let regime_display = snapshot.regime_label.clone();
        let confidence = snapshot
            .environment
            .as_ref()
            .map(|e| format!("{:.0}%", e.environment_score))
            .unwrap_or_else(|| "N/A".to_string());
        let breadth = snapshot
            .environment
            .as_ref()
            .map(|e| format!("{:.1}%", e.breadth_pct))
            .unwrap_or_else(|| "N/A".to_string());
        let risk = format!("{:.0}", snapshot.risk_score);
        md.push_str(&format!("| {} | {} | {} | {} |\n\n", regime_display, confidence, breadth, risk));

        // Rotation highlight
        if !snapshot.top_rotation.is_empty() {
            md.push_str("## Top Rotation\n\n");
            for item in snapshot.top_rotation.iter().take(3) {
                md.push_str(&format!(
                    "- {} | momentum={:.1}\n",
                    item.symbol, item.momentum_score
                ));
            }
            md.push('\n');
        }

        // Signals highlight
        let bullish_count = snapshot.bullish_signals.len();
        let defensive_count = snapshot.defensive_signals.len();
        if bullish_count > 0 || defensive_count > 0 {
            md.push_str("## Signal Highlights\n\n");
            md.push_str(&format!(
                "- Bullish: {} | Defensive: {}\n",
                bullish_count, defensive_count
            ));
            if let Some(top) = snapshot.top_signals.first() {
                md.push_str(&format!(
                    "- Top signal: {} | score={:.1} | label={:?}\n",
                    top.symbol, top.final_score, top.signal_label
                ));
            }
            md.push('\n');
        }

        // Trust verdict
        if let Some(trust) = &snapshot.trust_summary {
            md.push_str("## Trust Verdict\n\n");
            md.push_str(&format!(
                "- Level: {}\n- Headline: {}\n",
                trust.level, trust.headline
            ));
            md.push('\n');
        }

        // Backtest summary
        if let Some(bt) = &snapshot.latest_backtest {
            md.push_str("## Latest Backtest\n\n");
            md.push_str(&format!(
                "- CAGR: {:.2}% | Max DD: {:.2}% | Sharpe: {:.2}\n",
                bt.cagr * 100.0,
                bt.max_drawdown * 100.0,
                bt.sharpe
            ));
            md.push('\n');
        }

        md.push_str("---\n\n");
        md.push_str("*Generated by DailyReportComposer. Full detailed report available via standard export.*\n");

        md
    }
}
