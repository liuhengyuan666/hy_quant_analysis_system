# DOCS KNOWLEDGE BASE

## OVERVIEW
Project-facing documentation lives here. Mix of user workflow guides and engineering architecture/module references. Version-specific design docs are grouped under `docs/v2/`, `docs/v3/`, `docs/v5/`, and `docs/v6/`; current reference docs stay in the `docs/` root.

## WHERE TO LOOK
| Need | File | Notes |
|------|------|-------|
| Daily usage | `日常操作手册.md` | operational steps, command flow |
| Analysis interpretation | `分析使用手册.md` | MA20/MA60/MACD, regime, rotation, signals |
| Architecture overview | `系统架构与数据流.md` | end-to-end data flow, storage, date semantics |
| Module responsibilities | `功能模块与处理逻辑.md` | per-module inputs, outputs, source logic |
| Doc classification | `文档状态说明.md` | current reference vs active design vs archive |
| Phase progress | `阶段性更新.md` | single append-only record of system evolution |
| V2 environment layer | `v2/V2-Phase1-环境层详细技术设计.md` | per-scope regime + environment layer |
| V3 scoped reporting | `v3/` | code-review and scoped reporting docs |
| V5 execution / Shadow Production | `v5/` + `shadow-production-playbook.md` | execution layer, Shadow Production declaration, State/Economic ADRs |
| V6 reporting platform | `v6/` + `architecture-invariants.md` | reporting platform contracts and ADR-068/069 |
| Shadow Production ops | `shadow-production-playbook.md` | 90-day observation guidance |

## CONVENTIONS
- User-facing docs stay in Chinese unless there is a strong reason otherwise.
- README is the entry point; docs here are the detailed reference layer.
- Keep command examples aligned with actual CLI names and current date semantics.
- If dashboard behavior changes, update both user docs and architecture docs.
- Desktop Help/Usage viewer reads markdown from this directory; docs should remain renderable as standalone markdown.
- If freshness/debug workflows change, update docs to mention `pipeline-dates` and the distinction between market freshness and macro as-of dates.
- If scoped reporting changes, document which layers are scope-aware. All layers (regime, environment, strategy preference, signal, backtest) now support per-scope computation with explicit provenance fields.
- Version-specific docs go under `docs/vN/`; cross-version references stay in root.
- Root planning docs outside `docs/` (`设计规划-v1.md`, `数据源方案评审.md`) are archive/reference material unless `docs/文档状态说明.md` says they are current.

## ANTI-PATTERNS
- Do **not** document outdated date semantics (`report_date` only) without `regime_as_of_date` context.
- Do **not** describe Yahoo/Tushare as default runtime dependencies.
- Do **not** add commands to docs before verifying they still compile/run.
- Do **not** describe health-report filenames as export-time dates; they now track the freshest checked market date.
- Do **not** describe watchlist breadth proxy as stock-universe breadth.
- Do **not** omit provenance fields from documentation; strategy/signal/backtest now carry analysis_scope and regime_basis_scope.
- Do **not** place current reference docs inside version subdirectories if the frontend or README references them by root path.

## NOTES
- Desktop Help/Usage viewer reads markdown from this directory.
- Architecture docs should reflect the current bundled dashboard startup, background refresh model, and health-check flow.
- `docs/阶段性更新.md` is the single chronological progress record; append new milestones rather than creating new `阶段性更新-*.md` files.
- V6 Reporting Platform is frozen; future consumer docs build on the platform rather than documenting changes to it.
