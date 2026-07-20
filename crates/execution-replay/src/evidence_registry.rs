use serde::{Deserialize, Serialize};

/// TASK-160.3: Evidence Horizon Registry.
///
/// This module defines the canonical `EvidenceDescriptor` for V8 Execution
/// Platform. It turns ADR-105's Evidence Horizon / Role model into code, so
/// that Evidence Modeling and (eventually) Decision Integration always know
/// what role an evidence plays, at what horizon it is valid, and whether it is
/// safe to use standalone.
///
/// This is a Research-only module; it does not modify the Execution Pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceId {
    LeadershipDecay,
    LiquidityPressure,
    ConfirmationDecay,
    BreadthDeterioration,
    RecoveryFailure,
    RiskExpansion,
    Distribution,
    MomentumExpansion,
    TrendParticipation,
    MarketAcceptance,
}

impl std::fmt::Display for EvidenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceId::LeadershipDecay => write!(f, "LeadershipDecay"),
            EvidenceId::LiquidityPressure => write!(f, "LiquidityPressure"),
            EvidenceId::ConfirmationDecay => write!(f, "ConfirmationDecay"),
            EvidenceId::BreadthDeterioration => write!(f, "BreadthDeterioration"),
            EvidenceId::RecoveryFailure => write!(f, "RecoveryFailure"),
            EvidenceId::RiskExpansion => write!(f, "RiskExpansion"),
            EvidenceId::Distribution => write!(f, "Distribution"),
            EvidenceId::MomentumExpansion => write!(f, "MomentumExpansion"),
            EvidenceId::TrendParticipation => write!(f, "TrendParticipation"),
            EvidenceId::MarketAcceptance => write!(f, "MarketAcceptance"),
        }
    }
}

/// The role an evidence plays in the risk cognition layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceRole {
    EntrySignal,
    ExitSignal,
    HoldingRisk,
    RegimeRisk,
    Confirmation,
    Amplifier,
}

/// The natural prediction horizon of an evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceHorizon {
    Immediate,   // T+1 ~ T+5
    ShortTerm,   // T+5 ~ T+20
    MediumTerm,  // T+20 ~ T+60
    LongTerm,    // T+60+
}

/// Validation lifecycle status of an evidence asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    Draft,
    Validated,
    Rejected,
    Conditional,
    Superseded,
}

/// Statistical requirements for an Evidence Asset to be considered "Validated".
///
/// TASK-173: Evidence Validation Contract Hardening.
///
/// This struct defines the minimum statistical thresholds that an Evidence
/// Asset must meet before it can be promoted from `Conditional` to `Validated`.
/// It prevents premature decision integration based on insufficient samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRequirement {
    pub min_samples: usize,
    pub min_precision: f64,
    pub min_lift: f64,
    pub max_false_alarm: f64,
}

/// Quantitative validation metrics for an evidence.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetMetric {
    pub precision: f64,
    pub lift: f64,
    pub sample_count: usize,
    pub horizon_days: usize,
    pub false_reduce_rate: f64,
}

/// Provenance of an Evidence Asset's validation.
///
/// `EvidenceValidationRecord` makes the Evidence Registry traceable: every asset
/// knows which dataset, horizon, and report validated it. Without this, the
/// registry is just a manual status table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceValidationRecord {
    pub dataset_scope: String,
    pub dataset_from: String,
    pub dataset_to: String,
    pub horizon_days: usize,
    pub sample_size: usize,
    pub precision: f64,
    pub lift: f64,
    pub validated_at: String,
    pub report_reference: String,
}

/// Canonical Evidence Asset descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDescriptor {
    pub id: EvidenceId,
    pub name: String,
    pub role: EvidenceRole,
    pub horizon: EvidenceHorizon,
    pub validation_status: ValidationStatus,
    pub target_metric: Option<TargetMetric>,
    pub validation_record: Option<EvidenceValidationRecord>,
    pub validation_requirement: Option<ValidationRequirement>,
    pub dependencies: Vec<EvidenceId>,
    pub standalone_validity: bool,
    pub decision_candidate: bool,
    pub notes: String,
}

impl EvidenceDescriptor {
    /// Checks whether this Evidence Asset meets its `ValidationRequirement`.
    ///
    /// Returns `true` if:
    /// - `validation_status` is `Validated`, AND
    /// - `target_metric` meets `min_samples`, `min_precision`, `min_lift`, `max_false_alarm`
    ///
    /// If `validation_requirement` is `None`, returns `true` (no requirement).
    pub fn meets_validation_requirement(&self) -> bool {
        if self.validation_status != ValidationStatus::Validated {
            return false;
        }
        let Some(req) = &self.validation_requirement else {
            return true;
        };
        let Some(metric) = &self.target_metric else {
            return false;
        };
        metric.sample_count >= req.min_samples
            && metric.precision >= req.min_precision
            && metric.lift >= req.min_lift
            && metric.false_reduce_rate <= req.max_false_alarm
    }
}

