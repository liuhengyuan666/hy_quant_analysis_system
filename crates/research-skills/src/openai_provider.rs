use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use async_trait::async_trait;

use super::provider::{LlmCallConfig, LlmProvider};

/// OpenAI-compatible LLM provider (supports OpenAI, DeepSeek, and other OpenAI-compatible APIs)
pub struct OpenAiProvider {
    client: Client<OpenAIConfig>,
    model: String,
    timeout_secs: u64,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(base_url: &str, model: &str, api_key: &str, timeout_secs: u64) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);

        Self {
            client: Client::with_config(config),
            model: model.to_string(),
            timeout_secs,
        }
    }

    /// Create from LlmConfig (from core-domain)
    pub fn from_config(config: &core_domain::LlmConfig, api_key: &str) -> Self {
        Self::new(&config.base_url, &config.model, api_key, config.timeout_secs)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        config: &LlmCallConfig,
    ) -> anyhow::Result<String> {
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&self.model)
            .temperature(config.temperature as f32)
            .max_tokens(config.max_tokens as u32);
        if let Some(seed_val) = config.seed {
            request_builder.seed(seed_val as i64);
        }
        let request = request_builder
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()
                    .map_err(|e| anyhow::anyhow!("failed to build system message: {e}"))?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(user_prompt)
                    .build()
                    .map_err(|e| anyhow::anyhow!("failed to build user message: {e}"))?
                    .into(),
            ])
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build chat completion request: {e}"))?;

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            self.client.chat().create(request),
        )
        .await
        .map_err(|_| anyhow::anyhow!("LLM API call timed out after {}s", self.timeout_secs))?
        .map_err(|e| anyhow::anyhow!("LLM API call failed: {e}"))?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default();

        Ok(content)
    }
}
