//! Divergence ledger: local, deterministic observation artifact (TASK-093).
//!
//! Records versioned JSON at:
//!
//! ```text
//! workspace/divergence-ledger/{scope-lowercase}/{symbol}/{YYYY-MM-DD}.json
//! ```
//!
//! This is a gitignored local observation artifact, explicitly NOT a Research
//! Asset: it carries no `RA-XXXXXX` identity and no `AssetKind`. It records a
//! write-once observation fact (the full `SignalSnapshot` plus the exact-date
//! `StrategyStateSnapshot`) at a case key, together with three independently
//! updated forward-return outcomes (T+20 / T+60 / T+120).
//!
//! Semantics:
//! - Observation facts (`observation` + `observation_mode`), the case key, and
//!   `schema_version` are immutable once written.
//! - Outcomes start `Pending`. The automated classifier turns a `Pending`
//!   outcome into `Filled` maturity facts only when a valid observation bar and
//!   enough strictly-subsequent same-symbol bars with a valid maturity close
//!   exist. Missing or invalid bar data that a later backfill could repair
//!   (observation bar, maturity bar, or insufficient future bars) leaves the
//!   outcome `Pending`. `Filled` and `Unavailable` are terminal.
//! - `Unavailable` is a reserved explicit terminal state for a separately
//!   proven permanent structural reason; the automated classifier never assigns
//!   it, and it only ever arrives from explicit future or manual reasons.
//! - Classification is a versionable structure carrying a workflow status plus
//!   human-owned `category`/`notes`. The automated ledger always writes
//!   `Unclassified` (no category, no notes) and preserves any manually supplied
//!   values across maturity updates; it never defines or assigns a taxonomy.
//! - Deterministic pretty JSON with stable declaration order; identical bytes
//!   are never rewritten.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::research::signal::trading_bar_forward_return;
use core_domain::{
    AnalysisScope, DailyBar, SignalLabel, SignalSnapshot, StrategyState, StrategyStateSnapshot,
};
use market_store::StorageConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Current on-disk schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// How the observation was produced.
///
/// - `Prospective`: observed on the observation date itself (daily TASK-093 run).
/// - `Reconstructed`: rebuilt later from persisted historical data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationMode {
    Prospective,
    Reconstructed,
}

/// Status of a single forward-return outcome.
///
/// `Pending` carries no maturity fact yet: the bars needed to compute the
/// maturity are missing or invalid, and a later backfill may repair them.
/// `Filled` carries the maturity facts and is terminal. `Unavailable` is a
/// reserved explicit terminal state for a separately proven permanent
/// structural reason; it is never assigned by the automated classifier and only
/// ever arrives from explicit future or manual assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OutcomeStatus {
    Pending,
    Filled {
        horizon: usize,
        maturity_date: NaiveDate,
        maturity_close: f64,
        forward_return: f64,
    },
    Unavailable {
        reason: String,
    },
}

impl Default for OutcomeStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// One of the three tracked forward-return horizons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeHorizon {
    T20,
    T60,
    T120,
}

impl OutcomeHorizon {
    pub const ALL: [OutcomeHorizon; 3] = [
        OutcomeHorizon::T20,
        OutcomeHorizon::T60,
        OutcomeHorizon::T120,
    ];

    /// Number of strictly-subsequent trading bars to maturity.
    pub fn as_days(self) -> usize {
        match self {
            Self::T20 => 20,
            Self::T60 => 60,
            Self::T120 => 120,
        }
    }

    /// Field label used in the record and on-disk keys.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::T20 => "t20",
            Self::T60 => "t60",
            Self::T120 => "t120",
        }
    }
}

/// Lossless workflow status for a divergence case's classification.
///
/// The automated ledger writes only `unclassified`, while arbitrary future or
/// human-owned values round-trip verbatim without defining a taxonomy here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ClassificationStatus(pub String);

impl Default for ClassificationStatus {
    fn default() -> Self {
        Self("unclassified".to_string())
    }
}

/// Classification of a divergence case (workflow state + human-owned fields).
///
/// The automated ledger never invents or assigns a category or notes and always
/// writes `Unclassified`. `category` and `notes` are human-owned free-form
/// strings (future taxonomy work, e.g. TASK-100); this layer preserves them
/// verbatim but does not define their vocabulary. Maturity updates must never
/// change any of these fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DivergenceClassification {
    #[serde(default)]
    pub status: ClassificationStatus,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Immutable identity of a divergence case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CaseKey {
    /// Canonical scope string (`GLOBAL` / `CN` / `HK`).
    pub scope: String,
    pub symbol: String,
    pub observation_date: NaiveDate,
}

/// Write-once observation facts at a case key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Full signal snapshot for the observation date.
    pub signal: SignalSnapshot,
    /// Exact-date strategy state snapshot.
    pub strategy_state: StrategyStateSnapshot,
}

/// The three independent forward-return outcomes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeSet {
    #[serde(default)]
    pub t20: OutcomeStatus,
    #[serde(default)]
    pub t60: OutcomeStatus,
    #[serde(default)]
    pub t120: OutcomeStatus,
}

impl Default for OutcomeSet {
    fn default() -> Self {
        Self {
            t20: OutcomeStatus::Pending,
            t60: OutcomeStatus::Pending,
            t120: OutcomeStatus::Pending,
        }
    }
}

impl OutcomeSet {
    pub fn get(&self, horizon: OutcomeHorizon) -> &OutcomeStatus {
        match horizon {
            OutcomeHorizon::T20 => &self.t20,
            OutcomeHorizon::T60 => &self.t60,
            OutcomeHorizon::T120 => &self.t120,
        }
    }

    pub fn set(&mut self, horizon: OutcomeHorizon, status: OutcomeStatus) {
        match horizon {
            OutcomeHorizon::T20 => self.t20 = status,
            OutcomeHorizon::T60 => self.t60 = status,
            OutcomeHorizon::T120 => self.t120 = status,
        }
    }
}

/// Versioned on-disk record for a single divergence case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceRecord {
    /// Intentionally required (no `#[serde(default)]`): this is a v1 schema with
    /// no pre-existing version-less records, so a missing version is treated as
    /// a corrupt record rather than silently defaulted.
    pub schema_version: u32,
    pub case_key: CaseKey,
    /// Immutable provenance; never defaulted because it cannot be inferred safely.
    pub observation_mode: ObservationMode,
    pub observation: Observation,
    #[serde(default)]
    pub outcomes: OutcomeSet,
    #[serde(default)]
    pub classification: DivergenceClassification,
}

impl DivergenceRecord {
    /// Build a fresh record with three `Pending` outcomes and `Unclassified`
    /// classification. The caller is responsible for validation via the ledger.
    pub fn new(
        scope: AnalysisScope,
        symbol: &str,
        observation_date: NaiveDate,
        observation_mode: ObservationMode,
        signal: SignalSnapshot,
        strategy_state: StrategyStateSnapshot,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            case_key: CaseKey {
                scope: scope.as_str().to_string(),
                symbol: symbol.to_string(),
                observation_date,
            },
            observation_mode,
            observation: Observation {
                signal,
                strategy_state,
            },
            outcomes: OutcomeSet::default(),
            classification: DivergenceClassification::default(),
        }
    }
}

