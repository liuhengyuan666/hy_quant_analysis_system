# ADR-079: Research Snapshot — Reproducible Research Asset

**Status:** Accepted

**Tags:** v8, research-snapshot, workspace, reproducibility, evidence, adr-078, task-108

## Context

V7 established a read-only Research Layer: Observation (V6), Market Evolution (V7.1), Historical Evidence (V7.2), and Research Synthesis / Consensus (V7.3). ADR-078 added an Attribution Layer to explain why the same signal or condition performs differently across regimes. The common pattern across all of these layers is that they produce **interpretations** from raw market data and stored evidence.

A critical gap remains: there is no canonical, durable artifact that captures a complete research view on a given date. Consumers (CLI, Desktop, LLM, reports) today reconstruct the view on demand from dashboards and snapshots. This is fragile for three reasons:

1. **Reproducibility**: a research conclusion produced today cannot be replayed tomorrow if the underlying data or code has changed.
2. **Attribution**: ADR-078 needs a stable target to attribute against. Without a frozen snapshot, every attribution re-runs the full semantic pipeline and may silently drift.
3. **Workspace discipline**: historical replay, calibration, and validation are producing scattered artifacts under `reports/` and `shadow-production/`. They are research outputs, not durable research assets.

V8 therefore introduces the **Research Snapshot** as the canonical reproducible research asset.

## Decision

Introduce a `ResearchSnapshot` durable asset stored in the workspace. It is a point-in-time record of the complete research view for a specific `scope` and `date`, including references to observations, evolution, evidence, and consensus. It is versioned, provenance-tracked, and reference-based (not embedded).

The snapshot is a **Research Asset** in the same sense as Evidence. It belongs to the workspace, not to `reports/`.

## Final Design

### Research Snapshot is a Research Asset

```text
workspace/
├── evidence/
│   └── replay/
│       └── RA-XXXXX/          # immutable facts + derived statistics
│           ├── body.json
│           └── metadata.json
├── research-snapshots/
│   └── RA-XXXXX/              # canonical snapshot body
│       ├── body.json
│       └── metadata.json
└── registry/
    ├── asset-index.json       # unified registry (ADR-081)
    ├── evidence-index.json    # legacy index, maintained until migration
    └── snapshot-index.json    # legacy index, maintained until migration
```

Only durable, reproducible artifacts live in the workspace. Transient cache, logs, or build outputs are excluded.

Asset identifiers use the unified `RA-XXXXXX` scheme defined in ADR-081. The `kind` (Evidence, Snapshot, etc.) is stored in metadata, not in the identifier.

### Snapshot Body (`ResearchSnapshot`)

```rust
pub struct ResearchSnapshot {
    pub id: ResearchAssetId,            // stable unified ID, e.g. "RA-000001" (ADR-081)
    pub kind: AssetKind,                // Snapshot
    pub version: u32,                   // starts at 1, increments on semantic change
    pub scope: AnalysisScope,
    pub date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub provenance: Provenance,
    pub observation: Observation,       // market state, signal, breadth, liquidity
    pub evolution: Evolution,           // confirmation, recovery, analogues
    pub evidence_refs: Vec<EvidenceRef>, // references to Evidence assets, not embedded
    pub consensus: Option<Consensus>,   // optional, may be absent if not yet synthesized
}
```

`Observation` and `Evolution` are embedded because they are interpretations of the market state on that date and are small enough to be self-contained. `Evidence` is **not** embedded; it is referenced by stable ID and version so the snapshot remains lightweight and the evidence body remains the single source of truth.

### Evidence Reference (`EvidenceRef`)

```rust
pub struct EvidenceRef {
    pub id: ResearchAssetId,       // unified asset ID, e.g. "RA-000001" (ADR-081)
    pub version: u32,
    pub role: EvidenceRole,        // e.g. Supporting, Contradicting, Context
}
```

Embedding evidence would duplicate the historical facts and make drift detection impossible. References ensure that the same evidence asset can support multiple snapshots and that evidence lifecycle changes (ADR-080) are reflected everywhere that consumes it.

### Provenance (`Provenance`)

Every snapshot must carry provenance sufficient to reproduce the view from source data and configuration.

