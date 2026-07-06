use anyhow::Result;
use core_domain::research::percentile::percentile_label;
use reporting::{
    ReportDocument, ReportLayout, ReportMetadata, ReportSection, ReportingSnapshot, SectionContent,
    SectionKind, TableData,
};

/// ReportBuilder trait — Pending Evaluation.
///
/// Kept as a placeholder for builders that require only a `ReportingSnapshot`.
/// Currently no implementor exists. Do not add artificial implementations just
/// to satisfy the trait; revisit when a real single-input builder pattern emerges.
pub trait ReportBuilder {
    fn build(&self, snapshot: &ReportingSnapshot) -> Result<ReportDocument>;
}

/// 综述报告的文档类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewProfile {
    /// 季度研究综述（90 天窗口聚合）。
    Quarterly,
}

// =====================================================================
// ReportInput structs — document-specific input, owned by document generation
// =====================================================================

/// SRD (Signal-Regime Divergence) 报告输入。
///
/// 这些指标由 CLI/app-service 从历史数据计算后传入 Builder，
/// 属于 ReportInput（document-specific, transient, single-document），
/// 不属于 ResearchContext 的 consumer-neutral 契约。
#[derive(Debug, Clone)]
pub struct SrdReportInput {
    pub strong_buy_count: usize,
    pub buy_count: usize,
    pub average_signal: f64,
    pub duration: i64,
    pub breadth_trend: String,
    pub rotation_pattern: String,
    pub historical_percentile: f64,
    pub interpretation: String,
    pub confidence: String,
    pub state_label: String,
}

/// Stretch (市场拉伸) 报告输入。
#[derive(Debug, Clone)]
pub struct StretchReportInput {
    pub overall: String,
    pub crowding_level: String,
    pub crowding_concentration_pct: f64,
    pub crowding_percentile: f64,
    pub breadth_level: String,
    pub breadth_pct: f64,
    pub breadth_sma5: Option<f64>,
    pub momentum_level: String,
    pub rs120_max: f64,
    pub rs120_top5_avg: f64,
    pub leverage_level: String,
    pub interpretation: String,
    pub risk_level: String,
    pub overall_evidence: String,
}

/// 条件前向收益分析报告输入。
#[derive(Debug, Clone)]
pub struct AnalyticsReportInput {
    pub condition: String,
    pub horizon: usize,
    pub history_window: String,
    pub occurrences: usize,
    pub forward_return_median: f64,
    pub forward_return_mean: f64,
    pub forward_return_best: f64,
    pub forward_return_worst: f64,
    pub positive_ratio: f64,
    pub median_max_drawdown: f64,
}

/// 综述报告输入。
#[derive(Debug, Clone)]
pub struct ReviewReportInput {
    pub window_from: chrono::NaiveDate,
    pub window_to: chrono::NaiveDate,
    pub observation_days: usize,
    pub calendar_days: i64,
    pub srd_frequency_pct: f64,
    pub avg_divergence_duration: f64,
    pub longest_srd_streak: i64,
    pub latest_srd_dates: Vec<chrono::NaiveDate>,
    pub stretch_distribution: std::collections::HashMap<String, usize>,
    pub crowding_distribution: std::collections::HashMap<String, usize>,
    pub momentum_distribution: std::collections::HashMap<String, usize>,
    pub breadth_distribution: std::collections::HashMap<String, usize>,
    pub analytics_sections: Vec<String>,
    pub review_points: Vec<String>,
}

/// 审计报告 — 轮动排名输入。
#[derive(Debug, Clone)]
pub struct RotationRankingReportInput {
    pub rows: Vec<RotationRankingInputRow>,
}

#[derive(Debug, Clone)]
pub struct RotationRankingInputRow {
    pub rank: i32,
    pub symbol: String,
    pub name: String,
    pub momentum_score: f64,
    pub rs_120: f64,
    pub signal_label: String,
    pub final_score: f64,
}

/// 审计报告 — 标的打分板输入。
#[derive(Debug, Clone)]
pub struct ScoreboardReportInput {
    pub rows: Vec<ScoreboardInputRow>,
}

#[derive(Debug, Clone)]
pub struct ScoreboardInputRow {
    pub symbol: String,
    pub name: String,
    pub final_score: f64,
    pub signal_label: String,
    pub momentum_score: f64,
    pub regime_label: String,
}

// =====================================================================
// ResearchReportBuilder
// =====================================================================

