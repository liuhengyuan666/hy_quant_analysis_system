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

## CONVENTIONS
- User-facing docs stay in Chinese unless there is a strong reason otherwise.
- README is the entry point; docs here are the detailed reference layer.
- Keep command examples aligned with actual CLI names and current date semantics.
- If dashboard behavior changes, update both user docs and architecture docs.

## ANTI-PATTERNS
- Do **not** document outdated date semantics (`report_date` only) without `regime_as_of_date` context.
- Do **not** describe Yahoo/Tushare as default runtime dependencies.
- Do **not** add commands to docs before verifying they still compile/run.

## NOTES
- Desktop Help/Usage viewer reads markdown from this directory.
- Architecture docs should reflect the current background refresh model and health-check flow.
