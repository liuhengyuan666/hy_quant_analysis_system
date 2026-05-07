# DOCS KNOWLEDGE BASE

## OVERVIEW
Project-facing documentation lives here. Mix of user workflow guides and engineering architecture/module references.

## WHERE TO LOOK
| Need | File | Notes |
|------|------|-------|
| Daily usage | `日常操作手册.md` | operational steps, command flow |
| Analysis interpretation | `分析使用手册.md` | MA20/MA60/MACD, regime, rotation, signals |
| Architecture overview | `系统架构与数据流.md` | end-to-end data flow, storage, date semantics |
| Module responsibilities | `功能模块与处理逻辑.md` | per-module inputs, outputs, source logic |
| Breadth planning | `市场广度指标-MA30规划.md` | true market breadth planning |
| Breadth V1 plan | `市场广度指标-MA30-V1实施计划.md` | watchlist breadth proxy rollout |
| Scoped reporting semantics | `README.md` + `系统架构与数据流.md` | GLOBAL vs CN vs HK reporting behavior |

## CONVENTIONS
- User-facing docs stay in Chinese unless there is a strong reason otherwise.
- README is the entry point; docs here are the detailed reference layer.
- Keep command examples aligned with actual CLI names and current date semantics.
- If dashboard behavior changes, update both user docs and architecture docs.
- Desktop Help/Usage viewer reads markdown from this directory; docs should remain renderable as standalone markdown.
- If freshness/debug workflows change, update docs to mention `pipeline-dates` and the distinction between market freshness and macro as-of dates.
- If scoped reporting changes, document which layers are scope-aware. All layers (regime, environment, strategy preference, signal, backtest) now support per-scope computation with explicit provenance fields.

## ANTI-PATTERNS
- Do **not** document outdated date semantics (`report_date` only) without `regime_as_of_date` context.
- Do **not** describe Yahoo/Tushare as default runtime dependencies.
- Do **not** add commands to docs before verifying they still compile/run.
- Do **not** describe health-report filenames as export-time dates; they now track the freshest checked market date.
- Do **not** describe watchlist breadth proxy as stock-universe breadth.
- Do **not** omit provenance fields from documentation; strategy/signal/backtest now carry analysis_scope and regime_basis_scope.

## NOTES
- Desktop Help/Usage viewer reads markdown from this directory.
- Architecture docs should reflect the current bundled dashboard startup, background refresh model, and health-check flow.
- Root planning docs outside `docs/` (`设计规划.md`, `实施路径-v1.md`, `数据源方案评审.md`) still matter when architecture intent drifts from implementation.