/// 研究报告 Builder（Frozen API）。
///
/// 不实现 `ReportBuilder` trait，因为需要额外 domain 输入。
/// 每个 build_* 方法接受对应的具体 `ReportInput` 类型。
pub struct ResearchReportBuilder;

impl ResearchReportBuilder {
    pub fn build_srd(snapshot: &ReportingSnapshot, input: &SrdReportInput) -> Result<ReportDocument> {
        let sections = build_srd_sections(snapshot, input);
        Ok(ReportDocument {
            layout: ReportLayout::Detail,
            sections,
            metadata: build_metadata(snapshot),
        })
    }

    pub fn build_stretch(snapshot: &ReportingSnapshot, input: &StretchReportInput) -> Result<ReportDocument> {
        let sections = build_stretch_sections(snapshot, input);
        Ok(ReportDocument {
            layout: ReportLayout::Detail,
            sections,
            metadata: build_metadata(snapshot),
        })
    }

    pub fn build_analytics(snapshot: &ReportingSnapshot, input: &AnalyticsReportInput) -> Result<ReportDocument> {
        let sections = build_analytics_sections(snapshot, input);
        Ok(ReportDocument {
            layout: ReportLayout::Detail,
            sections,
            metadata: build_metadata(snapshot),
        })
    }
}

// =====================================================================
// ReviewReportBuilder
// =====================================================================

/// 综述报告 Builder（Frozen API）。
pub struct ReviewReportBuilder {
    pub profile: ReviewProfile,
}

impl ReviewReportBuilder {
    pub fn new(profile: ReviewProfile) -> Self {
        Self { profile }
    }

    pub fn build(
        &self,
        snapshot: &ReportingSnapshot,
        input: &ReviewReportInput,
    ) -> Result<ReportDocument> {
        let sections = match self.profile {
            ReviewProfile::Quarterly => build_review_sections(snapshot, input),
        };

        Ok(ReportDocument {
            layout: ReportLayout::Review,
            sections,
            metadata: build_metadata(snapshot),
        })
    }
}

// =====================================================================
// AuditReportBuilder
// =====================================================================

/// 审计报告 Builder（Frozen API）。
///
/// 不实现 `ReportBuilder` trait，因为需要额外 domain 输入。
/// 每个 build_* 方法接受对应的具体 `ReportInput` 类型。
pub struct AuditReportBuilder;

impl AuditReportBuilder {
    pub fn build_rotation_ranking(snapshot: &ReportingSnapshot, input: &RotationRankingReportInput) -> Result<ReportDocument> {
        let sections = build_rotation_ranking_sections(snapshot, input);
        Ok(ReportDocument {
            layout: ReportLayout::Summary,
            sections,
            metadata: build_metadata(snapshot),
        })
    }

    pub fn build_scoreboard(snapshot: &ReportingSnapshot, input: &ScoreboardReportInput) -> Result<ReportDocument> {
        let sections = build_scoreboard_sections(snapshot, input);
        Ok(ReportDocument {
            layout: ReportLayout::Summary,
            sections,
            metadata: build_metadata(snapshot),
        })
    }
}

// =====================================================================
// Section builders — SRD
// =====================================================================

fn build_srd_sections(snapshot: &ReportingSnapshot, input: &SrdReportInput) -> Vec<ReportSection> {
    let mut sections = Vec::new();

    // Observation header
    sections.push(ReportSection {
        kind: SectionKind::Observation,
        title: format!("SRD Statistics | Date: {} | Scope: {}", snapshot.research.date, snapshot.research.scope.as_str()),
        content: SectionContent::Markdown(format!(
            "  StrongBuy count:       {}\n\
             \x20 Buy count:             {}\n\
             \x20 Average Signal:        {:.1}\n\
             \x20 Duration:              {} days (consecutive trading days with divergence)\n\
             \x20 Breadth trend:         {}\n\
             \x20 Rotation pattern:      {}\n\
             \x20 Historical percentile: {:.0}% ({})",
            input.strong_buy_count,
            input.buy_count,
            input.average_signal,
            input.duration,
            input.breadth_trend,
            input.rotation_pattern,
            input.historical_percentile,
            percentile_label(input.historical_percentile),
        )),
    });

    // Interpretation
    sections.push(ReportSection {
        kind: SectionKind::Interpretation,
        title: "Interpretation".to_string(),
        content: SectionContent::Markdown(format!(
            "  Interpretation:        {}\n\
             \x20 Confidence:            {}",
            input.interpretation,
            input.confidence,
        )),
    });

    // Disclaimer
    sections.push(ReportSection {
        kind: SectionKind::Disclaimer,
        title: String::new(),
        content: SectionContent::Markdown("Observation tool \u{2014} does not influence any decision logic".to_string()),
    });

    sections
}