/// Result of attempting to create a new record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateOutcome {
    /// A new record was written.
    Created,
    /// An identical record already existed; no bytes were rewritten.
    AlreadyExists,
    /// A record exists with conflicting immutable facts; the existing record was
    /// preserved and no bytes were rewritten.
    Conflict,
}

/// Internal classification used by the write-once helpers.
enum WriteOutcome {
    Created,
    AlreadyExists,
    Conflict,
}

/// Result of attempting to fill a single outcome horizon.
#[derive(Debug, Clone, PartialEq)]
pub enum OutcomeUpdate {
    /// No record exists at the requested case key.
    RecordMissing,
    /// The outcome was already `Filled` or `Unavailable`; left untouched.
    TerminalPreserved,
    /// The outcome remains `Pending`; bars needed for the maturity are missing
    /// or invalid (a backfill may repair them).
    StillPending,
    /// The outcome was filled with concrete maturity facts.
    Filled {
        horizon: usize,
        maturity_date: NaiveDate,
        maturity_close: f64,
        forward_return: f64,
    },
    /// The outcome was marked `Unavailable` for an explicit permanent reason.
    ///
    /// Reserved: the automated classifier never produces this; only explicit
    /// future/manual pathways should.
    Unavailable { reason: String },
}

/// TASK-093 divergence candidate predicate.
///
/// A case is a candidate exactly when the symbol's exact-date signal label is
/// `StrongBuy` and the scope's exact-date strategy state is `DeRisk`. This is a
/// pure function of two snapshots; the caller is responsible for supplying
/// snapshots taken on the same date and for the same scope.
pub fn is_divergence_candidate(signal: &SignalSnapshot, state: &StrategyStateSnapshot) -> bool {
    matches!(signal.signal_label, SignalLabel::StrongBuy)
        && matches!(state.state, StrategyState::DeRisk)
}

/// Resolve the observation mode for a ledger update.
///
/// `Reconstructed` only when the caller explicitly requested a historical date
/// (`explicit_date == true`) that is strictly before the scope's latest
/// persisted signal date; otherwise `Prospective`. An unknown latest date
/// (`None`) never yields `Reconstructed`.
pub fn resolve_observation_mode(
    target_date: NaiveDate,
    latest_signal_date: Option<NaiveDate>,
    explicit_date: bool,
) -> ObservationMode {
    let is_historical = explicit_date
        && latest_signal_date
            .map(|latest| target_date < latest)
            .unwrap_or(false);
    if is_historical {
        ObservationMode::Reconstructed
    } else {
        ObservationMode::Prospective
    }
}

/// Deterministic counts for a single scope-wide outcome maturity sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SweepSummary {
    /// Number of `Pending` outcomes newly matured to `Filled`.
    pub horizons_filled: usize,
    /// Number of `Pending` outcomes newly marked `Unavailable` during this
    /// sweep. The automated classifier never assigns `Unavailable`, so this
    /// counts only transitions driven by explicitly preexisting/assigned
    /// permanent reasons (future/manual pathways); it is expected to be zero
    /// for ordinary automated sweeps.
    pub horizons_unavailable: usize,
    /// Number of `Pending` outcomes left `Pending` (insufficient future bars).
    pub horizons_pending: usize,
    /// Number of terminal outcomes (`Filled`/`Unavailable`) left untouched.
    pub horizons_terminal: usize,
    /// Number of records scanned during the sweep.
    pub records_scanned: usize,
}

/// Deterministic summary of a full divergence-ledger update pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceLedgerUpdateSummary {
    /// New records written for discovered StrongBuy + DeRisk candidates.
    pub cases_created: usize,
    /// Candidates whose identical record already existed (no bytes rewritten).
    pub cases_existing: usize,
    /// Candidates whose existing record had conflicting immutable facts and was
    /// therefore preserved without mutation.
    pub cases_conflicted: usize,
    /// `Pending` outcomes newly matured to `Filled` during the sweep.
    pub horizons_filled: usize,
    /// `Pending` outcomes newly marked `Unavailable` during the sweep. The
    /// automated classifier never assigns `Unavailable`, so this counts only
    /// explicitly preexisting/assigned permanent reasons (future/manual
    /// pathways); expected to be zero for ordinary automated sweeps.
    pub horizons_unavailable: usize,
    /// `Pending` outcomes left `Pending` during the sweep.
    pub horizons_pending: usize,
    /// Terminal outcomes left untouched during the sweep.
    pub horizons_terminal: usize,
    /// Total number of records scanned during the sweep.
    pub records_scanned: usize,
    /// Root-relative paths of newly created records, sorted for determinism.
    pub created_paths: Vec<String>,
}

impl Default for DivergenceLedgerUpdateSummary {
    fn default() -> Self {
        Self {
            cases_created: 0,
            cases_existing: 0,
            cases_conflicted: 0,
            horizons_filled: 0,
            horizons_unavailable: 0,
            horizons_pending: 0,
            horizons_terminal: 0,
            records_scanned: 0,
            created_paths: Vec::new(),
        }
    }
}

/// Reject symbols that are unsafe path components.
pub fn validate_symbol(symbol: &str) -> Result<()> {
    if symbol.is_empty() {
        anyhow::bail!("symbol must not be empty");
    }
    if symbol == "." || symbol == ".." || symbol.trim_end_matches(['.', ' ']) != symbol {
        anyhow::bail!("symbol must not be a path traversal component: {symbol}");
    }
    if symbol
        .chars()
        .any(|c| c == '/' || c == '\\' || c == ':' || c == '\0' || std::path::is_separator(c))
    {
        anyhow::bail!("symbol contains an unsafe path component: {symbol}");
    }
    Ok(())
}

fn validate_scope(scope: &str) -> Result<()> {
    if scope.is_empty()
        || scope == "."
        || scope == ".."
        || scope.trim_end_matches(['.', ' ']) != scope
    {
        anyhow::bail!("scope must not be empty");
    }
    if scope
        .chars()
        .any(|c| c == '/' || c == '\\' || c == ':' || c == '\0' || std::path::is_separator(c))
    {
        anyhow::bail!("scope contains an unsafe path component: {scope}");
    }
    Ok(())
}

/// Serialize a record to deterministic pretty JSON with stable field order.
pub fn serialize_record(record: &DivergenceRecord) -> Result<String> {
    serde_json::to_string_pretty(record).context("failed to serialize divergence record")
}

/// Filesystem-backed divergence ledger.
#[derive(Debug, Clone)]
pub struct DivergenceLedger {
    root: PathBuf,
}

impl DivergenceLedger {
    /// Create a ledger rooted at the given directory (the `divergence-ledger`
    /// base directory itself).
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Ledger rooted at the default workspace location.
    pub fn workspace_default() -> Result<Self> {
        Ok(Self::new(
            StorageConfig::project_root()?
                .join("workspace")
                .join("divergence-ledger"),
        ))
    }

