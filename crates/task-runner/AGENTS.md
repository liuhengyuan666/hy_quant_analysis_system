# TASK RUNNER KNOWLEDGE BASE

## OVERVIEW
Placeholder task execution framework. Currently only exports a runner name; no real scheduling or execution logic is implemented.

## STRUCTURE
```text
crates/task-runner/
└── src/
    └── lib.rs              # 3-line placeholder
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Runner name | `src/lib.rs` | `runner_name()` returns `"local-task-runner"` |

## CONVENTIONS
- This crate is intentionally a placeholder. Do not build production orchestration on it.
- Any future task execution logic should be designed with an ADR and not leak into `app-service`.

## ANTI-PATTERNS
- Do **not** add production logic here without an ADR.
- Do **not** run benchmarks or live workflows against this crate as if it were a real scheduler.
- Do **not** make `app-service` depend on task-runner behavior for critical paths.

## NOTES
- Status: placeholder-level. See `memory/decisions.md` and `docs/阶段性更新.md` before expanding.
