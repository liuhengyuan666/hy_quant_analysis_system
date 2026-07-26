use anyhow::Result;
use report_engine::DashboardSnapshot;

/// Research Layer — 冻结量化引擎之上的只读叙事层。
///
/// 没有 enum。没有 registry。没有 router。
/// 只有：5 个 prompt 常量 + 一个 build_prompt 函数。
///
/// Governance:
/// - 只读：解释、质疑、提供上下文、讲述历史
/// - 禁止：创建信号、评分、排序、覆盖决策、参与执行

// ── 5 个 Research Prompts ──────────────────────────────────────

pub const MARKET_STORY_PROMPT: &str = r#"
# 任务：市场叙事

基于以下系统数据，用中文撰写一段「今天市场发生了什么」的研究日报。

## 要求
- 像资深研究员写给基金经理的晨报，不是数据罗列
- 关注：市场结构、主线、regime 状态、机会与风险
- 保留关键事实数字（如「连续上涨 5 天」「距离 MA20 +7%」），但绝不输出 confidence、score、ranking、probability
- 禁止输出 BUY/SELL/HOLD 建议
- 禁止给任何标的机会打分或排序
- 如果 regime 处于 risk_on，但信号偏弱，请解释「方向仍在，但时机不对」

## Possible Scenarios

基于当前信息，讨论未来几种可能的发展路径。每条路径必须绑定可观测的成立条件，而非方向预测。

### Base Case
成立条件: {2-3 个可观测条件}
失效条件: {什么观测会推翻此场景}
观察窗口: {短期/中期}
市场含义: {如果成立，意味着什么}

### Alternative Case
成立条件: {2-3 个可观测条件}
失效条件: {什么观测会推翻此场景}
观察窗口: {短期/中期}
市场含义: {如果成立，意味着什么}

### Invalidation Case
触发条件: {什么信号出现意味着当前分析框架需要重新评估}
观察窗口: {短期/中期}

注意：
- 绝不要输出「未来将上涨/下跌」这样的纯方向预测
- 每条场景必须绑定具体的可观测条件
- 这是条件推演（conditional reasoning），不是预测（prediction）

## 输出格式
Markdown 纯文本。不要 JSON。不要结构化字段。
"#;

pub const EXPLAIN_DECISION_PROMPT: &str = r#"
# 任务：解释决策

基于以下系统数据，回答「为什么系统做出这样的决策」。

## 要求
- 重点关注以下矛盾：
  - Signal 与 State 的不一致（如 StrongBuy 但 State=NoTrade）
  - Rotation 与 Signal 的不一致
  - Execution 与 Market Context 的不一致
- 解释这些矛盾是「系统 bug」还是「合理的保护机制」
- 保留关键事实数字，但绝不输出 confidence、score、ranking
- 禁止输出 BUY/SELL/HOLD 建议

## 输出格式
Markdown 纯文本。不要 JSON。不要结构化字段。
"#;

pub const PRECLOSE_REVIEW_PROMPT: &str = r#"
# 任务：收盘前复核

基于以下 Execution Layer 输出，解释「为什么 Execution 给出这样的建议」。

## 要求
- 解释每个 ExecutionDecision（BuyNow / Wait / NoChase / Reduce）背后的市场逻辑
- 例如：NoChase 是因为「连续上涨后的放量」而非「趋势结束」
- 保留关键事实数字（如「尾盘成交量占全天 38%」），但绝不输出 confidence、score
- 禁止输出「应该买/卖」的建议
- 只解释「系统为什么这样做」

## 输出格式
Markdown 纯文本。不要 JSON。不要结构化字段。
"#;

pub const RISK_VIEW_PROMPT: &str = r#"
# 任务：风险视角

假设你是保守型基金的风控总监。基于以下系统数据，回答「我最担心什么」。

## 要求
- 优先寻找风险，而不是机会
- 关注：regime 转换、过热、流动性恶化、假突破、拥挤交易
- 如果 State=RiskOn 但信号正在减弱，提示「方向正确但收益空间压缩」
- 保留关键事实数字，但绝不输出 confidence、score、ranking
- 禁止输出 BUY/SELL/HOLD 建议

