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

impl TokenBudget {
    /// Estimate token count from text (4 chars ≈ 1 token)
    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }

    /// Trim context to fit within budget by priority.
    /// Priority: market > liquidity > regime > breadth > rotation > signals > macro > risk
    pub fn fit_context(&self, context: &mut research_context::ResearchContext) {
        let context_json = serde_json::to_string_pretty(context).unwrap_or_default();
        let current_tokens = Self::estimate_tokens(&context_json);

        if current_tokens <= self.max_context_tokens {
            return; // Within budget
        }

        // Priority order for trimming (lowest priority first)
        let trim_order: [fn(&mut research_context::ResearchContext); 8] = [
            // First trim: risk (lowest priority)
            |ctx: &mut research_context::ResearchContext| {
                ctx.risk = research_context::RiskContext {
                    skewness: None,
                    kurtosis: None,
                    tail_index: None,
                };
            },
            // Second trim: macro
            |ctx: &mut research_context::ResearchContext| {
                ctx.macro_ = research_context::MacroContext {
                    spread_10y: None,
                    dxy_index: None,
                    foreign_flow: None,
                    vix: None,
                };
            },
            // Third trim: signals
            |ctx: &mut research_context::ResearchContext| {
                ctx.signals = research_context::SignalsContext {
                    bullish_count: 0,
                    defensive_count: 0,
                    data_starved_count: 0,
                };
            },
            // Fourth trim: rotation
            |ctx: &mut research_context::ResearchContext| {
                ctx.rotation = research_context::RotationContext {
                    state: research_context::RotationState::Broad,
                    top_sectors: Vec::new(),
                    bottom_sectors: Vec::new(),
                    leadership_stability: 0.0,
                    momentum_factor: None,
                    value_factor: None,
                    quality_factor: None,
                    crowding_factor: None,
                };
            },
            // Fifth trim: breadth
            |ctx: &mut research_context::ResearchContext| {
                ctx.breadth = research_context::BreadthContext {
                    condition: research_context::BreadthCondition::Strong,
                    breadth_pct: 0.0,
                    breadth_delta: 0.0,
                };
            },
            // Sixth trim: regime (keep only current label)
            |ctx: &mut research_context::ResearchContext| {
                ctx.regime = research_context::RegimeContext {
                    current: ctx.regime.current.clone(),
                    confidence: 0.0,
                    macro_stale_days: 0,
                };
            },
            // Seventh trim: liquidity (keep only pressure)
            |ctx: &mut research_context::ResearchContext| {
                ctx.liquidity = research_context::LiquidityContext {
                    pressure: ctx.liquidity.pressure.clone(),
                    spread: None,
                    yield_curve_status: None,
                    dollar_strength: None,
                };
            },
            // Eighth trim: market (keep only current_state)
            |ctx: &mut research_context::ResearchContext| {
                ctx.market = research_context::MarketContext {
                    current_state: ctx.market.current_state.clone(),
                    previous_state: None,
                    confidence: 0.0,
                    drivers: Vec::new(),
                    transition: None,
                };
            },
        ];

        for trim_fn in &trim_order {
            trim_fn(context);
            let json = serde_json::to_string_pretty(context).unwrap_or_default();
            if Self::estimate_tokens(&json) <= self.max_context_tokens {
                return;
            }
        }
    }
}
