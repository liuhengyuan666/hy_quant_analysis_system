use research_context::ResearchContext;

use super::deterministic::DeterministicConfig;
use super::provider::LlmProvider;
use super::skill::Skill;
use super::token_budget::TokenBudget;

/// Execution result from a skill
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillOutput {
    pub skill_name: String,
    pub system_prompt: String,
    pub context_json: String,
    pub reasoning_yaml: String,
    pub rendered_prompt: String,
    pub response: Option<String>,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub system_tokens: usize,
    pub context_tokens: usize,
    pub reasoning_tokens: usize,
    pub output_tokens: usize,
}

/// Layered executor for skills
pub struct SkillExecutor {
    #[allow(dead_code)]
    budget: TokenBudget,
    deterministic: DeterministicConfig,
}

impl SkillExecutor {
    pub fn new(budget: TokenBudget, deterministic: DeterministicConfig) -> Self {
        Self {
            budget,
            deterministic,
        }
    }

    /// Execute a skill with the given context
    pub async fn execute(
        &self,
        skill: &Skill,
        context: &ResearchContext,
        provider: &dyn LlmProvider,
    ) -> anyhow::Result<SkillOutput> {
        // Layer 1: System prompt
        let system_prompt = self.render_system_layer(skill);

        // Layer 2: Semantic context (JSON)
        let context_json = self.render_semantic_layer(context);

        // Layer 3: Reasoning graph (YAML)
        let reasoning_yaml = self.render_reasoning_layer(skill);

        // Layer 4: Final rendered prompt
        let rendered_prompt = self.render_final_layer(
            skill,
            &system_prompt,
            &context_json,
            &reasoning_yaml,
        );

        // Calculate token usage (pre-call)
        let token_usage = TokenUsage {
            system_tokens: provider.token_count(&system_prompt),
            context_tokens: provider.token_count(&context_json),
            reasoning_tokens: provider.token_count(&reasoning_yaml),
            output_tokens: 0,
        };

        // Call LLM provider
        let response = provider
            .chat(&system_prompt, &rendered_prompt, &self.deterministic.to_config())
            .await?;

        Ok(SkillOutput {
            skill_name: skill.definition.name.clone(),
            system_prompt,
            context_json,
            reasoning_yaml,
            rendered_prompt,
            response: Some(response.clone()),
            token_usage: TokenUsage {
                output_tokens: provider.token_count(&response),
                ..token_usage
            },
        })
    }

    /// Layer 1: System prompt (research style, risk tolerance)
    fn render_system_layer(&self, skill: &Skill) -> String {
        format!(
            "You are a quantitative research analyst.\n\n\
             Skill: {}\n\
             Description: {}\n\n\
             Analyze the provided market context and produce a structured analysis.",
            skill.definition.name, skill.definition.description
        )
    }

    /// Layer 2: Semantic context (JSON)
    fn render_semantic_layer(&self, context: &ResearchContext) -> String {
        serde_json::to_string_pretty(context).unwrap_or_default()
    }

    /// Layer 3: Reasoning graph (YAML)
    fn render_reasoning_layer(&self, skill: &Skill) -> String {
        serde_yaml::to_string(&skill.reasoning).unwrap_or_default()
    }

    /// Layer 4: Final rendered prompt
    fn render_final_layer(
        &self,
        skill: &Skill,
        _system: &str,
        context: &str,
        reasoning: &str,
    ) -> String {
        format!(
            "# Market Context\n\n```json\n{}\n```\n\n\
             # Reasoning Graph\n\n```yaml\n{}\n```\n\n\
             # Task\n\n{}\n\n\
             # Output Format\n\n{}\n",
            context, reasoning, skill.overview, skill.output_format
        )
    }
}