/// The V8 Evidence Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRegistry {
    pub version: String,
    pub assets: Vec<EvidenceDescriptor>,
}

impl EvidenceRegistry {
    /// Returns the V8 default registry populated from Phase 2B findings.
    pub fn v8_default() -> Self {
        Self {
            version: "v8.0.0".into(),
            assets: vec![
                EvidenceDescriptor {
                    id: EvidenceId::LeadershipDecay,
                    name: "LeadershipDecay".into(),
                    role: EvidenceRole::HoldingRisk,
                    horizon: EvidenceHorizon::MediumTerm,
                    validation_status: ValidationStatus::Validated,
                    target_metric: Some(TargetMetric {
                        precision: 0.615,
                        lift: 1.50,
                        sample_count: 743,
                        horizon_days: 60,
                        false_reduce_rate: 0.385,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 60,
                        sample_size: 743,
                        precision: 0.615,
                        lift: 1.50,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/leadership_decay_horizon_cn_2026-07-18.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 100,
                        min_precision: 0.60,
                        min_lift: 1.30,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![],
                    standalone_validity: true,
                    decision_candidate: true,
                    notes: "Medium-term Holding Risk signal; best at T+60. Persistence >= 5 days further improves precision to 76.8%.".into(),
                },
                EvidenceDescriptor {
                    id: EvidenceId::LiquidityPressure,
                    name: "LiquidityPressure".into(),
                    role: EvidenceRole::Amplifier,
                    horizon: EvidenceHorizon::MediumTerm,
                    validation_status: ValidationStatus::Conditional,
                    target_metric: Some(TargetMetric {
                        precision: 0.449,
                        lift: 1.10,
                        sample_count: 637,
                        horizon_days: 60,
                        false_reduce_rate: 0.551,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 60,
                        sample_size: 637,
                        precision: 0.449,
                        lift: 1.10,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/liquidity_pressure_volonly_2d_cn_2026-07-18.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 500,
                        min_precision: 0.50,
                        min_lift: 1.20,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![EvidenceId::LeadershipDecay],
                    standalone_validity: false,
                    decision_candidate: false,
                    notes: "Not a standalone signal. Improves Holding Risk Bundle precision from 69.1% to 77.7% when combined with LeadershipDecay.".into(),
                },
                EvidenceDescriptor {
                    id: EvidenceId::ConfirmationDecay,
                    name: "ConfirmationDecay".into(),
                    role: EvidenceRole::Confirmation,
                    horizon: EvidenceHorizon::MediumTerm,
                    validation_status: ValidationStatus::Conditional,
                    target_metric: Some(TargetMetric {
                        precision: 0.376,
                        lift: 0.92,
                        sample_count: 2563,
                        horizon_days: 60,
                        false_reduce_rate: 0.624,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 60,
                        sample_size: 2563,
                        precision: 0.376,
                        lift: 0.92,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/confirmation_decay_strict_cn_2026-07-18.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 1000,
                        min_precision: 0.50,
                        min_lift: 1.20,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![
                        EvidenceId::LeadershipDecay,
                        EvidenceId::LiquidityPressure,
                    ],
                    standalone_validity: false,
                    decision_candidate: false,
                    notes: "Standalone lift < 1.0. As a Confirmatory Dimension in V4 bundle, the strongest bucket reaches 93.9% precision.".into(),
                },
                EvidenceDescriptor {
                    id: EvidenceId::BreadthDeterioration,
                    name: "BreadthDeterioration".into(),
                    role: EvidenceRole::HoldingRisk,
                    horizon: EvidenceHorizon::MediumTerm,
                    validation_status: ValidationStatus::Rejected,
                    target_metric: Some(TargetMetric {
                        precision: 0.486,
                        lift: 1.03,
                        sample_count: 3958,
                        horizon_days: 60,
                        false_reduce_rate: 0.514,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 60,
                        sample_size: 3958,
                        precision: 0.486,
                        lift: 1.03,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/transition_analysis_breadth_deterioration_cn_v2_2026-07-18.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 100,
                        min_precision: 0.50,
                        min_lift: 1.20,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![],
                    standalone_validity: false,
                    decision_candidate: false,
                    notes: "Rejected at T+20. May be revisited as a persistence-based combined dimension (TASK-160.2C).".into(),
                },
                EvidenceDescriptor {
                    id: EvidenceId::RecoveryFailure,
                    name: "RecoveryFailure".into(),
                    role: EvidenceRole::ExitSignal,
                    horizon: EvidenceHorizon::ShortTerm,
                    validation_status: ValidationStatus::Rejected,
                    target_metric: Some(TargetMetric {
                        precision: 0.468,
                        lift: 0.99,
                        sample_count: 1364,
                        horizon_days: 20,
                        false_reduce_rate: 0.532,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 20,
                        sample_size: 1364,
                        precision: 0.468,
                        lift: 0.99,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/transition_analysis_recovery_failure_cn_v2_2026-07-18.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 100,
                        min_precision: 0.50,
                        min_lift: 1.20,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![],
                    standalone_validity: false,
                    decision_candidate: false,
                    notes: "Rejected by ADR-102. Recovery signals are ambiguous in CN 2024-2025.".into(),
                },
                EvidenceDescriptor {
                    id: EvidenceId::RiskExpansion,
                    name: "RiskExpansion".into(),
                    role: EvidenceRole::HoldingRisk,
                    horizon: EvidenceHorizon::ShortTerm,
                    validation_status: ValidationStatus::Rejected,
                    target_metric: Some(TargetMetric {
                        precision: 0.50,
                        lift: 1.45,
                        sample_count: 6,
                        horizon_days: 20,
                        false_reduce_rate: 0.50,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 20,
                        sample_size: 6,
                        precision: 0.50,
                        lift: 1.45,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/bearish_analysis_cn_v2_2026-07-18.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 30,
                        min_precision: 0.50,
                        min_lift: 1.20,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![],
                    standalone_validity: false,
                    decision_candidate: false,
                    notes: "Very small sample (n=6). Not sufficient for validation.".into(),
                },
                EvidenceDescriptor {
                    id: EvidenceId::Distribution,
                    name: "Distribution".into(),
                    role: EvidenceRole::HoldingRisk,
                    horizon: EvidenceHorizon::ShortTerm,
                    validation_status: ValidationStatus::Conditional,
                    target_metric: Some(TargetMetric {
                        precision: 0.40,
                        lift: 1.16,
                        sample_count: 5,
                        horizon_days: 20,
                        false_reduce_rate: 0.60,
                    }),
                    validation_record: Some(EvidenceValidationRecord {
                        dataset_scope: "CN".into(),
                        dataset_from: "2024-01-01".into(),
                        dataset_to: "2025-06-30".into(),
                        horizon_days: 20,
                        sample_size: 5,
                        precision: 0.40,
                        lift: 1.16,
                        validated_at: "2026-07-18".into(),
                        report_reference: "reports/execution-validation/distribution_coverage_cn_full_2026-07-17.md".into(),
                    }),
                    validation_requirement: Some(ValidationRequirement {
                        min_samples: 30,
                        min_precision: 0.50,
                        min_lift: 1.20,
                        max_false_alarm: 0.40,
                    }),
                    dependencies: vec![],
                    standalone_validity: false,
                    decision_candidate: false,
                    notes: "Small sample. Part of V1 bundle but not standalone.".into(),
                },
            ],
        }
    }

    /// Finds an evidence descriptor by id.
    pub fn find(&self, id: EvidenceId) -> Option<&EvidenceDescriptor> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// Returns all evidence descriptors with the given validation status.
    pub fn by_status(&self, status: ValidationStatus) -> Vec<&EvidenceDescriptor> {
        self.assets
            .iter()
            .filter(|a| a.validation_status == status)
            .collect()
    }

    /// Returns all evidence descriptors with the given role.
    pub fn by_role(&self, role: EvidenceRole) -> Vec<&EvidenceDescriptor> {
        self.assets.iter().filter(|a| a.role == role).collect()
    }

    /// Returns all evidence descriptors that are validated as decision candidates.
    pub fn decision_candidates(&self) -> Vec<&EvidenceDescriptor> {
        self.assets
            .iter()
            .filter(|a| a.decision_candidate && a.validation_status == ValidationStatus::Validated)
            .collect()
    }

    /// Returns all evidence descriptors that meet their validation requirement.
    ///
    /// TASK-173: An Evidence Asset is only considered "validated" if it meets
    /// its `ValidationRequirement` (min_samples, min_precision, min_lift, max_false_alarm).
    pub fn validated_assets(&self) -> Vec<&EvidenceDescriptor> {
        self.assets
            .iter()
            .filter(|a| a.meets_validation_requirement())
            .collect()
    }

    /// Validates that a set of evidence ids can be used together in a bundle.
    ///
    /// Returns `Ok(())` if all ids are known and their dependencies are satisfied.
    /// Returns `Err` with a message if any evidence is unknown or missing dependencies.
    pub fn validate_bundle(&self, evidence_ids: &[EvidenceId]) -> anyhow::Result<()> {
        let mut missing = Vec::new();
        for id in evidence_ids {
            if self.find(*id).is_none() {
                missing.push(format!("unknown evidence: {}", id));
            }
        }
        if !missing.is_empty() {
            anyhow::bail!("Evidence Registry validation failed: {}", missing.join(", "));
        }

        for id in evidence_ids {
            let descriptor = self.find(*id).expect("checked above");
            for dep in &descriptor.dependencies {
                if !evidence_ids.contains(dep) {
                    anyhow::bail!(
                        "Evidence {} requires dependency {} which is not present in the bundle",
                        descriptor.name,
                        dep
                    );
                }
            }
        }
        Ok(())
    }

    /// Checks whether a single evidence can be used as a standalone decision input.
    pub fn is_standalone_candidate(&self, id: EvidenceId) -> bool {
        self.find(id)
            .map(|d| d.standalone_validity && d.decision_candidate)
            .unwrap_or(false)
    }

    /// Checks whether a bundle can enter the Decision path.
    ///
    /// A bundle is acceptable if it contains at least one validated decision
    /// candidate and all dependencies are satisfied.
    pub fn is_bundle_decision_ready(&self, evidence_ids: &[EvidenceId]) -> bool {
        if self.validate_bundle(evidence_ids).is_err() {
            return false;
        }
        self.decision_candidates()
            .iter()
            .any(|c| evidence_ids.contains(&c.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_all_phase2b_evidence() {
        let registry = EvidenceRegistry::v8_default();
        assert!(registry.find(EvidenceId::LeadershipDecay).is_some());
        assert!(registry.find(EvidenceId::LiquidityPressure).is_some());
        assert!(registry.find(EvidenceId::ConfirmationDecay).is_some());
        assert!(registry.find(EvidenceId::BreadthDeterioration).is_some());
        assert!(registry.find(EvidenceId::RecoveryFailure).is_some());
    }

    #[test]
    fn leadership_decay_is_validated_decision_candidate() {
        let registry = EvidenceRegistry::v8_default();
        let ld = registry.find(EvidenceId::LeadershipDecay).unwrap();
        assert_eq!(ld.validation_status, ValidationStatus::Validated);
        assert!(ld.standalone_validity);
        assert!(ld.decision_candidate);
    }

    #[test]
    fn liquidity_pressure_is_not_standalone() {
        let registry = EvidenceRegistry::v8_default();
        let lp = registry.find(EvidenceId::LiquidityPressure).unwrap();
        assert!(!lp.standalone_validity);
        assert!(!lp.decision_candidate);
    }

    #[test]
    fn confirmation_decay_requires_leadership_and_liquidity() {
        let registry = EvidenceRegistry::v8_default();
        let cd = registry.find(EvidenceId::ConfirmationDecay).unwrap();
        assert!(cd.dependencies.contains(&EvidenceId::LeadershipDecay));
        assert!(cd.dependencies.contains(&EvidenceId::LiquidityPressure));
    }

    #[test]
    fn validate_bundle_succeeds_with_valid_dependencies() {
        let registry = EvidenceRegistry::v8_default();
        let bundle = vec![
            EvidenceId::LeadershipDecay,
            EvidenceId::LiquidityPressure,
            EvidenceId::ConfirmationDecay,
        ];
        assert!(registry.validate_bundle(&bundle).is_ok());
    }

    #[test]
    fn validate_bundle_fails_without_leadership_dependency() {
        let registry = EvidenceRegistry::v8_default();
        let bundle = vec![EvidenceId::ConfirmationDecay];
        assert!(registry.validate_bundle(&bundle).is_err());
    }

    #[test]
    fn bundle_is_decision_ready_when_leadership_present() {
        let registry = EvidenceRegistry::v8_default();
        let bundle = vec![EvidenceId::LeadershipDecay];
        assert!(registry.is_bundle_decision_ready(&bundle));
    }

    #[test]
    fn bundle_not_decision_ready_without_validated_candidate() {
        let registry = EvidenceRegistry::v8_default();
        let bundle = vec![EvidenceId::LiquidityPressure, EvidenceId::ConfirmationDecay];
        assert!(!registry.is_bundle_decision_ready(&bundle));
    }
}