```rust
pub struct Provenance {
    pub generated_at: DateTime<Utc>,
    pub producer: String,              // e.g. "app-service::research_snapshot"
    pub git_revision: String,          // short SHA of the running code
    pub research_version: String,    // e.g. "7.3.0"
    pub semantic_version: u32,         // version of the semantic model / ADR set
    pub baseline_version: u32,       // calibration baseline version at time of snapshot
    pub dataset_hash: String,        // hash of the input dataset used to build the snapshot
    pub config_hash: String,         // hash of relevant config (universe, thresholds, weights)
}
```

`dataset_hash` and `config_hash` are the minimum required for reproducibility. The other fields are metadata that make manual audit easier.

### Reproducible View Principle

> A Research Snapshot must be **reproducible**: given the same `scope`, `date`, `semantic_version`, `baseline_version`, `dataset_hash`, and `config_hash`, a compliant implementation must produce the same `observation`, `evolution`, and `evidence_refs`.

This is a deterministic contract, not a guarantee of truth. Reproducibility makes it possible to:

- Compare a snapshot against a later re-run to detect drift.
- Replay the exact research view in Historical Replay or Shadow Production.
- Validate ADR-078 attributions against a frozen target.

### Snapshot Producer (`app-service`)

`app-service` owns the snapshot producer. The public API is:

```rust
impl AppContext {
    /// Build a ResearchSnapshot for the given scope and date from the current dataset.
    pub fn build_research_snapshot(
        &self,
        scope: AnalysisScope,
        date: NaiveDate,
    ) -> Result<ResearchSnapshot>;

    /// Save a ResearchSnapshot to the workspace.
    pub fn save_research_snapshot(&self, snapshot: &ResearchSnapshot) -> Result<()>;

    /// Load a ResearchSnapshot from the workspace by ID.
    pub fn load_research_snapshot(&self, id: &str) -> Result<ResearchSnapshot>;
}
```

The producer must not embed evidence. It must fetch or compute evidence assets, store them in `workspace/evidence/`, and include only `EvidenceRef` values in the snapshot body.

### Snapshot Registry

`workspace/registry/snapshot-index.json` is the mutable catalog of snapshots. Each entry contains:

```json
{
  "id": "SN-000001",
  "version": 1,
  "scope": "Global",
  "date": "2026-07-09",
  "path": "workspace/research-snapshots/SN-000001/snapshot.json",
  "status": "active",
  "provenance": { "generated_at": "...", "git_revision": "..." }
}
```

The registry is append-preferred: new versions are appended, old versions are marked `superseded` rather than deleted. Superseded snapshots remain available for historical replay and audit.

### Lifecycle Status

A snapshot follows the unified Research Asset lifecycle defined in ADR-080:

- `draft`: produced by a replay or calibration run, not yet reviewed.
- `verified`: reviewed and reproducible from provenance; internally trusted.
- `published`: accepted as the canonical view for that date.
- `superseded`: a newer published snapshot exists for the same scope+date.
- `archived`: retained for audit but no longer used for active decisions.

Only `published` snapshots should be consumed by downstream reports or LLM analysis.

### Relationship to Existing Layers

```text
ResearchContext          # canonical semantic model (ADR-068) — embedded as Observation
MarketFingerprint        # canonical historical representation (ADR-071) — embedded in Evolution
Evidence                 # historical facts (ADR-078) — referenced by EvidenceRef
Consensus                # synthesized view (V7.3) — embedded or referenced
↓
ResearchSnapshot         # durable, reproducible, point-in-time research asset
```

ResearchSnapshot is the durable envelope. It does not replace `ResearchContext`, `MarketFingerprint`, `Evidence`, or `Consensus`; it references or embeds them according to their ownership rules.

## Architecture Rules

> **Rule-01: Evidence is referenced, never embedded.**
> Snapshot bodies must use `EvidenceRef { id, version }`. This keeps evidence immutable, versioned, and reusable across multiple snapshots.

> **Rule-02: Provenance is mandatory and must include dataset and config hashes.**
> Without these two hashes, a snapshot is not reproducible.

> **Rule-03: Snapshots live in `workspace/research-snapshots/`, not in `reports/` or `shadow-production/`.**
> `reports/` contains rendered artifacts for human consumption. `shadow-production/` contains operational logs and replay reports. The workspace owns durable research assets.