// =====================================================================
// Section builders — Stretch
// =====================================================================

fn build_stretch_sections(snapshot: &ReportingSnapshot, input: &StretchReportInput) -> Vec<ReportSection> {
    let mut sections = Vec::new();

    // Overall
    sections.push(ReportSection {
        kind: SectionKind::Stretch,
        title: format!("Market Stretch | Scope: {} | Date: {}", snapshot.research.scope.as_str(), snapshot.research.date),
        content: SectionContent::Markdown(format!(
            "  Overall:               {}\n\
             \x20   Evidence:            {}\n\
             \x20 Interpretation:        {}\n\
             \x20 Risk Level:            {}",
            input.overall,
            input.overall_evidence,
            input.interpretation,
            input.risk_level,
        )),
    });

    // Crowding
    let mut crowding_md = format!(
        "  Crowding:              {}\n\
         \x20   Evidence:\n\
         \x20     Top5 Rotation = {:.1}%",
        input.crowding_level, input.crowding_concentration_pct,
    );
    if input.crowding_percentile.is_finite() {
        crowding_md.push_str(&format!("\n      Historical Percentile = {:.0}%", input.crowding_percentile));
    } else {
        crowding_md.push_str("\n      Historical Percentile = N/A (insufficient history)");
    }
    sections.push(ReportSection {
        kind: SectionKind::Evidence,
        title: "Crowding".to_string(),
        content: SectionContent::Markdown(crowding_md),
    });

    // Breadth
    let mut breadth_md = format!(
        "  Breadth:               {}\n\
         \x20   Evidence:\n\
         \x20     Breadth = {:.1}%",
        input.breadth_level, input.breadth_pct,
    );
    match input.breadth_sma5 {
        Some(sma5) => breadth_md.push_str(&format!("\n      SMA5 = {:.1}%", sma5)),
        None => breadth_md.push_str("\n      SMA5 = N/A"),
    }
    sections.push(ReportSection {
        kind: SectionKind::Breadth,
        title: "Breadth".to_string(),
        content: SectionContent::Markdown(breadth_md),
    });

    // Momentum
    sections.push(ReportSection {
        kind: SectionKind::Rotation,
        title: "Momentum".to_string(),
        content: SectionContent::Markdown(format!(
            "  Momentum:              {}\n\
             \x20   Evidence:\n\
             \x20     RS120 Max = {:.1}\n\
             \x20     RS120 Top5 Avg = {:.1}",
            input.momentum_level, input.rs120_max, input.rs120_top5_avg,
        )),
    });

    // Leverage
    let leverage_evidence = if input.leverage_level == "Normal" {
        "(not yet available — margin data source pending)".to_string()
    } else {
        format!("Leverage level = {}", input.leverage_level)
    };
    sections.push(ReportSection {
        kind: SectionKind::Risk,
        title: "Leverage".to_string(),
        content: SectionContent::Markdown(format!(
            "  Leverage:              {}\n\
             \x20   Evidence:\n\
             \x20     {}",
            input.leverage_level,
            leverage_evidence,
        )),
    });

    // Disclaimer
    sections.push(ReportSection {
        kind: SectionKind::Disclaimer,
        title: String::new(),
        content: SectionContent::Markdown("Observation tool \u{2014} does not influence any decision logic".to_string()),
    });

    sections
}

// =====================================================================
// Section builders — Analytics
// =====================================================================

