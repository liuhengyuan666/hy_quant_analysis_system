# RESEARCH-BENCHMARK KNOWLEDGE BASE

## OVERVIEW
Benchmarking harness for research skills. Measures skill execution performance, accuracy, and token usage.

## STRUCTURE
```text
crates/research-benchmark/src/
├── lib.rs       # module declarations
├── harness.rs   # benchmark execution harness
├── metrics.rs   # performance metrics collection
└── reporters.rs # benchmark result formatters
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Run benchmarks | `harness.rs` | BenchmarkHarness |
| Collect metrics | `metrics.rs` | MetricsCollector |
| Format results | `reporters.rs` | output formatters |

## CONVENTIONS
- Benchmarks use `research-context` for input and `research-skills` for execution.
- Metrics include token usage, latency, and accuracy scores.
- Reporters output JSON or markdown.

## ANTI-PATTERNS
- Do **not** add production logic here; this is test/evaluation only.
- Do **not** run benchmarks against live APIs without rate limiting.

## NOTES
- Wave 3 work: actual benchmark execution is TODO (line 15 in harness.rs).
- Depends on both `research-context` and `research-skills`.
