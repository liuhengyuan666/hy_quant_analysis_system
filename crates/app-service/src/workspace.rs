//! Workspace Research Asset management.
//!
//! The workspace is the durable filesystem store for Research Assets: Evidence,
//! Research Snapshots, and their registry indexes. It is intentionally separate
//! from rendered reports (`reports/`) and operational logs (`shadow-production/`).
//!
//! This module does not contain business logic for computing evidence; it only
//! owns the serialization, directory layout, and index management of already
//! computed Research Assets.
//!
//! Asset identity and lifecycle are governed by ADR-081 and ADR-080:
//! - All assets use a unified `RA-XXXXXX` identifier (`ResearchAssetId`).
//! - All assets share the same lifecycle vocabulary (`ResearchAssetLifecycle`).
//! - The asset `kind` is stored in metadata, not encoded in the identifier.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use core_domain::research::attribution::Evidence;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Directory layout for the workspace.
#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub evidence: PathBuf,
    pub evidence_replay: PathBuf,
    pub evidence_calibration: PathBuf,
    pub evidence_attribution: PathBuf,
    pub evidence_validation: PathBuf,
    pub research_snapshots: PathBuf,
    pub registry: PathBuf,
}

impl WorkspacePaths {
    /// Default workspace location relative to the project root.
    pub fn default_workspace() -> PathBuf {
        PathBuf::from("workspace")
    }

    pub fn from_root<P: AsRef<Path>>(root: P) -> Self {
        let root = root.as_ref().to_path_buf();
        let evidence = root.join("evidence");
        Self {
            evidence_replay: evidence.join("replay"),
            evidence_calibration: evidence.join("calibration"),
            evidence_attribution: evidence.join("attribution"),
            evidence_validation: evidence.join("validation"),
            research_snapshots: root.join("research-snapshots"),
            registry: root.join("registry"),
            evidence,
            root,
        }
    }

    /// Create all directories if they do not exist.
    pub fn ensure_directories(&self) -> Result<()> {
        for dir in [
            &self.root,
            &self.evidence,
            &self.evidence_replay,
            &self.evidence_calibration,
            &self.evidence_attribution,
            &self.evidence_validation,
            &self.research_snapshots,
            &self.registry,
        ] {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create workspace directory: {}", dir.display()))?;
        }
        Ok(())
    }
}

impl Default for WorkspacePaths {
    fn default() -> Self {
        Self::from_root(Self::default_workspace())
    }
}

/// Unified Research Asset identifier.
///
/// Format on disk: `RA-{sequence:06}` (e.g. `RA-000001`).
/// The sequence is unique across all asset kinds in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResearchAssetId {
    pub sequence: u64,
}

impl ResearchAssetId {
    pub fn new(sequence: u64) -> Self {
        Self { sequence }
    }

    pub fn as_string(&self) -> String {
        format!("RA-{:06}", self.sequence)
    }
}

impl std::fmt::Display for ResearchAssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

/// Asset kind. Stored in metadata, not encoded in the identifier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Evidence,
    Snapshot,
    Hypothesis,
    Validation,
    Knowledge,
}

impl std::fmt::Display for AssetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evidence => write!(f, "evidence"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Hypothesis => write!(f, "hypothesis"),
            Self::Validation => write!(f, "validation"),
            Self::Knowledge => write!(f, "knowledge"),
        }
    }
}

/// Unified Research Asset lifecycle.
///
/// All assets in the workspace share these five states (ADR-080).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchAssetLifecycle {
    Draft,
    Verified,
    Published,
    Superseded,
    Archived,
}

impl std::fmt::Display for ResearchAssetLifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Verified => write!(f, "verified"),
            Self::Published => write!(f, "published"),
            Self::Superseded => write!(f, "superseded"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

/// Metadata envelope shared by all Research Assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: ResearchAssetId,
    pub kind: AssetKind,
    pub version: u32,
    pub status: ResearchAssetLifecycle,
    pub created_at: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub producer: String,
    pub source: String,
}

/// Evidence-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceMetadata {
    #[serde(flatten)]
    pub asset: AssetMetadata,
    pub condition: String,
    pub scope: String,
    pub horizon: usize,
    pub history_window: String,
}

/// On-disk layout for an evidence asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceAsset {
    pub metadata: EvidenceMetadata,
    pub evidence: Evidence,
}

