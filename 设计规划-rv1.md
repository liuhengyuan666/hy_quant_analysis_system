# 设计规划 RV1 — 能力收敛与每日组合决策助手

> **分支**：`rv1`（从 `main` 切出）
> **定位**：从 V8 "Execution Platform" 收敛为 **Daily Portfolio Decision Assistant**（每日组合决策辅助系统）
> **核心原则**：真实场景是「每日收盘 → 判断趋势健康度 → 决定加仓/持有/减仓/等待」——不是实时交易、不是订单路由、不是执行平台。

---

## 一、系统定位重新对齐

| 维度 | V8 | RV1 |
|------|----|----|
| 系统自称 | Execution Platform | **Daily Portfolio Decision Assistant** |
| 核心产出 | ExecutionEvent, ShadowRiskAssessment | **Market Evidence + Strategy Perspectives + Portfolio Action** |
| 决策输出 | BuyNow / Wait / NoChase | **Increase / Maintain / Reduce / Avoid + 依据 + 风险提示** |
| 日常命令数 | 107 | **~10 核心 + ~9 隐藏** |
| 用户每日流程 | 记多个命令 | **3 条：market-refresh → daily-analysis → daily-report** |

### 为什么不是 Execution Platform

Execution Platform 隐含订单路由、滑点、成交、盘口管理——但真实场景是场外基金收盘决策，不需要这些。V8 的 `execution-replay`、`shadow-deployment`、`holding-risk-bundle` 等模块，本质上是一个"风险研究平台"被错放在了"执行"的标签下。

### 核心资产（不可删除）

- **Evidence Asset System**：可持久化、可复现、可审计的研究证据
- **Context Integrity Firewall**：输入数据完整性校验——之前模型失败往往不是因为模型错，而是输入是假数据
- **Horizon/Role Model**：不同证据有不同的时间角色，长期风险指标不应触发短期卖出

---

## 二、实施阶段

### Phase 1：减法 + 重命名 + Integrity 集成（当前）

#### 2.1 CLI 命令归类

**核心命令（~10 个，README 推荐）：**

```
每日三件套：
  market-refresh           ← 全链路数据刷新（替代 refresh-all）
  daily-analysis           ← 一键产出：Research Evidence + 多策略评分 + 风险快照 + 组合建议
  daily-report             ← 导出日报（替代 export-report）

深度分析：
  strategy-perspectives    ← 多策略独立评分 + 场景对比 + 归因
  risk-assessment          ← 持仓风险评估
  portfolio-decision       ← 当日组合操作建议（替代 preclose-analysis）

证据与验证：
  evidence-status          ← 查看 Evidence 资产状态
  validation-check         ← 校准基线验证
  historical-replay        ← 历史条件回放（替代 research replay）

LLM：
  llm-analyze              ← LLM 多视角市场分析（替代 analyze-with-llm）
```

**隐藏命令（~9 个，`--help` 可发现，README 不推荐）：**

```
research-srd             ← SRD 单独查询
research-stretch         ← Stretch 单独查询
research-calibration     ← 校准基线检查
pipeline-dates           ← 管线状态诊断
data-health              ← 数据健康检查
symbol-diagnostics       ← 单标的诊断
symbol-scoreboard        ← 全市场排行榜
rotation-ranking         ← 轮动排名
dashboard-snapshot       ← 查看历史快照
```

#### 2.2 移除的 CLI 命令（~50+ 个，底层 crate 逻辑保留）

- **审计类**（~35 个）：`audit-*` 全部移除，底层 `regime-audit` + `research-validation` crate 保留
- **V8 Execution 实验**（~20 个）：`execution-*` 全部移除，底层逻辑保留
- **Shadow Production**（4 个）：`shadow-mode`, `shadow-deployment`, `risk-lifecycle`, `holding-risk-calibration`
- **重复/废弃**（3 个）：`execution-leadership-decay-horizon`, `regime-risk`, `state-risk-acceleration`

#### 2.3 DecisionEngine 输出语义变更

```
旧：BuyNow / Wait / NoChase / Reduce / Skip
新：Increase / Maintain / Reduce / Avoid

每个 action 附带:
  - confidence: 置信度
  - evidence: 支撑证据列表
  - risk_note: 风险提示
```

#### 2.4 Context Integrity Firewall

`daily-analysis` 命令执行流程：

```
Step 0: Context Integrity Gate
  ├─ 检查关键数据源可达性
  ├─ 检查 feature 是否有变化（hash 对比）
  ├─ 检查输入数据完整性
  └─ 输出: integrity_status (PASS / DEGRADED / FAIL)

Step 1: Research Evidence（聚合 SRD + Stretch + Analytics + Health）
Step 2: Strategy Perspectives（多策略独立评分，当前输出已有的四策略分数）
Step 3: Risk Assessment（持仓风险 + 市场风险）
Step 4: Portfolio Action（组合操作建议）
```

### Phase 2：策略引擎重构 — 多策略独立评分 + 场景化（后续）

- 策略引擎不再合并四策略为一个总分，改为独立输出四套评分 + 归因
- 新增场景配置（`config/scenarios.toml`）：短线动量博弈 / 长线价值配置 / 激进博弈
- 新增命令 `strategy-perspectives`：多策略全市场排行 + 单标的详细归因

### Phase 3：LLM 增强 + 组合决策重构（后续）

- LLM 上下文增强：多策略矛盾点 + 历史参照 + 连续性上下文
- 对话历史持久化：上次结论 vs 现在变化
- Prompt 模板化：用户可定义"短线交易员"、"长线配置者"等分析人格
- portfolio-decision 用 LLM 替代硬编码 Pattern Library

---

## 三、预期效果

| 维度 | V8 | RV1 |
|------|----|----|
| CLI 命令数 | 107 | ~10 核心 + ~9 隐藏 |
| 策略输出 | 1 个模糊平均分 | 4 套独立评分 + 归因 + 场景加权 |
| 决策语义 | BuyNow/Wait/NoChase | Increase/Maintain/Reduce/Avoid |
| 数据可信度 | 无标注 | 每次输出标注 integrity 状态 |
| LLM 上下文 | 单一 label + rank | 多策略矛盾点 + 历史参照 + 连续性上下文 |
| 日常命令流 | 记多个命令 | 3 条：market-refresh → daily-analysis → daily-report |

---

## 四、不变的部分

- V6 Reporting Platform（冻结）
- V7 Research Platform（冻结）
- `regime-audit`、`research-validation` 等底层 crate 逻辑完整保留
- ClickHouse 存量数据不受 schema 变更影响（`#[serde(default)]` / `#[serde(alias)]`）
