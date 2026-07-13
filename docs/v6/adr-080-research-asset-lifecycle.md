# ADR-080: Research Asset Lifecycle — Unified State Machine

**Status:** Accepted

**Tags:** v8, research-asset, lifecycle, workspace, adr-079, adr-081

## Context

ADR-078 introduced Evidence as a Research Asset. ADR-079 introduced the Research Snapshot as a durable Research Asset in the workspace. As the workspace grows, it will hold multiple kinds of Research Assets: Evidence, Snapshots, Hypotheses, Validation Records, and eventually Knowledge.

Each kind currently risks inventing its own lifecycle vocabulary. Evidence has `Collected` / `Validated` / `Verified`; Snapshots have `Draft` / `Active` / `Superseded` / `Archived`. If left unchecked, this divergence will make the registry, search, and governance logic kind-specific and fragile.

A unified Research Asset lifecycle is the minimal abstraction needed to keep the workspace coherent across asset kinds.

## Decision

Define a single, asset-kind-agnostic lifecycle with five states:

```text
Draft → Verified → Published → Superseded → Archived
```

Every Research Asset in the workspace must use this vocabulary. The meaning of each state is interpreted per asset kind, but the transitions and governance rules are shared.

## Final Design

### Lifecycle States

| State | Meaning | Human-readable shorthand |
|-------|---------|---------------------------|
| **Draft** | Produced by a computation or replay, but not yet reviewed or trusted. | "Do not use for decisions." |
| **Verified** | Reviewed against source data and reproducibility checks; internally trusted. | "OK for internal research." |
| **Published** | Accepted as the canonical asset for a given scope/date/condition. | "Consumers may use this." |
| **Superseded** | A newer canonical asset exists for the same semantic slot. | "Kept for audit; do not use." |
| **Archived** | Retained for historical record, but no longer relevant to active research. | "Read-only historical record." |

### State Transitions

Allowed transitions:

- **Draft → Verified**: after review or automated validation passes.
- **Verified → Published**: after human/ADR approval.
- **Published → Superseded**: when a newer asset is published for the same semantic slot.
- **Superseded → Archived**: when the asset is no longer relevant to active research.
- **Any state → Discarded**: only for assets that were never meaningful (e.g. a failed computation). Discarded assets are removed from the registry, not just marked.

Forbidden transitions:

- **Verified → Draft**: review cannot be undone; a new draft asset is created instead.
- **Superseded → Published**: once superseded, an asset cannot be re-published. A new asset is created instead.
- **Archived → Published**: archived assets are read-only.

### Per-Asset Interpretation

#### Evidence

- **Draft**: collected from replay or analytics, raw facts not yet validated.
- **Verified**: forward-return computation reproduced; facts are internally trusted.
- **Published**: used as supporting/contradicting evidence in a Snapshot or Hypothesis.
- **Superseded**: newer evidence for the same condition/scope/horizon window is published.
- **Archived**: kept for audit but no longer referenced by active snapshots.

#### Snapshot

- **Draft**: produced by a replay or manual run, not yet reviewed.
- **Verified**: snapshot body reproducible from dataset/config hashes; internally trusted.
- **Published**: the canonical snapshot for a given scope+date.
- **Superseded**: a newer published snapshot exists for the same scope+date.
- **Archived**: retained for historical replay audit.

#### Hypothesis

- **Draft**: proposed explanation from attribution or LLM synthesis.
- **Verified**: confirmed by evidence or historical replay.
- **Published**: accepted into a Knowledge article or report.
- **Superseded**: replaced by a better hypothesis.
- **Archived**: historical hypothesis, no longer active.

#### Validation Record

- **Draft**: a validation run completed.
- **Verified**: results checked against ground truth or reproducibility criteria.
- **Published**: part of the official validation record for a model/ADR.
- **Superseded**: newer validation run replaces it.
- **Archived**: historical validation record.

#### Knowledge

- **Draft**: a synthesis article or finding.
- **Verified**: reviewed by human or automated consistency checks.
- **Published**: the canonical knowledge statement for a topic.
- **Superseded**: newer knowledge replaces it (Knowledge Evolution).
- **Archived**: historical knowledge, preserved for traceability.

### Registry Index Fields

Every registry index entry must include:

```json
{
  "id": "RA-000001",
  "kind": "Evidence",
  "version": 1,
  "status": "draft",
  "created_at": "...",
  "published_at": null,
  "superseded_at": null,
  "archived_at": null,
  "path": "workspace/evidence/replay/RA-000001/body.json"
}
```

The `status` field is the single source of truth for lifecycle state. Timestamps are optional metadata for audit.

## Architecture Rules

> **Rule-01: One lifecycle vocabulary for all Research Assets.**
> No asset kind may invent its own state names. This keeps registry queries, UI filters, and governance rules generic.

> **Rule-02: `Published` is the only state consumed by downstream consumers.**
> CLI, Desktop, LLM, reports, and Knowledge must prefer `Published` assets. `Verified` assets may be used internally but must not be the canonical input for published reports.

> **Rule-03: `Superseded` assets are never deleted.**
> They remain in the workspace for audit, drift detection, and reproducibility checks.

> **Rule-04: Transitions are append-only in the registry log.**
> The registry entry may be updated with a new status, but an audit log (or version history) should record the transition. For now, the registry entry itself is rewritten with a new status; a future ADR may introduce a transition log.

> **Rule-05: Status changes are deliberate, not automatic.**
> A computation may produce a `Draft` asset, but promotion to `Verified` or `Published` requires an explicit action (human review, automated check, or ADR approval). Do not auto-publish replay outputs.

> **Rule-06: Discarded assets are removed from the registry.**
> `Draft` assets that are wrong or useless should be deleted, not marked. Only meaningful assets enter the lifecycle.

> **Rule-07: The same lifecycle applies to Evidence, Snapshot, Hypothesis, Validation, and Knowledge.**
> Future asset kinds must reuse this lifecycle. If a new state is needed, it must be added to the shared lifecycle rather than per-kind.

## Rejected Alternatives

### 1. Per-asset-kind lifecycle states

**Reason rejected:** Evidence `Collected`, Snapshot `Active`, Knowledge `Approved` would fragment the registry and force every consumer to know every kind's vocabulary. A unified lifecycle keeps the workspace generic.

### 2. A single `status` enum without `Verified`

**Reason rejected:** Collapsing `Draft → Published` skips the internal review step. `Verified` is the boundary between raw computation and trusted input; it maps directly to reproducibility checks and automated validation.

### 3. `Active` instead of `Published`

**Reason rejected:** `Active` is a runtime concept; `Published` is a publication concept. The workspace is a research library, not a running process. `Published` better conveys that the asset is canonical for consumers.

## Validation

- `cargo check` across the workspace passes.
- `cargo test -p app-service` passes.
- The workspace manager uses the unified lifecycle enum for both Evidence and Snapshot statuses.
- A `research analytics --save-evidence` run produces a `Draft` evidence asset; promotion to `Verified` or `Published` is an explicit registry edit.

## Evolution Path

- **V8.0**: Lifecycle vocabulary frozen; Evidence and Snapshot use unified states.
- **V8.1**: Registry transition log introduced if needed for audit.
- **V9.0**: Knowledge assets enter the same lifecycle, enabling Knowledge Evolution.
- **V10.0+**: Autonomous research workflows may propose lifecycle transitions; human approval remains required for `Verified → Published`.

## Related Documents

- `docs/v6/adr-079-research-snapshot.md`
- `docs/v6/adr-081-research-asset-identity.md`
- `memory/decisions.md`
