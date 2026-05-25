/// Token budget for controlling prompt size
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub max_system_tokens: usize,
    pub max_context_tokens: usize,
    pub max_reasoning_tokens: usize,
    pub max_output_tokens: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_system_tokens: 1024,
            max_context_tokens: 2048,
            max_reasoning_tokens: 1536,
            max_output_tokens: 2048,
        }
    }
}