fn build_analytics_sections(snapshot: &ReportingSnapshot, input: &AnalyticsReportInput) -> Vec<ReportSection> {
    let mut sections = Vec::new();

    let mut md = format!(
        "Conditional Forward Return Analytics | Condition: {} | Scope: {}\n\
         \x20 Analytics version:        v1\n\
         \x20 History window:           {}\n\
         \x20 Occurrences:              {}\n\
         \x20 Horizon:                  {} trading days",
        input.condition,
        snapshot.research.scope.as_str(),
        input.history_window,
        input.occurrences,
        input.horizon,
    );

    if input.occurrences > 0 {
        md.push_str(&format!(
            "\n\
             \x20 Forward return median:    {:+.1}%\n\
             \x20 Forward return mean:      {:+.1}%\n\
             \x20 Forward return best:      {:+.1}%\n\
             \x20 Forward return worst:     {:+.1}%\n\
             \x20 Positive ratio:           {:.1}%\n\
             \x20 Median max drawdown:      {:.1}%",
            input.forward_return_median * 100.0,
            input.forward_return_mean * 100.0,
            input.forward_return_best * 100.0,
            input.forward_return_worst * 100.0,
            input.positive_ratio * 100.0,
            input.median_max_drawdown * 100.0,
        ));
    } else {
        md.push_str("\n  Note:                     Not enough observations. Need more samples.");
    }

    sections.push(ReportSection {
        kind: SectionKind::Analytics,
        title: format!("Analytics: {} | Horizon: {}d", input.condition, input.horizon),
        content: SectionContent::Markdown(md),
    });

    sections.push(ReportSection {
        kind: SectionKind::Disclaimer,
        title: String::new(),
        content: SectionContent::Markdown("Observation tool \u{2014} does not influence any decision logic".to_string()),
    });

    sections
}

// =====================================================================
// Section builders — Review
// =====================================================================

fn build_review_sections(snapshot: &ReportingSnapshot, input: &ReviewReportInput) -> Vec<ReportSection> {
    let mut sections = Vec::new();

    // Header
    sections.push(ReportSection {
        kind: SectionKind::Review,
        title: "Research Quarterly Review".to_string(),
        content: SectionContent::Markdown(format!(
            "**Scope**: {}\n\n\
             **Observation Window**: {} ~ {}\n\n\
             **Report Version**: v1\n\n\
             **Generated At**: {}\n\n\
             **Status**: Observation-only synthesis. Does not modify any decision logic.",
            snapshot.research.scope.as_str(),
            input.window_from,
            input.window_to,
            snapshot.generated_at.format("%Y-%m-%d %H:%M:%S"),
        )),
    });

    // Observation coverage
    sections.push(ReportSection {
        kind: SectionKind::Observation,
        title: "Observation Coverage".to_string(),
        content: SectionContent::Markdown(format!(
            "- Days with complete research data: {}\n\
             - Calendar window: {} days",
            input.observation_days,
            input.calendar_days,
        )),
    });

    // SRD summary
    let mut srd_md = String::new();
    if input.latest_srd_dates.is_empty() {
        srd_md.push_str("No SRD events observed in this window.\n");
    } else {
        srd_md.push_str(&format!(
            "- SRD days: {}\n\
             - SRD frequency: {:.1}%\n\
             - Average divergence duration: {:.1} days\n\
             - Longest consecutive SRD streak: {} days\n\n\
             **Latest SRD dates**:\n",
            input.latest_srd_dates.len(),
            input.srd_frequency_pct,
            input.avg_divergence_duration,
            input.longest_srd_streak,
        ));
        for date in input.latest_srd_dates.iter().rev().take(10) {
            srd_md.push_str(&format!("- {}\n", date));
        }
    }
    sections.push(ReportSection {
        kind: SectionKind::Divergence,
        title: "Signal-Regime Divergence (SRD) Summary".to_string(),
        content: SectionContent::Markdown(srd_md),
    });

    // Stretch distribution
    let mut stretch_md = String::from("### Overall\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = input.stretch_distribution.get(*level).copied().unwrap_or(0);
        stretch_md.push_str(&format!("- {}: {} days\n", level, count));
    }
    stretch_md.push_str("\n### By Dimension\n\n**Crowding**:\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = input.crowding_distribution.get(*level).copied().unwrap_or(0);
        stretch_md.push_str(&format!("- {}: {} days\n", level, count));
    }
    stretch_md.push_str("\n**Momentum**:\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = input.momentum_distribution.get(*level).copied().unwrap_or(0);
        stretch_md.push_str(&format!("- {}: {} days\n", level, count));
    }
    stretch_md.push_str("\n**Breadth**:\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = input.breadth_distribution.get(*level).copied().unwrap_or(0);
        stretch_md.push_str(&format!("- {}: {} days\n", level, count));
    }
    sections.push(ReportSection {
        kind: SectionKind::Stretch,
        title: "Market Stretch Distribution".to_string(),
        content: SectionContent::Markdown(stretch_md),
    });

    // Analytics sections
    for section_md in &input.analytics_sections {
        sections.push(ReportSection {
            kind: SectionKind::Analytics,
            title: "Conditional Forward-Return Analytics".to_string(),
            content: SectionContent::Markdown(section_md.clone()),
        });
    }

    // Evidence worth ADR review
    let review_md = if input.review_points.is_empty() {
        "No strong evidence requiring ADR review was observed in this window.\n".to_string()
    } else {
        let mut s = String::new();
        for point in &input.review_points {
            s.push_str(&format!("- {}\n", point));
        }
        s
    };
    sections.push(ReportSection {
        kind: SectionKind::Interpretation,
        title: "Evidence Worth ADR Review".to_string(),
        content: SectionContent::Markdown(review_md),
    });

    // Disclaimer
    sections.push(ReportSection {
        kind: SectionKind::Disclaimer,
        title: String::new(),
        content: SectionContent::Markdown(
            "**Disclaimer**: This report is produced by the Research Layer for evidence accumulation only. \
             It does not modify Strategy State, Signal, Execution, or Risk logic. \
             Historical statistics are not predictions of future returns.".to_string(),
        ),
    });

    sections
}