    /// The ledger base directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonical on-disk path for a case key, after safety validation.
    pub fn record_path(&self, scope: &str, symbol: &str, date: NaiveDate) -> Result<PathBuf> {
        validate_scope(scope)?;
        validate_symbol(symbol)?;
        Ok(self
            .root
            .join(scope.to_lowercase())
            .join(symbol)
            .join(format!("{date}.json")))
    }

    /// Create a new record, preserving write-once semantics.
    ///
    /// - If no record exists, writes it and returns `Created`.
    /// - If an identical record (same case key, mode, and observation facts)
    ///   exists, returns `AlreadyExists` without rewriting bytes.
    /// - If a record exists with different immutable facts, errors.
    pub fn create(
        &self,
        scope: AnalysisScope,
        symbol: &str,
        observation_date: NaiveDate,
        observation_mode: ObservationMode,
        signal: SignalSnapshot,
        strategy_state: StrategyStateSnapshot,
    ) -> Result<CreateOutcome> {
        let record = DivergenceRecord::new(
            scope,
            symbol,
            observation_date,
            observation_mode,
            signal,
            strategy_state,
        );
        let path = self.record_path(
            &record.case_key.scope,
            &record.case_key.symbol,
            record.case_key.observation_date,
        )?;
        match self.write_if_absent(record)? {
            WriteOutcome::Created => Ok(CreateOutcome::Created),
            WriteOutcome::AlreadyExists => Ok(CreateOutcome::AlreadyExists),
            WriteOutcome::Conflict => anyhow::bail!(
                "divergence record already exists with different immutable facts: {}",
                path.display()
            ),
        }
    }

    /// Like [`DivergenceLedger::create`], but a conflicting existing record is
    /// reported as [`CreateOutcome::Conflict`] (existing record preserved)
    /// instead of an error. Orchestration uses this so routine historical drift
    /// never breaks a maturity sweep.
    pub fn create_preserving(
        &self,
        scope: AnalysisScope,
        symbol: &str,
        observation_date: NaiveDate,
        observation_mode: ObservationMode,
        signal: SignalSnapshot,
        strategy_state: StrategyStateSnapshot,
    ) -> Result<CreateOutcome> {
        let record = DivergenceRecord::new(
            scope,
            symbol,
            observation_date,
            observation_mode,
            signal,
            strategy_state,
        );
        match self.write_if_absent(record)? {
            WriteOutcome::Created => Ok(CreateOutcome::Created),
            WriteOutcome::AlreadyExists => Ok(CreateOutcome::AlreadyExists),
            WriteOutcome::Conflict => Ok(CreateOutcome::Conflict),
        }
    }

    fn write_if_absent(&self, record: DivergenceRecord) -> Result<WriteOutcome> {
        validate_symbol(&record.case_key.symbol)?;
        let path = self.record_path(
            &record.case_key.scope,
            &record.case_key.symbol,
            record.case_key.observation_date,
        )?;
        self.validate_record(&record, &path)?;

        if let Some(existing) = self.read_file(&path)? {
            if immutable_facts_match(&existing, &record) {
                return Ok(WriteOutcome::AlreadyExists);
            }
            return Ok(WriteOutcome::Conflict);
        }

        self.write_record(&record)?;
        Ok(WriteOutcome::Created)
    }

    /// Load a record by case key, validating it against its on-disk path.
    pub fn load(
        &self,
        scope: AnalysisScope,
        symbol: &str,
        observation_date: NaiveDate,
    ) -> Result<Option<DivergenceRecord>> {
        validate_symbol(symbol)?;
        let path = self.record_path(scope.as_str(), symbol, observation_date)?;
        self.read_file(&path)
    }

    /// Fill a single outcome horizon from persisted bars.
    ///
    /// Only `Pending` outcomes may change. The maturity computation is delegated
    /// to the verified `trading_bar_forward_return` helper; any missing or
    /// invalid bar data (observation bar, maturity bar, or insufficient
    /// strictly-subsequent same-symbol bars) leaves the outcome `Pending`
    /// because a later backfill may repair it. `Unavailable` is never assigned
    /// here; it is a reserved explicit terminal state that is only preserved
    /// untouched when already present on the record.
    pub fn fill_outcome(
        &self,
        scope: AnalysisScope,
        symbol: &str,
        observation_date: NaiveDate,
        horizon: OutcomeHorizon,
        bars: &[DailyBar],
    ) -> Result<OutcomeUpdate> {
        let mut record = match self.load(scope, symbol, observation_date)? {
            Some(record) => record,
            None => return Ok(OutcomeUpdate::RecordMissing),
        };

        if !matches!(record.outcomes.get(horizon), OutcomeStatus::Pending) {
            return Ok(OutcomeUpdate::TerminalPreserved);
        }

        let new_status = classify_outcome(symbol, observation_date, horizon.as_days(), bars);
        match new_status {
            OutcomeStatus::Pending => Ok(OutcomeUpdate::StillPending),
            OutcomeStatus::Filled {
                horizon: h,
                maturity_date,
                maturity_close,
                forward_return,
            } => {
                record.outcomes.set(
                    horizon,
                    OutcomeStatus::Filled {
                        horizon: h,
                        maturity_date,
                        maturity_close,
                        forward_return,
                    },
                );
                self.write_record(&record)?;
                Ok(OutcomeUpdate::Filled {
                    horizon: h,
                    maturity_date,
                    maturity_close,
                    forward_return,
                })
            }
            OutcomeStatus::Unavailable { reason } => {
                record.outcomes.set(
                    horizon,
                    OutcomeStatus::Unavailable {
                        reason: reason.clone(),
                    },
                );
                self.write_record(&record)?;
                Ok(OutcomeUpdate::Unavailable { reason })
            }
        }
    }

    /// Enumerate every record in a scope, deterministically sorted.
    ///
    /// Walks `{root}/{scope-lowercase}` for `*.json` record files, loads and
    /// validates each one against its on-disk path, and returns them sorted by
    /// `(symbol, observation_date)` ascending. Malformed or path-mismatched
    /// records are surfaced as errors (never silently skipped).
    pub fn enumerate_scope_records(&self, scope: AnalysisScope) -> Result<Vec<DivergenceRecord>> {
        let scope_dir = self.root.join(scope.as_str().to_lowercase());
        if !scope_dir.exists() {
            return Ok(Vec::new());
        }

        let mut paths: Vec<PathBuf> = Vec::new();
        for symbol_entry in fs::read_dir(&scope_dir)
            .with_context(|| format!("failed to read scope directory: {}", scope_dir.display()))?
        {
            let symbol_entry = symbol_entry
                .with_context(|| format!("failed to read entry in {}", scope_dir.display()))?;
            let symbol_dir = symbol_entry.path();
            if !symbol_dir.is_dir() {
                continue;
            }
            for file_entry in fs::read_dir(&symbol_dir).with_context(|| {
                format!("failed to read symbol directory: {}", symbol_dir.display())
            })? {
                let file_entry = file_entry
                    .with_context(|| format!("failed to read entry in {}", symbol_dir.display()))?;
                let path = file_entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    paths.push(path);
                }
            }
        }

