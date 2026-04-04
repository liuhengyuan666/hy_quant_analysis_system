# DATA-INGESTION KNOWLEDGE BASE

## OVERVIEW
Provider boundary for market/macro fetch. Owns universe loading, provider symbol mapping, daily-bar fetch order, and canonical adjustment semantics.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Universe schema | `src/lib.rs::UniverseRecord` | JSON fields parsed here |
| Universe loading | `src/lib.rs::load_universe` | maps config to `Instrument` |
| Eastmoney fetch | `src/lib.rs::fetch_eastmoney_daily_bars` | primary daily-bar path |
| Tencent fallback | `src/lib.rs::fetch_tencent_daily_bars` | fallback / backup path |
| Provider order | `src/lib.rs::fetch_daily_bars` | primary then fallback |
| Macro fetch | `src/lib.rs::fetch_fred_series` | FRED CSV pull |
| Macro fetch status | `src/lib.rs::fetch_fred_series_with_status` | reqwest / curl transport visibility |
| Bar validation | `src/lib.rs::normalize_daily_bar` | canonical data-quality gate |

## CONVENTIONS
- Current canonical daily basis = **forward-adjusted**.
- Eastmoney must use `fqt=1`.
- Tencent must use `qfq`.
- `display_symbol` is presentation metadata only; provider IDs stay separate.
- Provider fetch should return parsed bars, then pass through `normalize_daily_bar()`.
- FRED macro fetch may use `curl` fallback on Windows when reqwest transport is not viable.
- FRED responses must look like real CSV headers (`DATE,...` or `observation_date,...`); HTML/error bodies are invalid and should fail loudly.
- Empty FRED observation windows are treated as fetch failure, not as valid zero-row success.
- Current V1 runtime universe is INDEX/ETF only; stock-universe expansion is future work.

## ANTI-PATTERNS
- Do **not** mix unadjusted and forward-adjusted history in the same stored series.
- Do **not** treat `display_symbol` as a fetch identifier.
- Do **not** move provider fallback logic into `app-service`.
- Do **not** bypass `normalize_daily_bar()` for new providers.
- Do **not** silently accept non-CSV macro responses; reject malformed FRED payloads before downstream scoring.

## NOTES
- Yahoo is intentionally not part of the default runtime chain.
- Tushare is a future optional enhancement source, not a current dependency.
- If a symbol lacks `tencent_symbol`, fallback coverage is intentionally partial.
- Macro transport status is now surfaced into Data Health; changes here affect CLI/report/UI diagnostics.
- Current macro fetch reliability issue is provider/network-side more often than parser-side; preserve explicit failure messages for `failed_items`.
- Watchlist breadth proxy consumes stored bars/MA30 downstream; do not special-case breadth logic inside ingestion.