// =====================================================================
// Section builders — Audit: Rotation Ranking
// =====================================================================

fn build_rotation_ranking_sections(_snapshot: &ReportingSnapshot, input: &RotationRankingReportInput) -> Vec<ReportSection> {
    let headers = vec![
        "Rank".to_string(),
        "Symbol".to_string(),
        "Name".to_string(),
        "Momentum".to_string(),
        "RS120".to_string(),
        "Signal".to_string(),
        "Score".to_string(),
    ];
    let rows: Vec<Vec<String>> = input.rows.iter().map(|r| {
        vec![
            r.rank.to_string(),
            r.symbol.clone(),
            r.name.clone(),
            format!("{:.2}", r.momentum_score),
            format!("{:.1}", r.rs_120),
            r.signal_label.clone(),
            format!("{:.1}", r.final_score),
        ]
    }).collect();

    vec![ReportSection {
        kind: SectionKind::Rotation,
        title: "Rotation Ranking".to_string(),
        content: SectionContent::Table(TableData { headers, rows }),
    }]
}

// =====================================================================
// Section builders — Audit: Scoreboard
// =====================================================================

fn build_scoreboard_sections(_snapshot: &ReportingSnapshot, input: &ScoreboardReportInput) -> Vec<ReportSection> {
    let headers = vec![
        "Symbol".to_string(),
        "Name".to_string(),
        "Score".to_string(),
        "Signal".to_string(),
        "Momentum".to_string(),
        "Regime".to_string(),
    ];
    let rows: Vec<Vec<String>> = input.rows.iter().map(|r| {
        vec![
            r.symbol.clone(),
            r.name.clone(),
            format!("{:.1}", r.final_score),
            r.signal_label.clone(),
            format!("{:.2}", r.momentum_score),
            r.regime_label.clone(),
        ]
    }).collect();

    vec![ReportSection {
        kind: SectionKind::Signal,
        title: "Symbol Scoreboard".to_string(),
        content: SectionContent::Table(TableData { headers, rows }),
    }]
}

// =====================================================================
// Shared helpers
// =====================================================================

