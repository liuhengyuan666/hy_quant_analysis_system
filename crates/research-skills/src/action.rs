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

/// 根据 action 字符串构建完整的 LLM prompt。
///
/// action 必须是以下之一：
/// - "market_story"
/// - "explain_decision"
/// - "preclose_review"
/// - "risk_view"
/// - "devils_advocate"
///
/// 返回 `(system_prompt, user_prompt)` 元组。
pub fn build_prompt<'a>(
    action: &str,
    snapshot: &DashboardSnapshot,
) -> Result<(&'static str, String)> {
    let (system_prompt, template) = match action {
        "market_story" => (
            "你是一位资深市场研究员。目标是解释市场发生了什么。只解释、提供上下文，绝不创建信号、评分、排序或修改决策。",
            MARKET_STORY_PROMPT,
        ),
        "explain_decision" => (
            "你是一位系统分析师。目标是解释量化系统为什么做出这样的决策。只解释、不评判。",
            EXPLAIN_DECISION_PROMPT,
        ),
        "preclose_review" => (
            "你是一位执行分析师。目标是解释 Execution Layer 为什么给出这样的执行建议。只解释、不推荐。",
            PRECLOSE_REVIEW_PROMPT,
        ),
        "risk_view" => (
            "你是一位保守型基金风控总监。优先寻找风险，而不是机会。只提示风险、不给出操作建议。",
            RISK_VIEW_PROMPT,
        ),
        "devils_advocate" => (
            "你是一位专门质疑系统结论的研究员。你的职责不是证明系统正确，而是寻找最脆弱的假设、可能失效的前提、幸存者偏差、过拟合迹象、数据窥探。保持建设性质疑，而不是否定一切。",
            DEVILS_ADVOCATE_PROMPT,
        ),
        _ => return Err(anyhow::anyhow!("Unknown research action: {}", action)),
    };

    let user_prompt = format_user_prompt(template, snapshot);
    Ok((system_prompt, user_prompt))
}

// ── Helpers ───────────────────────────────────────────────────

fn format_user_prompt(template: &str, snapshot: &DashboardSnapshot) -> String {
    let context = build_snapshot_context(snapshot);
    format!("{}\n\n## 系统数据\n\n{}", template, context)
}

fn build_snapshot_context(snapshot: &DashboardSnapshot) -> String {
    let mut ctx = String::new();

    ctx.push_str(&format!("**分析日期**: {}\n", snapshot.report_date));
    ctx.push_str(&format!("**Scope**: {}\n", snapshot.scope));

    ctx.push_str(&format!("**Regime**: {}\n", snapshot.regime_label));
    if let Some(ref env) = snapshot.environment {
        ctx.push_str(&format!("**Environment**: {:?}\n", env));
    }
    if let Some(ref state) = snapshot.strategy_state {
        ctx.push_str(&format!("**Strategy State**: {:?}\n", state));
    }

    // Top rotation — 只给排名，不给 RS 分数
    if !snapshot.top_rotation.is_empty() {
        ctx.push_str("\n**Top Rotation**:\n");
        for (i, item) in snapshot.top_rotation.iter().take(5).enumerate() {
            ctx.push_str(&format!("{}. {}\n", i + 1, item.symbol));
        }
    }

    // Top signals — 只给 label，不给 score
    if !snapshot.top_signals.is_empty() {
        ctx.push_str("\n**Top Signals**:\n");
        for (i, item) in snapshot.top_signals.iter().take(5).enumerate() {
            ctx.push_str(&format!("{}. {} ({:?})\n", i + 1, item.symbol, item.signal_label));
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

