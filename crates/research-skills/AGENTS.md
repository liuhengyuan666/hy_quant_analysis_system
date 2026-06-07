# RESEARCH-SKILLS KNOWLEDGE BASE

## OVERVIEW
LLM-powered research analysis engine. Parses SKILL.md definitions, routes to appropriate skills, executes via OpenAI-compatible APIs, and renders structured analysis output.

## STRUCTURE
```text
crates/research-skills/src/
├── lib.rs               # module declarations + re-exports
├── skill.rs             # SkillDefinition + Skill parsing from SKILL.md
├── agent_profile.rs     # AgentProfile with reasoning style + constraints
├── trigger.rs           # skill trigger conditions
├── reasoning.rs         # ReasoningGraph for skill execution flow
├── registry.rs          # skill registry (name → Skill lookup)
├── router.rs            # routes queries to appropriate skills
├── executor.rs          # orchestrates skill execution with LLM
├── provider.rs          # LlmProvider trait
├── openai_provider.rs   # OpenAI-compatible API implementation
├── renderer.rs          # render_analysis_markdown output formatter
├── schema.rs            # output schema definitions
├── token_budget.rs      # token counting + budget enforcement
├── deterministic.rs     # DeterministicConfig for testing without LLM
├── analysis.rs          # ResearchAnalysis output type
└── regime_state_machine.rs  # regime transition tracking
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add new skill | `skills/` directory + `registry.rs` | SKILL.md with YAML front matter |
| Change agent behavior | `agent_profile.rs` | risk_tolerance, output_depth, tone |
| Modify LLM calls | `openai_provider.rs` | async-openai crate |
| Adjust output format | `renderer.rs` | markdown rendering |
| Token limits | `token_budget.rs` | budget enforcement |
| Testing without LLM | `deterministic.rs` | DeterministicConfig |

## CONVENTIONS
- Skills are defined in SKILL.md files with YAML front matter (name, description, trigger, inputs, outputs).
- `ReasoningGraph` defines the execution flow within a skill.
- Agent profiles control reasoning style (conservative/moderate/aggressive risk, shallow/standard/deep output).
- `LlmProvider` trait abstracts LLM calls; `OpenAiProvider` is the default implementation.
- Token budget is enforced per-analysis to prevent runaway costs.
- Deterministic mode allows testing without actual LLM calls.

## ANTI-PATTERNS
- Do **not** hardcode API keys; use `app-service` credential management.
- Do **not** bypass token budget enforcement.
- Do **not** add persistence logic here; this crate is pure analysis.
- Do **not** change skill YAML schema without updating all SKILL.md files.

## NOTES
- Depends on `research-context` for `ResearchContext` input.
- Depends on `core-domain` for shared types.
- `async-openai` is the LLM client library.
- Many TODOs remain in `research-context/src/builder.rs` for data-quality computations.