/// Entry in the evidence registry index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceIndexEntry {
    pub id: ResearchAssetId,
    pub version: u32,
    pub condition: String,
    pub scope: String,
    pub horizon: usize,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub path: String,
}

/// Registry index for evidence assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceIndex {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub entries: Vec<EvidenceIndexEntry>,
}

impl EvidenceIndex {
    pub fn empty() -> Self {
        Self {
            version: 1,
            generated_at: Utc::now(),
            entries: Vec::new(),
        }
    }
}

/// Reference to an evidence asset from within another asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub id: ResearchAssetId,
    pub version: u32,
    pub role: String,
}

/// Snapshot-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    #[serde(flatten)]
    pub asset: AssetMetadata,
    pub scope: String,
    pub date: NaiveDate,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Entry in the snapshot registry index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotIndexEntry {
    pub id: ResearchAssetId,
    pub version: u32,
    pub scope: String,
    pub date: NaiveDate,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub path: String,
}

/// Registry index for research snapshot assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotIndex {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub entries: Vec<SnapshotIndexEntry>,
}

impl SnapshotIndex {
    pub fn empty() -> Self {
        Self {
            version: 1,
            generated_at: Utc::now(),
            entries: Vec::new(),
        }
    }
}

/// Counter state for the workspace-wide asset sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSequence {
    pub next_sequence: u64,
}

impl AssetSequence {
    pub fn new() -> Self {
        Self { next_sequence: 1 }
    }
}

/// Workspace manager for Research Assets.
#[derive(Debug, Clone)]
pub struct WorkspaceManager {
    pub paths: WorkspacePaths,
}