## 输出格式
Markdown 纯文本。不要 JSON。不要结构化字段。
"#;

pub const DEVILS_ADVOCATE_PROMPT: &str = r#"
# 任务：唱反调

你是专门质疑系统结论的研究员。基于以下系统数据，回答「系统可能错在哪里」。

## 要求
- 列举当前量化系统最脆弱的假设
- 如果错了，最大亏损可能来自哪里
- 寻找幸存者偏差、过拟合、数据窥探的证据
- 保留关键事实数字，但绝不输出 confidence、score、ranking
- 禁止输出 BUY/SELL/HOLD 建议
- 风格：建设性质疑，不是否定一切

## 输出格式
Markdown 纯文本。不要 JSON。不要结构化字段。
"#;

// ── Prompt Builder ────────────────────────────────────────────

/// Portfolio review prompt (RV1 Phase 3): explain the deterministic
/// portfolio decision — the action labels come from code, never from LLM.
pub const PORTFOLIO_REVIEW_PROMPT: &str = r#"
# 任务：组合决策解读

系统已通过确定性引擎产出当日的组合姿态建议（Increase / Maintain / Reduce / Avoid）。
这些标签由代码计算，不由你生成，也不可被你修改。

## 你的任务
- 解释「为什么引擎给出这样的姿态」，而不是给出你自己的建议
- 结合每个标的的多策略视角（动量 / 价值 / 趋势突破 / 回调）解释引擎行为是否合理
- 指出不同策略视角之间的矛盾点，以及这些矛盾对持仓者意味着什么
- 保留关键事实数字（策略分数、归因驱动因素），但绝不输出你自己的评分或排名
- 禁止输出 BUY/SELL/HOLD 建议，禁止输出仓位百分比
- 如果引擎在某标的上给出 Avoid 但动量策略强烈看多，解释这种张力而非裁决对错

## 输出格式
Markdown 纯文本。不要 JSON。不要结构化字段。
"#;

/// Resolve a built-in persona (system prompt, template) by action key.
/// Returns `None` for unknown actions so callers can fall back to file-based personas.
pub fn builtin_persona(action: &str) -> Option<(&'static str, &'static str)> {
    match action {
        "market_story" => Some((
            "你是一位资深市场研究员。目标是解释市场发生了什么。只解释、提供上下文，绝不创建信号、评分、排序或修改决策。",
            MARKET_STORY_PROMPT,
        )),
        "explain_decision" => Some((
            "你是一位系统分析师。目标是解释量化系统为什么做出这样的决策。只解释、不评判。",
            EXPLAIN_DECISION_PROMPT,
        )),
        "preclose_review" => Some((
            "你是一位执行分析师。目标是解释 Execution Layer 为什么给出这样的执行建议。只解释、不推荐。",
            PRECLOSE_REVIEW_PROMPT,
        )),
        "risk_view" => Some((
            "你是一位保守型基金风控总监。优先寻找风险，而不是机会。只提示风险、不给出操作建议。",
            RISK_VIEW_PROMPT,
        )),
        "devils_advocate" => Some((
            "你是一位专门质疑系统结论的研究员。你的职责不是证明系统正确，而是寻找最脆弱的假设、可能失效的前提、幸存者偏差、过拟合迹象、数据窥探。保持建设性质疑，而不是否定一切。",
            DEVILS_ADVOCATE_PROMPT,
        )),
        "portfolio_review" => Some((
            "你是一位组合决策顾问。确定性引擎已经产出组合姿态（Increase/Maintain/Reduce/Avoid），你的职责是解释引擎为什么这样判断，结合多策略视角的矛盾点给出背景。你绝不生成自己的买卖建议，绝不修改引擎结论。",
            PORTFOLIO_REVIEW_PROMPT,
        )),
        _ => None,
    }
}

/// 根据 action 字符串构建完整的 LLM prompt。
///
/// 返回 `(system_prompt, user_prompt)` 元组。
pub fn build_prompt(
    action: &str,
    snapshot: &DashboardSnapshot,
) -> Result<(String, String)> {
    let (system_prompt, template) = builtin_persona(action)
        .ok_or_else(|| anyhow::anyhow!("Unknown research action: {}", action))?;
    Ok(build_prompt_with_persona(system_prompt, template, snapshot, None))
}