        // Lexicographic path sort is equivalent to (symbol, date) sort because
        // the layout is `{scope}/{symbol}/{YYYY-MM-DD}.json`.
        paths.sort();

        let mut records = Vec::with_capacity(paths.len());
        for path in paths {
            if let Some(record) = self.read_file(&path)? {
                records.push(record);
            }
        }
        records.sort_by(|a, b| {
            a.case_key.symbol.cmp(&b.case_key.symbol).then(
                a.case_key
                    .observation_date
                    .cmp(&b.case_key.observation_date),
            )
        });
        Ok(records)
    }

    fn read_file(&self, path: &Path) -> Result<Option<DivergenceRecord>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read divergence record: {}", path.display()))?;
        let record: DivergenceRecord = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse divergence record: {}", path.display()))?;
        self.validate_record(&record, path)?;
        Ok(Some(record))
    }

    /// Write a record, skipping the write when the bytes are identical.
    fn write_record(&self, record: &DivergenceRecord) -> Result<bool> {
        let path = self.record_path(
            &record.case_key.scope,
            &record.case_key.symbol,
            record.case_key.observation_date,
        )?;
        self.validate_record(record, &path)?;

        let json = serialize_record(record)?;
        if let Ok(existing) = fs::read(&path) {
            if existing == json.as_bytes() {
                return Ok(false);
            }
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }
        fs::write(&path, json.as_bytes())
            .with_context(|| format!("failed to write divergence record: {}", path.display()))?;
        Ok(true)
    }

    /// Validate that a record's case key matches its path and that embedded
    /// observation facts are consistent with the case key.
    fn validate_record(&self, record: &DivergenceRecord, path: &Path) -> Result<()> {
        let expected = self.record_path(
            &record.case_key.scope,
            &record.case_key.symbol,
            record.case_key.observation_date,
        )?;
        if expected != path {
            anyhow::bail!(
                "case key does not match record path: expected {}, got {}",
                expected.display(),
                path.display()
            );
        }

        let observation = &record.observation;
        if observation.signal.symbol != record.case_key.symbol {
            anyhow::bail!(
                "signal symbol {} does not match case key symbol {}",
                observation.signal.symbol,
                record.case_key.symbol
            );
        }
        if observation.signal.date != record.case_key.observation_date {
            anyhow::bail!(
                "signal date {} does not match observation date {}",
                observation.signal.date,
                record.case_key.observation_date
            );
        }
        if observation.signal.analysis_scope != record.case_key.scope {
            anyhow::bail!(
                "signal analysis_scope {} does not match case key scope {}",
                observation.signal.analysis_scope,
                record.case_key.scope
            );
        }
        if observation.signal.regime_basis_scope != record.case_key.scope {
            anyhow::bail!(
                "signal regime_basis_scope {} does not match case key scope {}",
                observation.signal.regime_basis_scope,
                record.case_key.scope
            );
        }
        if observation.strategy_state.date != record.case_key.observation_date {
            anyhow::bail!(
                "strategy state date {} does not match observation date {}",
                observation.strategy_state.date,
                record.case_key.observation_date
            );
        }
        if observation.strategy_state.scope != record.case_key.scope {
            anyhow::bail!(
                "strategy state scope {} does not match case key scope {}",
                observation.strategy_state.scope,
                record.case_key.scope
            );
        }
        Ok(())
    }
}

