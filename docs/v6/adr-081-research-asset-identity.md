# ADR-081: Research Asset Identity — Unified RA-XXXXXX Identifier

**Status:** Accepted

**Tags:** v8, research-asset, identity, registry, adr-079, adr-080

## Context

ADR-079 introduced `EV-XXXXXX` identifiers for Evidence assets. ADR-079 introduced `SN-XXXXXX` identifiers for Research Snapshots. As the workspace grows, it will naturally add more asset kinds: Hypotheses (`HP-XXXXXX`), Validation Records (`VL-XXXXXX`), and Knowledge articles (`KB-XXXXXX`).

Using a separate prefix per asset kind has two problems:

1. **Registry complexity**: every index query, sort, and allocation must branch on the kind-specific prefix. There is no single counter or namespace for the workspace.
2. **Kind migration**: an asset may start as a Hypothesis, become Evidence-backed, and later be promoted to Knowledge. A kind-specific prefix forces a re-identity event, breaking references.

A unified Research Asset identity (`RA-XXXXXX`) with a separate `kind` metadata field keeps identifiers stable while allowing kind to evolve.

## Decision

Introduce a unified `ResearchAssetId` format `RA-XXXXXX` (e.g. `RA-000001`) for all assets stored in the workspace. The `kind` of the asset is recorded in metadata, not in the identifier.

Existing `EV-XXXXXX` and `SN-XXXXXX` identifiers are grandfathered and remain valid for their current assets. New assets must use `RA-XXXXXX`. A future migration may rewrite old indexes to use `RA-XXXXXX`, but this is not required for V8.

## Final Design

### ResearchAssetId

```rust
pub struct ResearchAssetId {
    pub sequence: u64,   // monotonically increasing workspace-wide sequence
}
```

String format: `RA-{sequence:06}` (e.g. `RA-000001`).

The sequence is assigned by the workspace registry and is unique across all asset kinds. It is **not** kind-specific.

### AssetKind

```rust
pub enum AssetKind {
    Evidence,
    Snapshot,
    Hypothesis,
    Validation,
    Knowledge,
}
```

The `kind` is stored in metadata, not encoded in the identifier.

### Registry Index Entry

```rust
pub struct AssetIndexEntry {
    pub id: ResearchAssetId,
    pub kind: AssetKind,
    pub version: u32,
    pub status: ResearchAssetLifecycle,  // ADR-080
    pub created_at: DateTime<Utc>,
    pub path: String,
}
```

### Directory Layout

Kind-specific directories remain because they are useful for human browsing and bulk operations, but the identifier inside each directory is the unified `RA-XXXXXX`:

```text
workspace/
├── evidence/
│   └── replay/
│       └── RA-000001/
│           ├── body.json
│           └── metadata.json
├── research-snapshots/
│   └── RA-000002/
│       ├── body.json
│       └── metadata.json
└── registry/
    ├── asset-index.json
    ├── evidence-index.json   # legacy, maintained until migration
    └── snapshot-index.json   # legacy, maintained until migration
```

### Sequence Allocation

The workspace registry maintains a single counter. The counter starts at the maximum existing sequence found in the workspace, so that existing `EV-000001` / `SN-000001` do not collide with new `RA-XXXXXX` allocations.

Migration strategy (optional, V8.1+):

1. Read existing `EV-XXXXXX` / `SN-XXXXXX` entries.
2. Assign new `RA-XXXXXX` identifiers with `sequence` greater than all existing sequences.
3. Rewrite body/metadata paths.
4. Update all references in snapshots and knowledge articles.
5. Drop legacy index files.

## Architecture Rules

> **Rule-01: New assets use `RA-XXXXXX`.**
> All assets created after this ADR must use the unified identifier. No new `EV-`, `SN-`, `HP-`, `VL-`, or `KB-` prefixes.

> **Rule-02: `kind` is metadata, not part of the identifier.**
> The identifier tells you nothing about whether the asset is Evidence, Snapshot, or Knowledge. The metadata tells you that.

> **Rule-03: The sequence counter is workspace-wide.**
> Do not maintain separate counters per kind. A single counter guarantees no collisions and simplifies registry allocation.

> **Rule-04: Existing `EV-XXXXXX` / `SN-XXXXXX` identifiers remain valid.**
> Grandfathering avoids a breaking migration during V8. The legacy indexes continue to work. New code should prefer the unified `asset-index.json` if it exists, but fall back to legacy indexes.

> **Rule-05: References between assets use `ResearchAssetId`.**
> `EvidenceRef`, `SnapshotRef`, `KnowledgeRef`, and any future reference type must use the unified identifier, not a kind-specific string.

> **Rule-06: Directory layout may still be kind-specific.**
> Human-readable directory structure (`evidence/`, `research-snapshots/`) is orthogonal to asset identity. The path in the registry entry tells consumers where to find the asset body.

> **Rule-07: Sequence numbers are never reused.**
> Even if an asset is discarded, its sequence number is retired. This avoids accidental resurrection of deleted assets.

## Rejected Alternatives

### 1. Keep kind-specific prefixes (EV, SN, HP, KB, VL)

**Reason rejected:** As the workspace grows, the registry would accumulate N different prefix counters and parsers. Cross-kind references would require special handling. A unified identifier is the simplest long-term solution.

### 2. Use UUIDs instead of sequential RA-XXXXXX

**Reason rejected:** UUIDs are not human-friendly in a filesystem directory and make browsing/regression difficult. A workspace-wide sequence is deterministic, sortable, and readable.

### 3. Embed both kind and sequence in the identifier (e.g. `RA-EV-000001`)

**Reason rejected:** This is the same as the old kind-specific prefix, just with a common `RA-` prefix. The goal is to make the identifier fully kind-agnostic so that kind can change without re-identity.

### 4. Force migration of all existing EV/SN identifiers now

**Reason rejected:** The existing evidence and snapshot assets are already stable. A forced migration would rewrite paths and references without clear benefit. Grandfathering is acceptable because the new identifier scheme is forward-compatible.

## Validation

- `cargo check` across the workspace passes.
- `cargo test -p app-service` passes.
- New evidence assets are assigned `RA-XXXXXX` identifiers.
- The legacy `evidence-index.json` and `snapshot-index.json` still load existing `EV-XXXXXX` / `SN-XXXXXX` assets.
- The workspace manager can allocate a new identifier without colliding with legacy IDs.

## Evolution Path

- **V8.0**: Unified `RA-XXXXXX` identifier introduced; new assets use it. Legacy `EV-XXXXXX` / `SN-XXXXXX` grandfathered.
- **V8.1**: Optional migration tool rewrites legacy indexes to `asset-index.json` with unified identifiers.
- **V9.0+**: New asset kinds (Hypothesis, Validation, Knowledge) use `RA-XXXXXX` from day one.

## Related Documents

- `docs/v6/adr-079-research-snapshot.md`
- `docs/v6/adr-080-research-asset-lifecycle.md`
- `memory/decisions.md`