/// Build a prompt from an explicit persona plus optional extra context sections
/// (RV1 Phase 3: strategy perspectives, integrity status, previous interpretation).
pub fn build_prompt_with_persona(
    system_prompt: &str,
    template: &str,
    snapshot: &DashboardSnapshot,
    extra_context: Option<&str>,
) -> (String, String) {
    let user_prompt = format_user_prompt(template, snapshot, extra_context);
    (system_prompt.to_string(), user_prompt)
}

// ── Helpers ───────────────────────────────────────────────────

fn format_user_prompt(template: &str, snapshot: &DashboardSnapshot, extra_context: Option<&str>) -> String {
    let context = build_snapshot_context(snapshot);
    match extra_context {
        Some(extra) if !extra.trim().is_empty() => {
            format!("{}\n\n## 系统数据\n\n{}\n\n{}", template, context, extra)
        }
        _ => format!("{}\n\n## 系统数据\n\n{}", template, context),
    }
}

fn build_snapshot_context(snapshot: &DashboardSnapshot) -> String {
    let mut ctx = String::new();

    ctx.push_str(&format!("**分析日期**: {}\n", snapshot.report_date));
    ctx.push_str(&format!("**Scope**: {}\n", snapshot.scope));

    ctx.push_str(&format!("**Regime**: {}\n", snapshot.regime_label));
    ctx.push_str(&format!("**流动性评分**: {:.1}\n", snapshot.liquidity_score));
    ctx.push_str(&format!("**Regime 数据新鲜度**: {} 天\n", snapshot.regime_stale_days.max(0)));
    if let Some(ref env) = snapshot.environment {
        ctx.push_str(&format!(
            "**Environment**: {} (breadth: {})\n",
            env.environment_label, env.breadth_state
        ));
        let fmt_opt = |v: Option<f64>| v.map(|x| format!("{:.1}", x)).unwrap_or_else(|| "N/A".to_string());
        ctx.push_str(&format!("**广度 5 日变化**: {}\n", fmt_opt(env.breadth_5d_delta)));
        ctx.push_str(&format!("**成交量扩张**: {}\n", fmt_opt(env.volume_expansion_pct)));
        ctx.push_str(&format!("**换手率覆盖**: {}\n", fmt_opt(env.turnover_coverage_pct)));
    }
    if let Some(ref state) = snapshot.strategy_state {
        ctx.push_str(&format!(
            "**Strategy State**: {} (transition: {})\n",
            state.state, state.transition_reason
        ));
    }

    // Top rotation — 只给排名，不给 RS 分数
    if !snapshot.top_rotation.is_empty() {
        ctx.push_str("\n**Top Rotation**:\n");
        for (i, item) in snapshot.top_rotation.iter().take(5).enumerate() {
            ctx.push_str(&format!("{}. {}\n", i + 1, item.symbol));
        }
    }

    // Bottom rotation — 退潮方向，只给 symbol 名
    if !snapshot.bottom_rotation.is_empty() {
        ctx.push_str("\n**轮动末位（退潮方向）**:\n");
        for (i, item) in snapshot.bottom_rotation.iter().take(5).enumerate() {
            ctx.push_str(&format!("{}. {}\n", i + 1, item.symbol));
        }
    }

    // Top signals — 只给 label，不给 score
    if !snapshot.top_signals.is_empty() {
        ctx.push_str("\n**Top Signals**:\n");
        for (i, item) in snapshot.top_signals.iter().take(5).enumerate() {
            ctx.push_str(&format!("{}. {} ({})\n", i + 1, item.symbol, item.signal_label));
        }
    }

    // Phase1: 不喂 backtest（CAGR/Sharpe/MaxDD），避免 LLM 成为 Meta Decision Layer
    // 90 天观察期后再考虑是否恢复

    ctx
}

// ── Re-exports (简化版) ─────────────────────────────────────

pub use crate::provider::{LlmProvider, LlmCallConfig};
pub use crate::openai_provider::OpenAiProvider;
pub use crate::inference::InferenceConfig;