/// Compare the immutable facts of two records (everything except outcomes and
/// classification). `SignalSnapshot` / `StrategyStateSnapshot` do not derive
/// `PartialEq`, so comparison is done on deterministic JSON bytes.
fn immutable_facts_match(a: &DivergenceRecord, b: &DivergenceRecord) -> bool {
    if a.schema_version != b.schema_version
        || a.case_key != b.case_key
        || a.observation_mode != b.observation_mode
    {
        return false;
    }
    match (
        serde_json::to_string(&a.observation),
        serde_json::to_string(&b.observation),
    ) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn valid_close(price: f64) -> bool {
    price.is_finite() && price > 0.0
}

/// Classify the outcome status for a single horizon from persisted bars.
///
/// - Missing observation bar, or an observation bar with an invalid close,
///   remains `Pending` because a later data backfill may repair it.
/// - A valid observation bar but fewer than `horizon` strictly-subsequent
///   same-symbol bars leaves the outcome `Pending`.
/// - Otherwise maturity is computed via `trading_bar_forward_return`; a `None`
///   result (e.g. an invalid close on the Nth maturity bar) also remains
///   `Pending`, because the maturity bar itself may be repaired by a backfill.
///
/// This classifier never returns `OutcomeStatus::Unavailable`: that state is
/// reserved for explicit future/manual permanent structural reasons only.
pub fn classify_outcome(
    symbol: &str,
    observation_date: NaiveDate,
    horizon: usize,
    bars: &[DailyBar],
) -> OutcomeStatus {
    match bars
        .iter()
        .find(|bar| bar.symbol == symbol && bar.date == observation_date)
    {
        None => return OutcomeStatus::Pending,
        Some(bar) if !valid_close(bar.close) => return OutcomeStatus::Pending,
        Some(_) => {}
    }

    let subsequent = bars
        .iter()
        .filter(|bar| bar.symbol == symbol && bar.date > observation_date)
        .count();
    if subsequent < horizon {
        return OutcomeStatus::Pending;
    }

    match trading_bar_forward_return(bars, symbol, observation_date, horizon) {
        Some(result) => OutcomeStatus::Filled {
            horizon,
            maturity_date: result.maturity_date,
            maturity_close: result.maturity_close,
            forward_return: result.forward_return,
        },
        None => OutcomeStatus::Pending,
    }
}

/// Mature every `Pending` outcome horizon for a scope using already-fetched
/// bars, delegating each horizon to [`DivergenceLedger::fill_outcome`].
///
/// `bars_by_symbol` maps each symbol to its batched persisted bars (the
/// observation-date bar must be included). Symbols missing from the map are
/// treated as having no bars; such records simply stay `Pending` until a
/// backfill provides their bars. Returns deterministic counts.
pub fn sweep_scope_outcomes(
    ledger: &DivergenceLedger,
    scope: AnalysisScope,
    bars_by_symbol: &HashMap<String, Vec<DailyBar>>,
) -> Result<SweepSummary> {
    let records = ledger.enumerate_scope_records(scope)?;
    let mut summary = SweepSummary {
        records_scanned: records.len(),
        ..SweepSummary::default()
    };

    for record in &records {
        let symbol = record.case_key.symbol.as_str();
        let bars = bars_by_symbol.get(symbol).map(Vec::as_slice).unwrap_or(&[]);
        for horizon in OutcomeHorizon::ALL {
            match ledger.fill_outcome(
                scope,
                symbol,
                record.case_key.observation_date,
                horizon,
                bars,
            )? {
                OutcomeUpdate::RecordMissing => {}
                OutcomeUpdate::TerminalPreserved => summary.horizons_terminal += 1,
                OutcomeUpdate::StillPending => summary.horizons_pending += 1,
                OutcomeUpdate::Filled { .. } => summary.horizons_filled += 1,
                OutcomeUpdate::Unavailable { .. } => summary.horizons_unavailable += 1,
            }
        }
    }

    Ok(summary)
}

/// Create records for candidate divergence cases and tally counts into
/// `summary`. This is the data-independent selection/creation step: it takes
/// already-fetched exact-date signals plus the scope's exact-date strategy state
/// and writes (or preserves) one record per `StrongBuy` + `DeRisk` candidate,
/// sorted by symbol for determinism. The caller supplies exact-date snapshots;
/// any state other than `DeRisk` (including a prior-day-only state) yields no
/// candidates.
pub fn create_candidates(
    ledger: &DivergenceLedger,
    scope: AnalysisScope,
    observation_date: NaiveDate,
    observation_mode: ObservationMode,
    signals: &[SignalSnapshot],
    strategy_state: &StrategyStateSnapshot,
    summary: &mut DivergenceLedgerUpdateSummary,
) -> Result<()> {
    let mut candidates: Vec<&SignalSnapshot> = signals
        .iter()
        .filter(|signal| is_divergence_candidate(signal, strategy_state))
        .collect();
    candidates.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    for signal in candidates {
        match ledger.create_preserving(
            scope,
            &signal.symbol,
            observation_date,
            observation_mode,
            signal.clone(),
            strategy_state.clone(),
        )? {
            CreateOutcome::Created => {
                summary.cases_created += 1;
                let rel = format!(
                    "{}/{}/{}.json",
                    scope.as_str().to_lowercase(),
                    signal.symbol,
                    observation_date
                );
                summary.created_paths.push(rel);
            }
            CreateOutcome::AlreadyExists => summary.cases_existing += 1,
            CreateOutcome::Conflict => summary.cases_conflicted += 1,
        }
    }
    summary.created_paths.sort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Days;
    use core_domain::{
        RegimeReason, RotationReason, SignalLabel, SignalReason, StrategyKind, StrategyState,
    };
    use std::fs;
    use tempfile::TempDir;

    fn d(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn add_days(date: NaiveDate, n: u64) -> NaiveDate {
        date.checked_add_days(Days::new(n)).unwrap()
    }

    fn bar(symbol: &str, date: NaiveDate, close: f64) -> DailyBar {
        DailyBar {
            date,
            symbol: symbol.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
            turnover: None,
        }
    }

    fn signal(symbol: &str, scope: &str, date: NaiveDate, score: f64) -> SignalSnapshot {
        SignalSnapshot {
            date,
            symbol: symbol.to_string(),
            final_score: score,
            signal_label: SignalLabel::StrongBuy,
            analysis_scope: scope.to_string(),
            regime_basis_scope: scope.to_string(),
            reason: SignalReason {
                best_strategy: StrategyKind::TrendBreakout,
                strategy_score: score,
                strategy_contribution: 0.4,
                alignment: 2,
                aligned_strategies: vec![StrategyKind::TrendBreakout, StrategyKind::MomentumRight],
                alignment_contribution: 0.3,
                regime: RegimeReason {
                    trend_score: 70.0,
                    risk_score: 30.0,
                    combined_score: 60.0,
                    contribution: 0.2,
                },
                rotation: RotationReason {
                    momentum_score: 65.0,
                    rank: Some(3),
                    combined_score: 55.0,
                    contribution: 0.1,
                },
                final_score: score,
                label: SignalLabel::StrongBuy,
                summary: "strong trend".to_string(),
            },
        }
    }

    fn state(scope: &str, date: NaiveDate) -> StrategyStateSnapshot {
        StrategyStateSnapshot {
            date,
            scope: scope.to_string(),
            state: StrategyState::DeRisk,
            state_score: 30.0,
            transition_reason: "risk elevated".to_string(),
            recommended_position_pct: 30.0,
        }
    }

    fn ledger() -> (TempDir, DivergenceLedger) {
        let tmp = TempDir::new().unwrap();
        let ledger = DivergenceLedger::new(tmp.path());
        (tmp, ledger)
    }

    #[test]
    fn canonical_path_layout() {
        let (_tmp, ledger) = ledger();
        let path = ledger
            .record_path("GLOBAL", "000300", d(2026, 7, 20))
            .unwrap();
        let expected = ledger
            .root()
            .join("global")
            .join("000300")
            .join("2026-07-20.json");
        assert_eq!(path, expected);
    }

    #[test]
    fn create_initializes_pending_outcomes_and_unclassified() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        let outcome = ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();
        assert_eq!(outcome, CreateOutcome::Created);

        let record = ledger
            .load(AnalysisScope::Global, "000300", date)
            .unwrap()
            .unwrap();
        assert_eq!(record.schema_version, SCHEMA_VERSION);
        assert_eq!(record.case_key.scope, "GLOBAL");
        assert_eq!(record.case_key.symbol, "000300");
        assert_eq!(record.case_key.observation_date, date);
        assert_eq!(record.observation_mode, ObservationMode::Prospective);
        assert_eq!(record.outcomes.t20, OutcomeStatus::Pending);
        assert_eq!(record.outcomes.t60, OutcomeStatus::Pending);
        assert_eq!(record.outcomes.t120, OutcomeStatus::Pending);
        assert_eq!(
            record.classification.status,
            ClassificationStatus::default()
        );
        assert_eq!(record.classification.category, None);
        assert_eq!(record.classification.notes, None);
    }

    #[test]
    fn round_trip_preserves_observation_facts() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Reconstructed,
                signal("000300", "GLOBAL", date, 91.5),
                state("GLOBAL", date),
            )
            .unwrap();

        let record = ledger
            .load(AnalysisScope::Global, "000300", date)
            .unwrap()
            .unwrap();
        assert_eq!(record.observation_mode, ObservationMode::Reconstructed);
        assert_eq!(record.observation.signal.final_score, 91.5);
        assert!(matches!(
            record.observation.signal.signal_label,
            SignalLabel::StrongBuy
        ));
        assert!(matches!(
            record.observation.signal.reason.best_strategy,
            StrategyKind::TrendBreakout
        ));
        assert_eq!(
            record.observation.strategy_state.state,
            StrategyState::DeRisk
        );
        assert_eq!(record.observation.strategy_state.scope, "GLOBAL");
    }

    #[test]
    fn missing_evolvable_fields_default_on_deserialize() {
        let date = d(2026, 7, 20);
        let record = DivergenceRecord::new(
            AnalysisScope::Global,
            "000300",
            date,
            ObservationMode::Prospective,
            signal("000300", "GLOBAL", date, 85.0),
            state("GLOBAL", date),
        );
        let mut value = serde_json::to_value(&record).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("outcomes");
        object.remove("classification");
        let json = serde_json::to_string(&object).unwrap();

        let restored: DivergenceRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.outcomes.t20, OutcomeStatus::Pending);
        assert_eq!(restored.outcomes.t60, OutcomeStatus::Pending);
        assert_eq!(restored.outcomes.t120, OutcomeStatus::Pending);
        assert_eq!(
            restored.classification.status,
            ClassificationStatus::default()
        );
        assert_eq!(restored.classification.category, None);
        assert_eq!(restored.classification.notes, None);
        assert_eq!(restored.case_key.symbol, "000300");
    }

    #[test]
    fn serialization_is_deterministic_and_duplicate_create_is_noop() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();
        let path = ledger.record_path("GLOBAL", "000300", date).unwrap();
        let before = fs::read(&path).unwrap();

        let record = ledger
            .load(AnalysisScope::Global, "000300", date)
            .unwrap()
            .unwrap();
        assert_eq!(
            serialize_record(&record).unwrap(),
            serialize_record(&record).unwrap()
        );

        let duplicate = ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();
        assert_eq!(duplicate, CreateOutcome::AlreadyExists);

        let after = fs::read(&path).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn unchanged_update_does_not_rewrite_bytes() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();
        let path = ledger.record_path("GLOBAL", "000300", date).unwrap();
        let before = fs::read(&path).unwrap();

        // Fewer than 20 subsequent bars -> still pending, no write.
        let bars = (0..=10)
            .map(|i| bar("000300", add_days(date, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                date,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::StillPending);

        let after = fs::read(&path).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn fill_t20_leaves_t60_and_t120_pending() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        let bars = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();
        match update {
            OutcomeUpdate::Filled { horizon, .. } => assert_eq!(horizon, 20),
            other => panic!("expected Filled, got {other:?}"),
        }

        let record = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        match record.outcomes.t20 {
            OutcomeStatus::Filled {
                maturity_close,
                forward_return,
                ..
            } => {
                assert_eq!(maturity_close, 120.0);
                assert!((forward_return - 0.2).abs() < 1e-9);
            }
            other => panic!("expected Filled, got {other:?}"),
        }
        assert_eq!(record.outcomes.t60, OutcomeStatus::Pending);
        assert_eq!(record.outcomes.t120, OutcomeStatus::Pending);
    }

    #[test]
    fn changed_observation_cannot_overwrite() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 80.0),
                state("GLOBAL", date),
            )
            .unwrap();

        let result = ledger.create(
            AnalysisScope::Global,
            "000300",
            date,
            ObservationMode::Prospective,
            signal("000300", "GLOBAL", date, 95.0),
            state("GLOBAL", date),
        );
        assert!(result.is_err());

        let record = ledger
            .load(AnalysisScope::Global, "000300", date)
            .unwrap()
            .unwrap();
        assert_eq!(record.observation.signal.final_score, 80.0);
    }

    #[test]
    fn classification_is_preserved_through_updates() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        let mut record = DivergenceRecord::new(
            AnalysisScope::Global,
            "000300",
            obs,
            ObservationMode::Prospective,
            signal("000300", "GLOBAL", obs, 85.0),
            state("GLOBAL", obs),
        );
        // Simulate a manually supplied classification using an arbitrary,
        // test-only category string (no project taxonomy is blessed here).
        record.classification = DivergenceClassification {
            status: ClassificationStatus("human_reviewed_v2".to_string()),
            category: Some("test-only-human-category".to_string()),
            notes: Some("test-only human note".to_string()),
        };
        let path = ledger.record_path("GLOBAL", "000300", obs).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serialize_record(&record).unwrap()).unwrap();

        let bars = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();

        let loaded = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        assert_eq!(
            loaded.classification.status,
            ClassificationStatus("human_reviewed_v2".to_string())
        );
        assert_eq!(
            loaded.classification.category.as_deref(),
            Some("test-only-human-category")
        );
        assert_eq!(
            loaded.classification.notes.as_deref(),
            Some("test-only human note")
        );
        assert!(matches!(loaded.outcomes.t20, OutcomeStatus::Filled { .. }));
    }

    #[test]
    fn scope_and_date_are_distinct_cases() {
        let (_tmp, ledger) = ledger();
        let date1 = d(2026, 7, 20);
        let date2 = d(2026, 7, 21);

        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date1,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date1, 80.0),
                state("GLOBAL", date1),
            )
            .unwrap();
        ledger
            .create(
                AnalysisScope::Cn,
                "000300",
                date1,
                ObservationMode::Prospective,
                signal("000300", "CN", date1, 81.0),
                state("CN", date1),
            )
            .unwrap();
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date2,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date2, 82.0),
                state("GLOBAL", date2),
            )
            .unwrap();

        assert_eq!(
            ledger
                .load(AnalysisScope::Global, "000300", date1)
                .unwrap()
                .unwrap()
                .observation
                .signal
                .final_score,
            80.0
        );
        assert_eq!(
            ledger
                .load(AnalysisScope::Cn, "000300", date1)
                .unwrap()
                .unwrap()
                .observation
                .signal
                .final_score,
            81.0
        );
        assert_eq!(
            ledger
                .load(AnalysisScope::Global, "000300", date2)
                .unwrap()
                .unwrap()
                .observation
                .signal
                .final_score,
            82.0
        );

        let global_date1 = ledger.record_path("GLOBAL", "000300", date1).unwrap();
        let cn_date1 = ledger.record_path("CN", "000300", date1).unwrap();
        let global_date2 = ledger.record_path("GLOBAL", "000300", date2).unwrap();
        assert!(global_date1 != cn_date1);
        assert!(global_date1 != global_date2);
        assert!(cn_date1 != global_date2);
    }

    #[test]
    fn unsafe_symbols_are_rejected() {
        for symbol in ["", ".", "..", ".. ", "a. ", "a/b", "a\\b", "a:b"] {
            assert!(validate_symbol(symbol).is_err(), "should reject {symbol:?}");
        }
        assert!(validate_symbol("000300").is_ok());
        assert!(validate_symbol("HSI").is_ok());
        assert!(validate_symbol("a..b").is_ok());
    }

    #[test]
    fn missing_observation_bar_remains_pending_for_backfill() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        // No exact observation bar (bars start strictly after obs).
        let bars = (1..=30)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::StillPending);

        let record = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        assert_eq!(record.outcomes.t20, OutcomeStatus::Pending);
    }

    #[test]
    fn invalid_observation_close_remains_pending_for_backfill() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        // Observation bar exists but close is invalid.
        let mut bars = vec![bar("000300", obs, 0.0)];
        for i in 1..=30 {
            bars.push(bar("000300", add_days(obs, i), 100.0 + i as f64));
        }
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::StillPending);
    }

    #[test]
    fn invalid_maturity_close_remains_pending_for_backfill() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        // Valid observation close and enough strictly-subsequent bars, but the
        // 20th subsequent (maturity) bar has an invalid close. A backfill of
        // that bar could repair the outcome, so it must stay Pending rather
        // than become terminal Unavailable.
        let mut bars = vec![bar("000300", obs, 100.0)];
        for i in 1..=30 {
            let close = if i == 20 { 0.0 } else { 100.0 + i as f64 };
            bars.push(bar("000300", add_days(obs, i), close));
        }
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::StillPending);

        let record = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        assert_eq!(record.outcomes.t20, OutcomeStatus::Pending);
    }

    #[test]
    fn insufficient_future_bars_remain_pending() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        // 5 strictly-subsequent bars, need 20.
        let bars = (0..=5)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::StillPending);

        let record = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        assert_eq!(record.outcomes.t20, OutcomeStatus::Pending);
    }

    #[test]
    fn filled_is_terminal() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        let bars = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars,
            )
            .unwrap();

        // Second fill with different bars must be terminal-preserved.
        let bars2 = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 200.0 + i as f64))
            .collect::<Vec<_>>();
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                obs,
                OutcomeHorizon::T20,
                &bars2,
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::TerminalPreserved);

        let record = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        match record.outcomes.t20 {
            OutcomeStatus::Filled {
                maturity_close,
                forward_return,
                ..
            } => {
                assert_eq!(maturity_close, 120.0);
                assert!((forward_return - 0.2).abs() < 1e-9);
            }
            other => panic!("expected Filled, got {other:?}"),
        }
    }

    #[test]
    fn fill_outcome_on_missing_record_reports_record_missing() {
        let (_tmp, ledger) = ledger();
        let update = ledger
            .fill_outcome(
                AnalysisScope::Global,
                "000300",
                d(2026, 1, 1),
                OutcomeHorizon::T20,
                &[],
            )
            .unwrap();
        assert_eq!(update, OutcomeUpdate::RecordMissing);
    }

    #[test]
    fn signal_analysis_scope_mismatch_is_rejected() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        let mut sig = signal("000300", "GLOBAL", date, 85.0);
        sig.analysis_scope = "CN".to_string();
        let result = ledger.create(
            AnalysisScope::Global,
            "000300",
            date,
            ObservationMode::Prospective,
            sig,
            state("GLOBAL", date),
        );
        let err = result.unwrap_err();
        assert!(err.to_string().contains("analysis_scope"));
    }

    #[test]
    fn signal_regime_basis_scope_mismatch_is_rejected() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        let mut sig = signal("000300", "GLOBAL", date, 85.0);
        sig.regime_basis_scope = "CN".to_string();
        let result = ledger.create(
            AnalysisScope::Global,
            "000300",
            date,
            ObservationMode::Prospective,
            sig,
            state("GLOBAL", date),
        );
        let err = result.unwrap_err();
        assert!(err.to_string().contains("regime_basis_scope"));
    }

    #[test]
    fn classification_inner_fields_default_and_unknown_status_round_trips() {
        // Inner fields default when the classification object is empty.
        let inner_default: DivergenceClassification = serde_json::from_str("{}").unwrap();
        assert_eq!(inner_default.status, ClassificationStatus::default());
        assert_eq!(inner_default.category, None);
        assert_eq!(inner_default.notes, None);

        // A future/manual status round-trips verbatim without defining taxonomy.
        let manual: DivergenceClassification = serde_json::from_str(
            r#"{"status":"some_future_status","category":"arbitrary","notes":"n"}"#,
        )
        .unwrap();
        assert_eq!(manual.status.0, "some_future_status");
        assert_eq!(manual.category.as_deref(), Some("arbitrary"));
        assert_eq!(manual.notes.as_deref(), Some("n"));

        let manual_json = serde_json::to_string(&manual).unwrap();
        assert!(manual_json.contains("some_future_status"));

        // The default classification round-trips losslessly.
        let default_json = serde_json::to_string(&DivergenceClassification::default()).unwrap();
        let roundtrip: DivergenceClassification = serde_json::from_str(&default_json).unwrap();
        assert_eq!(roundtrip.status, ClassificationStatus::default());
        assert_eq!(roundtrip.category, None);
        assert_eq!(roundtrip.notes, None);
    }

    #[test]
    fn candidate_predicate_is_exactly_strongbuy_and_derisk() {
        let date = d(2026, 7, 20);
        let sig = signal("000300", "GLOBAL", date, 85.0);
        let st = state("GLOBAL", date);

        // StrongBuy + DeRisk -> candidate.
        assert!(is_divergence_candidate(&sig, &st));

        // Buy + DeRisk -> not candidate.
        let mut buy = sig.clone();
        buy.signal_label = SignalLabel::Buy;
        assert!(!is_divergence_candidate(&buy, &st));

        // StrongBuy + non-DeRisk -> not candidate.
        let mut full = st.clone();
        full.state = StrategyState::FullTrend;
        assert!(!is_divergence_candidate(&sig, &full));

        // Buy + non-DeRisk -> not candidate.
        assert!(!is_divergence_candidate(&buy, &full));
    }

    #[test]
    fn observation_mode_resolution_rules() {
        let target = d(2026, 7, 20);

        // Explicit + strictly before latest -> Reconstructed.
        assert_eq!(
            resolve_observation_mode(target, Some(d(2026, 7, 21)), true),
            ObservationMode::Reconstructed
        );
        // Explicit but equal to latest -> Prospective.
        assert_eq!(
            resolve_observation_mode(target, Some(d(2026, 7, 20)), true),
            ObservationMode::Prospective
        );
        // Explicit but after latest -> Prospective.
        assert_eq!(
            resolve_observation_mode(target, Some(d(2026, 7, 19)), true),
            ObservationMode::Prospective
        );
        // Not explicit, even when earlier -> Prospective.
        assert_eq!(
            resolve_observation_mode(target, Some(d(2026, 7, 21)), false),
            ObservationMode::Prospective
        );
        // Unknown latest date -> Prospective.
        assert_eq!(
            resolve_observation_mode(target, None, true),
            ObservationMode::Prospective
        );
    }

    #[test]
    fn enumerate_scope_records_is_sorted_by_symbol_then_date() {
        let (_tmp, ledger) = ledger();
        let date1 = d(2026, 7, 20);
        let date2 = d(2026, 7, 21);
        // Insert out of order.
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date2,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date2, 82.0),
                state("GLOBAL", date2),
            )
            .unwrap();
        ledger
            .create(
                AnalysisScope::Global,
                "512480",
                date1,
                ObservationMode::Prospective,
                signal("512480", "GLOBAL", date1, 83.0),
                state("GLOBAL", date1),
            )
            .unwrap();
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date1,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date1, 81.0),
                state("GLOBAL", date1),
            )
            .unwrap();

        let records = ledger
            .enumerate_scope_records(AnalysisScope::Global)
            .unwrap();
        let keys: Vec<(String, NaiveDate)> = records
            .iter()
            .map(|r| (r.case_key.symbol.clone(), r.case_key.observation_date))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("000300".to_string(), date1),
                ("000300".to_string(), date2),
                ("512480".to_string(), date1),
            ]
        );
    }

    #[test]
    fn enumerate_scope_records_rejects_malformed() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();

        let bad_dir = ledger.root().join("global").join("512480");
        fs::create_dir_all(&bad_dir).unwrap();
        fs::write(bad_dir.join("2026-07-20.json"), "{ not valid json").unwrap();

        assert!(ledger
            .enumerate_scope_records(AnalysisScope::Global)
            .is_err());
    }

    #[test]
    fn enumerate_scope_records_empty_for_missing_scope() {
        let (_tmp, ledger) = ledger();
        let records = ledger
            .enumerate_scope_records(AnalysisScope::Global)
            .unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn create_candidates_creates_only_strongbuy_derisk_sorted() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        let st = state("GLOBAL", date);

        // Signals out of order, with a non-candidate Buy and a non-candidate
        // (StrongBuy would be filtered only if state is DeRisk; here Buy is the
        // non-candidate label).
        let mut buy = signal("000001", "GLOBAL", date, 70.0);
        buy.signal_label = SignalLabel::Buy;
        let signals = vec![
            signal("512480", "GLOBAL", date, 83.0),
            buy,
            signal("000300", "GLOBAL", date, 85.0),
        ];

        let mut summary = DivergenceLedgerUpdateSummary::default();
        create_candidates(
            &ledger,
            AnalysisScope::Global,
            date,
            ObservationMode::Prospective,
            &signals,
            &st,
            &mut summary,
        )
        .unwrap();

        assert_eq!(summary.cases_created, 2);
        assert_eq!(summary.cases_existing, 0);
        assert_eq!(summary.cases_conflicted, 0);
        assert_eq!(summary.created_paths.len(), 2);
        // Sorted: 000300 before 512480.
        let paths = summary.created_paths.clone();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
        assert_eq!(summary.created_paths[0], "global/000300/2026-07-20.json");
        assert_eq!(summary.created_paths[1], "global/512480/2026-07-20.json");
    }

    #[test]
    fn create_candidates_creates_nothing_for_non_derisk_state() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        // Prior-day-only DeRisk is not acceptable: exact-date state is FullTrend.
        let mut st = state("GLOBAL", date);
        st.state = StrategyState::FullTrend;
        let signals = vec![signal("000300", "GLOBAL", date, 85.0)];

        let mut summary = DivergenceLedgerUpdateSummary::default();
        create_candidates(
            &ledger,
            AnalysisScope::Global,
            date,
            ObservationMode::Prospective,
            &signals,
            &st,
            &mut summary,
        )
        .unwrap();

        assert_eq!(summary.cases_created, 0);
        assert!(summary.created_paths.is_empty());
        assert!(ledger
            .enumerate_scope_records(AnalysisScope::Global)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn create_candidates_is_deterministic_across_identical_ledgers() {
        let date = d(2026, 7, 20);
        let st = state("GLOBAL", date);
        let signals = vec![
            signal("512480", "GLOBAL", date, 83.0),
            signal("000300", "GLOBAL", date, 85.0),
        ];

        let (_t1, l1) = ledger();
        let (_t2, l2) = ledger();
        let mut s1 = DivergenceLedgerUpdateSummary::default();
        let mut s2 = DivergenceLedgerUpdateSummary::default();
        create_candidates(
            &l1,
            AnalysisScope::Global,
            date,
            ObservationMode::Prospective,
            &signals,
            &st,
            &mut s1,
        )
        .unwrap();
        create_candidates(
            &l2,
            AnalysisScope::Global,
            date,
            ObservationMode::Prospective,
            &signals,
            &st,
            &mut s2,
        )
        .unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn create_preserving_reports_conflict_without_mutation() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 80.0),
                state("GLOBAL", date),
            )
            .unwrap();
        let path = ledger.record_path("GLOBAL", "000300", date).unwrap();
        let before = fs::read(&path).unwrap();

        // Conflicting immutable facts (different signal score).
        let outcome = ledger
            .create_preserving(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 95.0),
                state("GLOBAL", date),
            )
            .unwrap();
        assert_eq!(outcome, CreateOutcome::Conflict);

        let after = fs::read(&path).unwrap();
        assert_eq!(before, after);
        let record = ledger
            .load(AnalysisScope::Global, "000300", date)
            .unwrap()
            .unwrap();
        assert_eq!(record.observation.signal.final_score, 80.0);
    }

    #[test]
    fn create_preserving_identical_is_already_exists() {
        let (_tmp, ledger) = ledger();
        let date = d(2026, 7, 20);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();
        let outcome = ledger
            .create_preserving(
                AnalysisScope::Global,
                "000300",
                date,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", date, 85.0),
                state("GLOBAL", date),
            )
            .unwrap();
        assert_eq!(outcome, CreateOutcome::AlreadyExists);
    }

    #[test]
    fn sweep_fills_t20_keeps_others_pending() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        // 21 bars: obs + 20 strictly-subsequent.
        let bars = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let mut by_symbol = HashMap::new();
        by_symbol.insert("000300".to_string(), bars);

        let summary = sweep_scope_outcomes(&ledger, AnalysisScope::Global, &by_symbol).unwrap();
        assert_eq!(summary.records_scanned, 1);
        assert_eq!(summary.horizons_filled, 1);
        assert_eq!(summary.horizons_pending, 2);
        assert_eq!(summary.horizons_unavailable, 0);
        assert_eq!(summary.horizons_terminal, 0);

        let record = ledger
            .load(AnalysisScope::Global, "000300", obs)
            .unwrap()
            .unwrap();
        assert!(matches!(record.outcomes.t20, OutcomeStatus::Filled { .. }));
        assert_eq!(record.outcomes.t60, OutcomeStatus::Pending);
        assert_eq!(record.outcomes.t120, OutcomeStatus::Pending);
    }

    #[test]
    fn sweep_preserves_terminal_outcomes() {
        let (_tmp, ledger) = ledger();
        let obs = d(2026, 1, 1);
        ledger
            .create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();

        let bars = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let mut by_symbol = HashMap::new();
        by_symbol.insert("000300".to_string(), bars);

        let first = sweep_scope_outcomes(&ledger, AnalysisScope::Global, &by_symbol).unwrap();
        assert_eq!(first.horizons_filled, 1);

        // Second sweep: T20 already terminal, untouched; T60/T120 still pending.
        let second = sweep_scope_outcomes(&ledger, AnalysisScope::Global, &by_symbol).unwrap();
        assert_eq!(second.horizons_filled, 0);
        assert_eq!(second.horizons_terminal, 1);
        assert_eq!(second.horizons_pending, 2);
        assert_eq!(second.horizons_unavailable, 0);
    }

    #[test]
    fn sweep_is_deterministic_across_identical_ledgers() {
        let obs = d(2026, 1, 1);
        let bars = (0..=20)
            .map(|i| bar("000300", add_days(obs, i), 100.0 + i as f64))
            .collect::<Vec<_>>();
        let mut by_symbol = HashMap::new();
        by_symbol.insert("000300".to_string(), bars.clone());

        let (_t1, l1) = ledger();
        let (_t2, l2) = ledger();
        for l in [&l1, &l2] {
            l.create(
                AnalysisScope::Global,
                "000300",
                obs,
                ObservationMode::Prospective,
                signal("000300", "GLOBAL", obs, 85.0),
                state("GLOBAL", obs),
            )
            .unwrap();
        }
        let s1 = sweep_scope_outcomes(&l1, AnalysisScope::Global, &by_symbol).unwrap();
        let s2 = sweep_scope_outcomes(&l2, AnalysisScope::Global, &by_symbol).unwrap();
        assert_eq!(s1, s2);
    }
}
