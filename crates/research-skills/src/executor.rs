use research_context::ResearchContext;

use super::agent_profile::AgentProfile;
use super::inference::InferenceConfig;
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
    inference: InferenceConfig,
}

impl SkillExecutor {
    pub fn new(budget: TokenBudget, inference: InferenceConfig) -> Self {
        Self {
            budget,
            inference,
        }
    }

    /// Execute a skill with the given context and optional agent profile
    pub async fn execute(
        &self,
        skill: &Skill,
        context: &ResearchContext,
        provider: &dyn LlmProvider,
        profile: Option<&AgentProfile>,
    ) -> anyhow::Result<SkillOutput> {
        // Layer 1: System prompt
        let system_prompt = self.render_system_layer(skill, profile);

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
            .chat(&system_prompt, &rendered_prompt, &self.inference.to_call_config())
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

    /// Layer 1: System prompt (research style, risk tolerance, profile instructions)
    fn render_system_layer(&self, skill: &Skill, profile: Option<&AgentProfile>) -> String {
        let mut prompt = String::new();

        // Base system instructions
        prompt.push_str("You are a quantitative research analyst.\n\n");

        // Add profile-specific instructions if provided
        if let Some(profile) = profile {
            prompt.push_str(&profile.render_system_prompt());
            prompt.push_str("\n\n");

            // Add emphasis instructions based on profile constraints
            prompt.push_str("Analysis emphasis:\n");
            prompt.push_str(&format!(
                "- Regime transitions: {:?}\n",
                profile.analysis_constraints.emphasis.regime_transition
            ));
            prompt.push_str(&format!(
                "- Breadth signals: {:?}\n",
                profile.analysis_constraints.emphasis.breadth_signal
            ));
            prompt.push_str(&format!(
                "- Liquidity signals: {:?}\n",
                profile.analysis_constraints.emphasis.liquidity_signal
            ));
            prompt.push_str(&format!(
                "- Rotation signals: {:?}\n",
                profile.analysis_constraints.emphasis.rotation_signal
            ));
            prompt.push_str("\n");
        }

        // Add skill-specific instructions
        prompt.push_str(&format!("Skill: {}\n", skill.definition.name));
        prompt.push_str(&format!("Description: {}\n", skill.definition.description));
        prompt.push_str("\nAnalyze the provided market context and produce a structured analysis.");

        prompt
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