impl WorkspaceManager {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let paths = WorkspacePaths::from_root(root);
        paths.ensure_directories()?;
        Ok(Self { paths })
    }

    pub fn default_workspace() -> Result<Self> {
        Self::new(WorkspacePaths::default_workspace())
    }

    /// Load or create the workspace-wide asset sequence counter.
    fn load_sequence(&self) -> Result<AssetSequence> {
        let path = self.paths.registry.join("asset-sequence.json");
        if !path.exists() {
            return Ok(AssetSequence::new());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read asset sequence: {}", path.display()))?;
        let seq: AssetSequence = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse asset sequence: {}", path.display()))?;
        Ok(seq)
    }

    /// Save the workspace-wide asset sequence counter.
    fn save_sequence(&self, seq: &AssetSequence) -> Result<()> {
        let path = self.paths.registry.join("asset-sequence.json");
        let content = serde_json::to_string_pretty(seq)
            .context("Failed to serialize asset sequence")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write asset sequence: {}", path.display()))?;
        Ok(())
    }

    /// Allocate the next unified Research Asset ID.
    fn next_asset_id(&self) -> Result<ResearchAssetId> {
        let mut seq = self.load_sequence()?;
        let id = ResearchAssetId::new(seq.next_sequence);
        seq.next_sequence += 1;
        self.save_sequence(&seq)?;
        Ok(id)
    }

    /// Load or create the evidence registry index.
    pub fn load_evidence_index(&self) -> Result<EvidenceIndex> {
        let path = self.paths.registry.join("evidence-index.json");
        if !path.exists() {
            return Ok(EvidenceIndex::empty());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read evidence index: {}", path.display()))?;
        let index: EvidenceIndex = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse evidence index: {}", path.display()))?;
        Ok(index)
    }

    /// Save the evidence registry index.
    pub fn save_evidence_index(&self, index: &EvidenceIndex) -> Result<()> {
        let path = self.paths.registry.join("evidence-index.json");
        let content = serde_json::to_string_pretty(index)
            .context("Failed to serialize evidence index")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write evidence index: {}", path.display()))?;
        Ok(())
    }

    /// Determine the evidence subdirectory based on the source category.
    fn evidence_dir_for_source(&self, source: &str) -> PathBuf {
        match source {
            "replay" => self.paths.evidence_replay.clone(),
            "calibration" => self.paths.evidence_calibration.clone(),
            "attribution" => self.paths.evidence_attribution.clone(),
            "validation" => self.paths.evidence_validation.clone(),
            _ => self.paths.evidence.clone(),
        }
    }

    /// Write an evidence asset to the workspace and append it to the registry.
    ///
    /// The evidence body is written as `workspace/evidence/<source>/RA-XXXXX/body.json`.
    /// The metadata is written as `workspace/evidence/<source>/RA-XXXXX/metadata.json`.
    /// The registry index is updated atomically (rewrite).
    pub fn write_evidence(
        &self,
        evidence: &Evidence,
        condition: &str,
        scope: core_domain::AnalysisScope,
        horizon: usize,
        source: &str,
        status: ResearchAssetLifecycle,
    ) -> Result<ResearchAssetId> {
        let id = self.next_asset_id()?;
        let base_dir = self.evidence_dir_for_source(source);
        let dir = base_dir.join(id.as_string());
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create evidence directory: {}", dir.display()))?;

        let asset_meta = AssetMetadata {
            id: id.clone(),
            kind: AssetKind::Evidence,
            version: 1,
            status,
            created_at: Utc::now(),
            generated_at: Utc::now(),
            producer: "app-service::workspace".to_string(),
            source: source.to_string(),
        };
        let metadata = EvidenceMetadata {
            asset: asset_meta,
            condition: condition.to_string(),
            scope: scope.as_str().to_string(),
            horizon,
            history_window: evidence.history_window.clone(),
        };

        let asset = EvidenceAsset {
            metadata: metadata.clone(),
            evidence: evidence.clone(),
        };

        let body_path = dir.join("body.json");
        let body_content = serde_json::to_string_pretty(&asset)
            .context("Failed to serialize evidence asset")?;
        fs::write(&body_path, body_content)
            .with_context(|| format!("Failed to write evidence body: {}", body_path.display()))?;

        let mut index = self.load_evidence_index()?;
        index.entries.push(EvidenceIndexEntry {
            id: id.clone(),
            version: metadata.asset.version,
            condition: metadata.condition.clone(),
            scope: metadata.scope.clone(),
            horizon: metadata.horizon,
            status: metadata.asset.status.to_string(),
            created_at: metadata.asset.created_at,
            path: format!("workspace/evidence/{}/{}/body.json", source, id.as_string()),
        });
        index.generated_at = Utc::now();
        self.save_evidence_index(&index)?;

        Ok(id)
    }

    /// Read a single evidence asset by ID.
    ///
    /// Searches all source subdirectories for the ID.
    pub fn read_evidence(&self, id: &ResearchAssetId) -> Result<EvidenceAsset> {
        for source in ["replay", "calibration", "attribution", "validation"] {
            let path = self
                .evidence_dir_for_source(source)
                .join(id.as_string())
                .join("body.json");
            if path.exists() {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read evidence asset: {}", path.display()))?;
                let asset: EvidenceAsset = serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse evidence asset: {}", path.display()))?;
                return Ok(asset);
            }
        }
        anyhow::bail!("Evidence asset not found: {}", id.as_string())
    }

    /// Rebuild the evidence index from the filesystem.
    ///
    /// This is useful if the index becomes out of sync with the evidence directories.
    pub fn rebuild_evidence_index(&self) -> Result<EvidenceIndex> {
        let mut entries = Vec::new();
        for source in ["replay", "calibration", "attribution", "validation"] {
            let base_dir = self.evidence_dir_for_source(source);
            if !base_dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&base_dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let body_path = entry.path().join("body.json");
                if !body_path.exists() {
                    continue;
                }
                let content = fs::read_to_string(&body_path)
                    .with_context(|| format!("Failed to read evidence body: {}", body_path.display()))?;
                let asset: EvidenceAsset = serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse evidence body: {}", body_path.display()))?;
                entries.push(EvidenceIndexEntry {
                    id: asset.metadata.asset.id.clone(),
                    version: asset.metadata.asset.version,
                    condition: asset.metadata.condition,
                    scope: asset.metadata.scope,
                    horizon: asset.metadata.horizon,
                    status: asset.metadata.asset.status.to_string(),
                    created_at: asset.metadata.asset.created_at,
                    path: format!(
                        "workspace/evidence/{}/{}/body.json",
                        source,
                        entry.file_name().to_string_lossy()
                    ),
                });
            }
        }
        entries.sort_by(|a, b| a.id.sequence.cmp(&b.id.sequence));
        let index = EvidenceIndex {
            version: 1,
            generated_at: Utc::now(),
            entries,
        };
        self.save_evidence_index(&index)?;
        Ok(index)
    }

    /// Load or create the snapshot registry index.
    pub fn load_snapshot_index(&self) -> Result<SnapshotIndex> {
        let path = self.paths.registry.join("snapshot-index.json");
        if !path.exists() {
            return Ok(SnapshotIndex::empty());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read snapshot index: {}", path.display()))?;
        let index: SnapshotIndex = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse snapshot index: {}", path.display()))?;
        Ok(index)
    }

    /// Save the snapshot registry index.
    pub fn save_snapshot_index(&self, index: &SnapshotIndex) -> Result<()> {
        let path = self.paths.registry.join("snapshot-index.json");
        let content = serde_json::to_string_pretty(index)
            .context("Failed to serialize snapshot index")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write snapshot index: {}", path.display()))?;
        Ok(())
    }

    /// Write a snapshot asset to the workspace and append it to the registry.
    ///
    /// The snapshot body is written as `workspace/research-snapshots/RA-XXXXX/body.json`.
    /// The metadata is written as `workspace/research-snapshots/RA-XXXXX/metadata.json`.
    /// The registry index is updated atomically (rewrite).
    pub fn write_snapshot<T: Serialize>(
        &self,
        body: &T,
        scope: core_domain::AnalysisScope,
        date: NaiveDate,
        status: ResearchAssetLifecycle,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<ResearchAssetId> {
        let id = self.next_asset_id()?;
        let dir = self.paths.research_snapshots.join(id.as_string());
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create snapshot directory: {}", dir.display()))?;

        let asset_meta = AssetMetadata {
            id: id.clone(),
            kind: AssetKind::Snapshot,
            version: 1,
            status,
            created_at: Utc::now(),
            generated_at: Utc::now(),
            producer: "app-service::workspace".to_string(),
            source: "snapshot".to_string(),
        };
        let metadata = SnapshotMetadata {
            asset: asset_meta,
            scope: scope.as_str().to_string(),
            date,
            evidence_refs,
        };

        let metadata_path = dir.join("metadata.json");
        let metadata_content = serde_json::to_string_pretty(&metadata)
            .context("Failed to serialize snapshot metadata")?;
        fs::write(&metadata_path, metadata_content)
            .with_context(|| format!("Failed to write snapshot metadata: {}", metadata_path.display()))?;

        let body_path = dir.join("body.json");
        let body_content = serde_json::to_string_pretty(body)
            .context("Failed to serialize snapshot body")?;
        fs::write(&body_path, body_content)
            .with_context(|| format!("Failed to write snapshot body: {}", body_path.display()))?;

        let mut index = self.load_snapshot_index()?;
        index.entries.push(SnapshotIndexEntry {
            id: id.clone(),
            version: metadata.asset.version,
            scope: metadata.scope.clone(),
            date: metadata.date,
            status: metadata.asset.status.to_string(),
            created_at: metadata.asset.created_at,
            path: format!("workspace/research-snapshots/{}/body.json", id.as_string()),
        });
        index.generated_at = Utc::now();
        self.save_snapshot_index(&index)?;

        Ok(id)
    }

    /// Read a snapshot metadata header by ID.
    pub fn read_snapshot_metadata(&self, id: &ResearchAssetId) -> Result<SnapshotMetadata> {
        let path = self
            .paths
            .research_snapshots
            .join(id.as_string())
            .join("metadata.json");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read snapshot metadata: {}", path.display()))?;
        let metadata: SnapshotMetadata = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse snapshot metadata: {}", path.display()))?;
        Ok(metadata)
    }

    /// Rebuild the snapshot index from the filesystem.
    pub fn rebuild_snapshot_index(&self) -> Result<SnapshotIndex> {
        let mut entries = Vec::new();
        if self.paths.research_snapshots.exists() {
            for entry in fs::read_dir(&self.paths.research_snapshots)? {
                let entry = entry?;
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let metadata_path = entry.path().join("metadata.json");
                if !metadata_path.exists() {
                    continue;
                }
                let content = fs::read_to_string(&metadata_path)
                    .with_context(|| format!("Failed to read snapshot metadata: {}", metadata_path.display()))?;
                let metadata: SnapshotMetadata = serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse snapshot metadata: {}", metadata_path.display()))?;
                entries.push(SnapshotIndexEntry {
                    id: metadata.asset.id.clone(),
                    version: metadata.asset.version,
                    scope: metadata.scope,
                    date: metadata.date,
                    status: metadata.asset.status.to_string(),
                    created_at: metadata.asset.created_at,
                    path: format!(
                        "workspace/research-snapshots/{}/body.json",
                        entry.file_name().to_string_lossy()
                    ),
                });
            }
        }
        entries.sort_by(|a, b| a.id.sequence.cmp(&b.id.sequence));
        let index = SnapshotIndex {
            version: 1,
            generated_at: Utc::now(),
            entries,
        };
        self.save_snapshot_index(&index)?;
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use core_domain::research::attribution::Evidence;
    use tempfile::TempDir;

    fn test_evidence() -> Evidence {
        Evidence::from_facts(
            vec![NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()],
            vec![0.05],
            vec![0.02],
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        )
    }

    #[test]
    fn research_asset_id_formatting() {
        let id = ResearchAssetId::new(42);
        assert_eq!(id.as_string(), "RA-000042");
        assert_eq!(id.to_string(), "RA-000042");
    }

    #[test]
    fn workspace_paths_create_all_dirs() {
        let tmp = TempDir::new().unwrap();
        let paths = WorkspacePaths::from_root(tmp.path());
        paths.ensure_directories().unwrap();
        assert!(paths.evidence_replay.exists());
        assert!(paths.registry.exists());
    }

    #[test]
    fn write_and_read_evidence_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let manager = WorkspaceManager::new(tmp.path()).unwrap();
        let evidence = test_evidence();
        let id = manager
            .write_evidence(
                &evidence,
                "srd-strong",
                core_domain::AnalysisScope::Global,
                20,
                "replay",
                ResearchAssetLifecycle::Draft,
            )
            .unwrap();
        assert_eq!(id.as_string(), "RA-000001");

        let asset = manager.read_evidence(&id).unwrap();
        assert_eq!(asset.metadata.condition, "srd-strong");
        assert_eq!(asset.metadata.asset.kind, AssetKind::Evidence);
        assert_eq!(asset.evidence.occurrences, 1);

        let index = manager.load_evidence_index().unwrap();
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].id.as_string(), "RA-000001");
    }

    #[test]
    fn rebuild_evidence_index_from_filesystem() {
        let tmp = TempDir::new().unwrap();
        let manager = WorkspaceManager::new(tmp.path()).unwrap();
        let evidence = test_evidence();
        let id = manager
            .write_evidence(
                &evidence,
                "stretch-extreme-crowding-momentum",
                core_domain::AnalysisScope::Cn,
                60,
                "replay",
                ResearchAssetLifecycle::Verified,
            )
            .unwrap();

        // Remove the index file and rebuild from directories.
        fs::remove_file(manager.paths.registry.join("evidence-index.json")).unwrap();
        let index = manager.rebuild_evidence_index().unwrap();
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].id, id);
    }

    #[test]
    fn write_and_read_snapshot_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let manager = WorkspaceManager::new(tmp.path()).unwrap();
        let body = serde_json::json!({
            "observation": { "state": "neutral", "signal_summary": "3 bullish / 1 strong-buy" },
            "evidence_refs": []
        });
        let evidence_refs = vec![EvidenceRef {
            id: ResearchAssetId::new(1),
            version: 1,
            role: "supporting".to_string(),
        }];
        let id = manager
            .write_snapshot(
                &body,
                core_domain::AnalysisScope::Global,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                ResearchAssetLifecycle::Published,
                evidence_refs,
            )
            .unwrap();
        assert!(id.as_string().starts_with("RA-"));

        let metadata = manager.read_snapshot_metadata(&id).unwrap();
        assert_eq!(metadata.scope, "GLOBAL");
        assert_eq!(metadata.asset.kind, AssetKind::Snapshot);
        assert_eq!(metadata.asset.status, ResearchAssetLifecycle::Published);
        assert_eq!(metadata.evidence_refs.len(), 1);

        let index = manager.load_snapshot_index().unwrap();
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].id.as_string(), id.as_string());
    }

    #[test]
    fn rebuild_snapshot_index_from_filesystem() {
        let tmp = TempDir::new().unwrap();
        let manager = WorkspaceManager::new(tmp.path()).unwrap();
        let body = serde_json::json!({ "placeholder": true });
        let id = manager
            .write_snapshot(
                &body,
                core_domain::AnalysisScope::Hk,
                NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
                ResearchAssetLifecycle::Draft,
                vec![],
            )
            .unwrap();

        fs::remove_file(manager.paths.registry.join("snapshot-index.json")).unwrap();
        let index = manager.rebuild_snapshot_index().unwrap();
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].id, id);
    }
}