fn build_metadata(snapshot: &ReportingSnapshot) -> ReportMetadata {
    ReportMetadata {
        title: format!("Research Report — {}", snapshot.research.scope.as_str()),
        generated_at: snapshot.generated_at,
        scope: snapshot.research.scope.as_str().to_string(),
        date: snapshot.research.date,
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use core_domain::AnalysisScope;
    use research_context::{
        BreadthSummary, DivergenceSummary, MarketStateSummary, ResearchContext, RotationItem,
        RotationSummary, SignalItem, SignalSummary, TrustSummary,
    };

    fn dummy_context() -> ResearchContext {
        ResearchContext {
            version: 1,
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            market_state: MarketStateSummary {
                label: "risk_on".to_string(),
                trend_score: 75.0,
                liquidity_score: 60.0,
                risk_score: 40.0,
                confidence: 0.8,
            },
            breadth: BreadthSummary {
                breadth_pct: 65.0,
                sma5: Some(62.0),
                delta_5d: Some(3.0),
                condition: "Strong".to_string(),
            },
            rotation: RotationSummary {
                top: vec![RotationItem {
                    rank: 1,
                    symbol: "TECH".to_string(),
                    momentum_score: 95.0,
                }],
                bottom: vec![],
                rotation_state: "Concentrated".to_string(),
                leadership_stability: 0.7,
            },
            signal: SignalSummary {
                signals: vec![SignalItem {
                    symbol: "TECH".to_string(),
                    final_score: 88.0,
                    signal_label: "StrongBuy".to_string(),
                }],
                bullish_count: 5,
                strong_buy_count: 2,
                average_score: 72.0,
            },
            divergence: DivergenceSummary {
                divergence_duration: 0,
                samples: vec![],
            },
            trust: TrustSummary {
                level: research_context::TrustLevel::Unassessed,
                headline: "Data healthy".to_string(),
                is_data_complete: true,
            },
        }
    }

    fn dummy_snapshot() -> ReportingSnapshot {
        ReportingSnapshot {
            generated_at: chrono::Utc::now(),
            research: dummy_context(),
        }
    }

    #[test]
    fn srd_builder_produces_document() {
        let snapshot = dummy_snapshot();
        let input = SrdReportInput {
            strong_buy_count: 3,
            buy_count: 2,
            average_signal: 72.0,
            duration: 2,
            breadth_trend: "Improving".to_string(),
            rotation_pattern: "Mixed".to_string(),
            historical_percentile: 75.0,
            interpretation: "Signals are strong while Strategy remains conservative.".to_string(),
            confidence: "Moderate".to_string(),
            state_label: "NO_TRADE".to_string(),
        };
        let doc = ResearchReportBuilder::build_srd(&snapshot, &input).expect("build should succeed");

        assert!(!doc.sections.is_empty());
        assert_eq!(doc.metadata.scope, "GLOBAL");
    }

    #[test]
    fn stretch_builder_produces_document() {
        let snapshot = dummy_snapshot();
        let input = StretchReportInput {
            overall: "Normal".to_string(),
            crowding_level: "Normal".to_string(),
            crowding_concentration_pct: 25.0,
            crowding_percentile: 40.0,
            breadth_level: "Normal".to_string(),
            breadth_pct: 65.0,
            breadth_sma5: Some(62.0),
            momentum_level: "Normal".to_string(),
            rs120_max: 55.0,
            rs120_top5_avg: 45.0,
            leverage_level: "Normal".to_string(),
            interpretation: "Stretch readings are within normal ranges.".to_string(),
            risk_level: "Low".to_string(),
            overall_evidence: "All dimensions within normal ranges".to_string(),
        };
        let doc = ResearchReportBuilder::build_stretch(&snapshot, &input).expect("build should succeed");

        assert!(!doc.sections.is_empty());
    }

    #[test]
    fn analytics_builder_produces_document() {
        let snapshot = dummy_snapshot();
        let input = AnalyticsReportInput {
            condition: "srd-strong".to_string(),
            horizon: 20,
            history_window: "2025-01-01 ~ 2026-06-30".to_string(),
            occurrences: 5,
            forward_return_median: 0.03,
            forward_return_mean: 0.05,
            forward_return_best: 0.15,
            forward_return_worst: -0.08,
            positive_ratio: 0.6,
            median_max_drawdown: 0.05,
        };
        let doc = ResearchReportBuilder::build_analytics(&snapshot, &input).expect("build should succeed");

        assert!(!doc.sections.is_empty());
    }

    #[test]
    fn rotation_ranking_builder_produces_document() {
        let snapshot = dummy_snapshot();
        let input = RotationRankingReportInput {
            rows: vec![RotationRankingInputRow {
                rank: 1,
                symbol: "TECH".to_string(),
                name: "Tech ETF".to_string(),
                momentum_score: 95.0,
                rs_120: 80.0,
                signal_label: "StrongBuy".to_string(),
                final_score: 88.0,
            }],
        };
        let doc = AuditReportBuilder::build_rotation_ranking(&snapshot, &input).expect("build should succeed");

        assert!(!doc.sections.is_empty());
    }

    #[test]
    fn scoreboard_builder_produces_document() {
        let snapshot = dummy_snapshot();
        let input = ScoreboardReportInput {
            rows: vec![ScoreboardInputRow {
                symbol: "TECH".to_string(),
                name: "Tech ETF".to_string(),
                final_score: 88.0,
                signal_label: "StrongBuy".to_string(),
                momentum_score: 95.0,
                regime_label: "risk_on".to_string(),
            }],
        };
        let doc = AuditReportBuilder::build_scoreboard(&snapshot, &input).expect("build should succeed");

        assert!(!doc.sections.is_empty());
    }
}
