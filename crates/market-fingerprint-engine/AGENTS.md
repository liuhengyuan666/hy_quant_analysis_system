# MARKET FINGERPRINT ENGINE KNOWLEDGE BASE

## OVERVIEW
V7.2B historical evidence similarity matching. Builds canonical market fingerprints from observations and retrieves historical analogues by distance, consumed by `research analogues`.

## STRUCTURE
```text
crates/market-fingerprint-engine/
└── src/
    ├── lib.rs              # thin re-export hub
    ├── config.rs           # fingerprint configuration
    ├── distance.rs         # similarity metrics (cosine, etc.)
    ├── feature.rs          # feature extraction from observations
    ├── fingerprint.rs      # canonical fingerprint model
    ├── normalize.rs        # feature normalization
    └── store.rs            # in-memory fingerprint store
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Fingerprint model | `src/fingerprint.rs` | `Fingerprint`, `FingerprintVector` |
| Feature extraction | `src/feature.rs` | observation → feature vector |
| Normalization | `src/normalize.rs` | zero-mean / range normalization helpers |
| Distance metrics | `src/distance.rs` | `cosine_distance`, `euclidean_distance` |
| Configuration | `src/config.rs` | `FingerprintConfig`, lookback, feature weights |
| Store | `src/store.rs` | `FingerprintStore`, historical lookup |

## CONVENTIONS
- Fingerprint definition is canonical; similarity algorithms are consumers, not part of the definition (ADR-071).
- Matching operates on preloaded historical observations; never repeatedly reconstruct semantic models.
- Feature vectors are normalized before distance computation.
- The crate is pure computation; no I/O, no HTTP, no persistence.

## ANTI-PATTERNS
- Do **not** change the canonical feature representation without ADR review.
- Do **not** couple this crate to `app-service` orchestration logic.
- Do **not** perform ClickHouse/SQLite queries here; consumers load observations and pass them in.
- Do **not** add new distance metrics without validating against historical stability.

## NOTES
- This crate is a V7.2 addition and is missing from older root/crates AGENTS.md summaries; it should be referenced alongside other V7 research engines.
- `crates/market-fingerprint-engine/` has no integration tests; validate via `cargo check` and live `research analogues` flows.
