pub mod action;
pub mod analysis;
pub mod deterministic;
pub mod inference;
pub mod openai_provider;
pub mod provider;
pub mod renderer;
pub mod token_budget;

// Re-exports — 只保留分析层真正需要的类型
pub use action::{build_prompt, build_prompt_with_persona, builtin_persona, MARKET_STORY_PROMPT, EXPLAIN_DECISION_PROMPT, PRECLOSE_REVIEW_PROMPT, RISK_VIEW_PROMPT, DEVILS_ADVOCATE_PROMPT, PORTFOLIO_REVIEW_PROMPT};
pub use provider::{LlmProvider, LlmCallConfig};
pub use openai_provider::OpenAiProvider;
pub use inference::InferenceConfig;

// Re-export analysis types (保留但简化用途)
pub use analysis::ResearchAnalysis;

// Re-export renderer
pub use renderer::render_analysis_markdown;