> **Rule-04: Snapshot registry is append-preferred, not append-only.**
> Old versions can be marked `superseded` or `archived`; they are not deleted. This supports audit and drift detection.

> **Rule-05: Snapshot IDs are stable and scoped to the workspace.**
> IDs are assigned by the workspace registry and remain constant across re-runs. Re-running the same date does not produce a new ID unless the producer explicitly creates a new snapshot.

> **Rule-06: Only `published` snapshots are consumed by downstream consumers.**
> CLI, Desktop, LLM, and reports must prefer the `published` snapshot for a given scope+date. Draft and verified snapshots are for review and calibration only.

> **Rule-07: Snapshot body must be serializable to plain JSON.**
> The workspace is a filesystem store, not a database. The snapshot body must be a plain JSON file so it can be opened, diffed, and version-controlled alongside code if desired.

> **Rule-08: Snapshot semantic version must match the ADR/semantic model version.**
> When the semantic model changes (e.g. new Observation field, new Evolution dimension), `semantic_version` must increment. This lets consumers detect that two snapshots are not directly comparable.

> **Rule-09: Re-running a snapshot with the same inputs must produce the same body.**
> The producer must be deterministic. Non-determinism (e.g. unordered hash maps, timestamps inside the body) is a bug.

> **Rule-10: Historical Replay produces snapshots, not just condition analytics.**
> A replay run should produce one `ResearchSnapshot` per target date and store it in the workspace. The replay report is a human-readable summary of the snapshot.

## Rejected Alternatives

### 1. Embed Evidence directly in the snapshot

**Reason rejected:** Embedding would duplicate historical facts, break evidence lifecycle management, and make evidence updates impossible without rewriting every snapshot. Evidence is a standalone Research Asset with its own lifecycle.

### 2. Store snapshots in ClickHouse or SQLite

**Reason rejected:** The workspace is meant to be human-readable, diffable, and portable. A database table would make it harder to inspect, version-control, or move across environments. Market data still lives in ClickHouse; research assets live in the workspace.

### 3. Make snapshots append-only and immutable

**Reason rejected:** An append-only registry would make it impossible to mark a draft as active or to supersede a snapshot after a semantic bug is fixed. Status transitions are necessary for a practical audit workflow. The evidence body remains immutable; only the catalog is mutable.

### 4. Use `ResearchContext` as the snapshot

**Reason rejected:** `ResearchContext` is a semantic model, not a durable asset. It lacks provenance, version, lifecycle status, and evidence references. Conflating the two would break the V6 reporting platform boundary (ADR-068).

### 5. Store snapshots in `shadow-production/`

**Reason rejected:** `shadow-production/` is an operational observation directory for the 90-day Shadow Production period. Research snapshots are long-term assets that outlive any single observation campaign. Mixing them would blur operational and research ownership.

## Validation

- `cargo check` across the workspace passes.
- `cargo test -p app-service -p core-domain` passes.
- A CLI smoke test can produce and save a snapshot: `quant-cli research snapshot --scope global --date 2026-07-09`.
- A saved snapshot can be loaded and its evidence refs resolved to the same evidence body.
- Re-running the same command produces the same `dataset_hash` and `config_hash` for the same inputs.

## Evolution Path

- **V8.0**: Snapshot structure, workspace layout, and producer API are implemented. Historical Replay is updated to produce snapshots.
- **V8.1**: Snapshot diff / drift detection tool compares two snapshots for the same scope+date.
- **V8.2**: Snapshot replay tool loads a snapshot and regenerates the exact research view.
- **V9+**: New semantic dimensions (e.g. Fear, Liquidity) are added via `semantic_version` bump and new embedded fields; old snapshots remain readable.

## Related Documents

- `memory/decisions.md` — ADR-078 Research Attribution Layer
- `docs/v6/adr-068-research-context-reporting-layer.md`
- `docs/v6/adr-071-market-fingerprint-evidence-layer.md`
- `docs/v6/adr-080-research-asset-lifecycle.md` — unified lifecycle for all Research Assets
- `docs/v6/adr-081-research-asset-identity.md` — unified `RA-XXXXXX` asset identity
- `docs/architecture-invariants.md` — V6 reporting platform ownership rules
- `docs/shadow-production-playbook.md` — operational observation workflow
