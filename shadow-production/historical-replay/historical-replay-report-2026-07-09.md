# Shadow Production 历史复盘测试报告

**测试日期**: 2026-07-09  
**目标**: 通过历史数据复盘加速 TASK-093（StrongBuy + DE_RISK 分歧模式观察）和 Shadow Production 90 天观察期  
**执行范围**: Global / CN / HK  
**数据窗口**: 2017-01-03 至 2026-07-09（Global 覆盖最完整）  
**输出目录**: `shadow-production/historical-replay/`

---

## 1. 测试方法

### 1.1 Plan A：全局条件前向收益分析
- 命令：`quant-cli research analytics --condition srd-strong --scope {global|cn|hk} --horizon {20|60}`
- 输出：所有历史上满足 `srd-strong` 条件日期的 T+20 / T+60 前向收益统计
- 评估指标：median return、mean return、positive ratio、best/worst、median max drawdown

### 1.2 Plan A：多窗口季度研究综述
- 命令：`quant-cli research review --scope global --from <start> --to <end> --output <file>`
- 窗口选择：
  - 2023-04-01 ~ 2023-06-30（早期熊市反弹后回调）
  - 2024-01-01 ~ 2024-06-30（震荡整理期）
  - 2025-01-01 ~ 2025-06-30（慢牛初期）
  - 2025-07-01 ~ 2025-12-31（强势上涨期）
  - 2026-01-01 ~ 2026-06-30（当前观察期）
- 输出：每个窗口的 SRD 天数、频率、持续时间、Stretch 分布、前向收益

### 1.3 Plan B：分歧样本快照扫描
- 命令：`quant-cli symbol-scoreboard --date <YYYY-MM-DD> --scope global`
- 样本日期选择：
  - 2025-12-30（2025 H2 强势上涨期末）
  - 2025-08-15（2025 H2 中期）
  - 2024-05-21（2024 H1 震荡期）
  - 2023-04-20（2023 Q2 回调期）
- 目的：识别具体哪些标的在 SRD 日出现 StrongBuy + DeRisk，观察其行业和主题分布

---

## 2. 测试结果

### 2.1 全历史条件前向收益（Global / CN / HK）

| Scope | Horizon | Occurrences | Median | Mean | Positive Ratio | Best | Worst | Median Max DD |
|-------|---------|-------------|--------|------|----------------|------|-------|---------------|
| Global | 20d | 212 | +0.2% | +0.1% | 51.9% | +9.8% | -9.0% | 4.0% |
| Global | 60d | 184 | -0.5% | +0.7% | 45.7% | +18.0% | -10.0% | 7.6% |
| CN | 20d | 193 | +0.7% | +0.4% | 57.5% | +9.0% | -9.0% | 3.7% |
| HK | 20d | 0 | N/A | N/A | N/A | N/A | N/A | N/A |

**关键发现**：
- Global 全历史 SRD-strong 是一个**弱随机信号**：H20 positive ratio 51.9%，H20 median 接近 0，H60 median 甚至略负。
- CN 表现略好于 Global（H20 positive ratio 57.5%，median +0.7%），但仍远非稳定盈利信号。
- HK 在 20d 条件下 0 次触发，说明 HK 市场的 SRD-strong 条件极其罕见，或 State 与 Signal 在 HK 高度一致。

### 2.2 分窗口季度研究综述（Global）

| 窗口 | SRD Days | SRD Freq | Avg Duration | Longest Streak | H20 Median | H20 Mean | H20 Positive | H60 Median | H60 Mean | 评估 |
|------|----------|----------|--------------|----------------|------------|----------|--------------|------------|----------|------|
| 2023 Q2 | 13 | 21.0% | 68.0d | 13d | **-3.1%** | **-3.1%** | **0.0%** | **-6.2%** | **-6.1%** | 强烈负向：State 保守正确 |
| 2024 H1 | 12 | 9.7% | 10.3d | 9d | **-2.7%** | **-2.8%** | **0.0%** | **-8.5%** | **-7.3%** | 强烈负向：State 保守正确 |
| 2025 H1 | 25 | 20.5% | 5.8d | 11d | +0.2% | -1.0% | 60.0% | -0.9% | -0.1% | 混合/略负 |
| 2025 H2 | 65 | **50.0%** | 11.8d | 11d | **+2.8%** | **+2.6%** | **84.6%** | **+2.4%** | **+5.4%** | 强烈正向：Candidate Evidence（潜在失效窗口） |
| 2026 H1 | 51 | 41.8% | 10.8d | 18d | -0.1% | -0.3% | 45.8% | +1.4% | +1.1% | 混合/略正 |

**关键发现**：
- **2025 H2 是异常窗口**：SRD 频率高达 50%（即一半交易日都是 SRD），且 StrongBuy + DeRisk 后 20 天正收益比例 84.6%，中位数 +2.8%。这**不是**已确认的 Kill Criterion，而是重要的 **Candidate Evidence**（候选证据）。单一窗口不能触发 Kill Criterion，否则容易陷入过拟合；但它证明 State Layer 在特定市场阶段存在系统性保守倾向。
- 2023 Q2 和 2024 H1 是反例：SRD 出现后 20/60 天全部负收益，说明 State Layer 保守保护有效。
- 2026 H1 接近随机，这与当前系统观察一致（数据 freshness 正常，但信号与状态分歧没有系统性预测力）。

### 2.3 分歧样本快照（symbol-scoreboard）

#### 2025-12-30（2025 H2 强势期末）
| Rank | Symbol | Name | Score | Signal | Momentum | Regime |
|------|--------|------|-------|--------|----------|--------|
| 1 | 512400 | 有色ETF | 89.3 | Strong Buy | 24.99 | DeRisk |
| 2 | 515880 | 通信ETF | 89.3 | Strong Buy | 34.42 | DeRisk |
| 3 | 515070 | 人工智能ETF | 88.5 | Strong Buy | 15.97 | DeRisk |
| 4 | 399673 | 创业板50 | 86.5 | Strong Buy | 15.45 | DeRisk |
| 5 | 516150 | 嘉实中证稀土产业ETF | 84.5 | Strong Buy | 15.80 | DeRisk |
| 6 | 399006 | 创业板指 | 83.9 | Strong Buy | 13.29 | DeRisk |

**特征**：科技/成长/小盘主题高度集中，Momentum 极高，但 Regime 全部 DeRisk。后续 20 天市场继续上涨，State Layer 错失机会。

#### 2025-08-15（2025 H2 中期）
| Rank | Symbol | Name | Score | Signal | Momentum | Regime |
|------|--------|------|-------|--------|----------|--------|
| 1 | 512400 | 有色ETF | 90.2 | Strong Buy | 20.81 | DeRisk |
| 2 | 515880 | 通信ETF | 90.2 | Strong Buy | 34.05 | DeRisk |
| 3 | 516150 | 稀土产业ETF | 90.2 | Strong Buy | 26.52 | DeRisk |
| 4 | 159851 | 金融科技ETF | 89.9 | Strong Buy | 17.65 | DeRisk |
| 5 | 000698 | 科创100 | 88.9 | Strong Buy | 17.35 | DeRisk |
| 6 | 399673 | 创业板50 | 87.7 | Strong Buy | 16.89 | DeRisk |
| 7 | 512000 | 券商ETF | 85.6 | Strong Buy | 15.27 | DeRisk |
| 8 | 399006 | 创业板指 | 84.6 | Strong Buy | 15.03 | DeRisk |
| 9 | 000852 | 中证1000 | 80.3 | Strong Buy | 11.59 | DeRisk |

**特征**：分歧极其广泛，甚至大盘指数（中证1000、创业板指）都是 Strong Buy + DeRisk。这是典型的 State Layer 因风险阈值过高而全面压制的阶段。

#### 2024-05-21（2024 H1 震荡期）
| Rank | Symbol | Name | Score | Signal | Momentum | Regime |
|------|--------|------|-------|--------|----------|--------|
| 1 | 513050 | 中概互联网ETF | 92.1 | Strong Buy | 21.47 | DeRisk |
| 2 | HSCEI | 恒生国企指数 | 92.1 | Strong Buy | 18.87 | DeRisk |
| 3 | HSTECH | 恒生科技指数 | 92.1 | Strong Buy | 17.02 | DeRisk |
| 4 | 512400 | 有色ETF | 84.2 | Strong Buy | 10.08 | DeRisk |
| 5 | 512800 | 银行ETF | 81.1 | Strong Buy | 9.22 | DeRisk |

**特征**：HK/中概股 + 银行/有色周期板块。后续 20/60 天全部负收益，说明 DeRisk 对周期股和外资敏感板块的保护有效。

#### 2023-04-20（2023 Q2 回调期）
| Rank | Symbol | Name | Score | Signal | Momentum | Regime |
|------|--------|------|-------|--------|----------|--------|
| 1 | 159851 | 金融科技ETF | 88.2 | Strong Buy | 19.50 | DeRisk |
| 2 | 512480 | 半导体ETF | 88.2 | Strong Buy | 21.19 | DeRisk |
| 3 | 515070 | 人工智能ETF | 88.2 | Strong Buy | 28.93 | DeRisk |
| 4 | 515880 | 通信ETF | 88.2 | Strong Buy | 24.02 | DeRisk |
| 5 | 159995 | 芯片ETF | 87.9 | Strong Buy | 18.63 | DeRisk |
| 6 | 000688 | 科创50 | 84.2 | Strong Buy | 15.03 | DeRisk |

**特征**：AI/芯片/通信主题炒作尾声。后续 20/60 天全部负收益，State Layer 成功规避了主题退潮。

---

## 3. 系统功能评估

### 3.1 State Layer v1.0（ADR-065 冻结）

| 评估维度 | 评分 | 说明 |
|----------|------|------|
| 行为稳定性 | 高 | 冻结后状态转移逻辑一致，没有漂移或异常跳变 |
| 保守性 | 高 | 在 2023 Q2、2024 H1 的回调中，DeRisk 保护有效 |
| 适应性 | 中低 | 在 2025 H2 的持续上涨中，DeRisk 出现系统性压制，导致 StrongBuy 信号被大量抑制 |
| 预测准确性 | 窗口依赖 | 2023-2024 表现优秀，2025 H2 表现差，2026 H1 接近随机 |

**核心判断**：State Layer v1.0 不是"对"或"错"的问题，而是**在特定市场阶段（高动量、低波动、主题扩散）过度保守**。这正符合 TASK-093 的设计目标：观察 StrongBuy + DE_RISK 是否在长期有正前向收益。

### 3.2 `research-srd` / `research-stretch` 工具

| 评估维度 | 评分 | 说明 |
|----------|------|------|
| 数据可用性 | 高 | 支持 `--date` 参数，可跑任意历史日期 |
| 统计稳定性 | 中 | SRD 频率和持续时间在 2025 H2 明显异常，工具能捕捉这种变化 |
| 可解释性 | 高 | 输出 Duration、Breadth trend、Rotation pattern、Historical percentile，便于归因 |
| 预测性 | 低 | 单独 SRD 信号无法稳定预测未来收益，但可作为观察状态的维度 |

### 3.3 `research-analytics` 工具

| 评估维度 | 评分 | 说明 |
|----------|------|------|
| 历史覆盖 | 高 | 自动使用全量历史数据，无需手动指定日期范围 |
| 前向收益计算 | 高 | 正确计算 T+20/T+60 的 median/mean/best/worst/positive ratio/max drawdown |
| 条件支持 | 中 | 当前仅支持 `srd-strong` 和 `stretch-extreme-crowding-momentum`，扩展性有限 |
| 对 TASK-093 的价值 | 高 | 直接回答核心问题：StrongBuy + DeRisk 后是否有正前向收益 |

### 3.4 `symbol-scoreboard` / `symbol-diagnostics` 工具

| 评估维度 | 评分 | 说明 |
|----------|------|------|
| 历史回溯 | 高 | 支持 `--date` 参数，可跑历史日期 |
| 归因清晰度 | 高 | 输出 Strategy / Alignment / Regime / Rotation 四段贡献 |
| 分歧识别 | 高 | 可直接筛选 StrongBuy + DeRisk 标的 |
| 跨主题对比 | 中 | 能横向对比所有标的，但无法自动聚类或统计主题偏好 |

### 3.5 `research-review` 工具

| 评估维度 | 评分 | 说明 |
|----------|------|------|
| 窗口聚合 | 高 | 可指定任意 `--from/--to`，自动跳过无数据日期 |
| 输出完整性 | 高 | 聚合 SRD、Stretch、Analytics 三类信息 |
| 文件输出 | 高 | 支持 `--output` 输出 JSON，便于后续分析 |
| 对 Shadow Production 的价值 | 高 | 相当于把 90 天观察期压缩成一次命令执行 |

### 3.6 对当前研究框架的深层评估：从 Model Right/Wrong 到 Model Bias

本次历史复盘最重要的结论不是 State Layer "对" 或 "错"，而是揭示了 **Regime Dependency（市场状态依赖）** 和 **Model Bias（模型偏置）**。

#### 3.6.1 Regime Dependency 的证据

| 市场阶段 | SRD 行为 | State Layer 表现 | 解释 |
|---------|----------|-------------------|------|
| 2023 Q2（主题退潮/回调） | 全部负收益 | 保守正确 | DeRisk 规避了 AI/芯片主题退潮 |
| 2024 H1（震荡/外资敏感） | 全部负收益 | 保守正确 | DeRisk 规避了周期股和 HK/中概回调 |
| 2025 H2（高动量/低波动/成长扩散） | 84.6% 正收益 | 过度保守 | DeRisk 压制了整个 Growth/Theme 板块 |
| 2026 H1（当前观察期） | 接近随机 | 中性 | 市场分歧没有明确方向 |

这说明：
> **State Layer 不是泛化失效，而是在"高动量 + 低波动 + 主题扩散"的 Regime 下出现系统性 Under-react。**

这是典型的量化模型 Regime Dependency，与 Momentum 在熊市有效/牛市失效、Value 在震荡市有效等经典问题一致。

#### 3.6.2 Systematic Bias 的证据

从 `symbol-scoreboard` 快照可以看出，2025 H2 的 SRD 不是单个标的的偶然现象，而是**整个主题的系统性压制**：

- 2025-08-15：创业板、科创100、中证1000、券商、通信、人工智能等 **全部 StrongBuy + DeRisk**
- 2025-12-30：有色、通信、人工智能、创业板、稀土等 **全部 StrongBuy + DeRisk**

这意味着：
> **当 Growth / SmallCap / Momentum 主题同时处于高动量状态时，State Layer 的风险阈值会系统性地压制整个主题。**

这不是单个信号的误差，而是模型对特定市场结构的系统性偏见。

#### 3.6.3 从 Prediction 到 Explanation：下一步需要 Failure Attribution 层

当前 `research-analytics` 只能回答：
> Condition X → Forward Return Y

下一步应该回答：
> Condition X → Forward Return Y → 为什么在这个 Regime 下有效/失效？

需要引入的 Failure Attribution 维度：
- Breadth 水平（% 标的站上均线）
- Liquidity 压力（spread、turnover、proxy）
- Volatility 水平（VIX、realized vol）
- Macro Regime（RiskOn/Neutral/RiskOff）
- Economic Layer 状态（Favorable/Neutral/Unfavorable）
- Theme dispersion / Crowding 程度
- Leadership stability

目标输出示例：
```
Condition: srd-strong
Positive Ratio: 84.6% (in 2025 H2)
Best Environment:
  - Breadth > 60%
  - Liquidity proxy > 70
  - Volatility < 30 percentile
  - Macro Regime = RiskOn
  - Economic Layer = Favorable
  - Crowding = Elevated but not Extreme
Failure Attribution:
  - State Layer under-reacts when breadth is broad and momentum is broad-based
  - DeRisk threshold is too conservative in low-volatility RiskOn regimes
```

这会将 Research Layer 从"预测工具"升级为"可解释的研究平台"。

---

## 4. 对 Shadow Production 和 TASK-093 的影响

### 4.1 是否触发 Kill Criteria？

**Kill Criterion #1**：Persistent StrongBuy + `DE_RISK` divergence with positive forward returns across **multiple** symbols and **multiple** windows.

- **2025 H2 的数据**：SRD 频率 50%，H20 positive ratio 84.6%，median +2.8%。这是一个**强烈的 Candidate Evidence**（候选证据），但**尚不构成已确认的 Kill Criterion**。Kill Criterion 需要跨多个独立窗口重复出现，单一窗口可能只是特殊市场阶段（如 AI/成长主题驱动的牛市）的偶发现象。
- **但需要注意**：这是**历史复盘**，不是当前 90 天观察期。Shadow Production Playbook 要求的是当前运行的 90 天观察期证据。
- **结论**：2025 H2 的历史证据表明，在特定市场阶段 State Layer 确实过度保守，但**不能提前终止当前观察期**。建议将这一证据作为季度 ADR review 的 Candidate Evidence，而非 Kill Criterion 触发记录。

### 4.2 对 90 天观察期的加速价值

- 历史复盘证明：单纯等待 90 天可能会错过识别这种阶段性失效（如果当前 90 天正好是 2023 或 2024 类型的窗口，可能得出"State Layer 正确"的错误结论）。
- 建议：**在真实 90 天观察期结束后，将历史复盘结果作为背景证据，与真实观察期合并评估**。如果当前 90 天也出现类似 2025 H2 的模式，Kill Criterion 被触发的概率会显著上升。

### 4.3 对 FROZEN 任务的解锁影响

- 历史复盘**不能提前解锁** TASK-000 ~ TASK-004 或 TASK-081。
- 但可以为这些任务提供证据：
  - TASK-000（Regime Threshold Calibration）：2025 H2 的过度保守说明阈值可能需要调整。
  - TASK-081（6 missing factors）：可分析 HY Spread、Term Spread、SOFR 等因子在 2025 H2 是否有解释力，帮助设计后续因子集成。

### 4.4 历史复盘与 Shadow Production 的定位

- **Historical Replay（历史复盘）**：负责提出假设、寻找历史证据、识别 Candidate Evidence。它可以压缩"数据收集"阶段，但不能替代真实观察。
- **Shadow Production（90 天真实观察）**：负责验证这些假设是否在未来仍然成立。它能捕捉历史回测无法覆盖的数据漂移、市场结构变化、执行一致性问题。
- 两者形成闭环：Historical Replay 提出"State Layer 在 2025 H2 过度保守"的假设；Shadow Production 验证"当前市场是否正在重复这一模式"。

### 4.5 项目定位的演变：从交易系统到 Quant Research Platform

本次测试进一步证明，项目的长期价值可能正在从 **Strategy Layer** 向 **Research Layer** 转移：

- **Strategy Layer**：Signal 和 Execution 可以持续替换和优化。
- **Research Layer**：Observation → Evolution → Evidence → Consensus 的框架，以及 Historical Replay / Failure Attribution 能力，正在成为可复用的研究基础设施。

未来核心竞争力的方向可能包括：
- 更丰富的条件分析（Condition Analytics）
- Regime / 宏观归因（Failure Attribution）
- 自动生成历史研究报告（Historical Replay）
- 多市场、多主题、多窗口的统计对比
- 最终形成可解释、可复用的研究资产

这比单纯追求策略收益率更符合项目当前的架构演进方向。

---

## 5. 关键风险与注意事项

1. **历史复盘 ≠ 未来预测**：2025 H2 的数据不能证明未来也会出现同样模式。
2. **模型使用当前冻结参数**：复盘是在 State Layer v1.0 固定参数上跑历史，不存在"用未来数据优化"的问题，但也没有引入新的模型失效证据。
3. **HK 数据不足**：HK 的 SRD-strong 0 次触发，说明 HK 样本无法支持结论，需要更多 HK 历史数据或更长时间的真实观察。
4. **数据质量**：早期数据（2023-2024）可能存在 turnover 缺失、Eastmoney fallback 等问题，影响 signal 和 rotation 的准确性。
5. **统计显著性**：单个窗口（2025 H2）的 65 个样本仍有限，需要多个类似窗口才能确认系统性失效。

---

## 6. 建议

### 6.1 立即行动

1. **将 2025 H2 的复盘结果作为季度 ADR review 材料**：
   - 文件位置：`shadow-production/historical-replay/review-global-2025-h2.json`
   - 重点：84.6% H20 positive ratio、median +2.8%、SRD 频率 50%。

2. **持续真实 90 天观察**：
   - 不要因历史复盘提前终止 Shadow Production。
   - 继续每日 `research-srd/stretch` + `symbol-diagnostics`，每周 `symbol-scoreboard` + `research analytics`。

3. **建立分歧样本库**：
   - 将本次扫描的 4 个代表性日期扩展到更多历史日期。
   - 输出格式：`shadow-production/divergence-sample-library.csv`（Symbol, Date, Score, Signal, State, T+20 return, T+60 return, Theme）。

### 6.2 中期优化

1. **扩展 `research-analytics` 条件**：
   - 当前仅支持 `srd-strong` 和 `stretch-extreme-crowding-momentum`。
   - 建议新增条件：`strongbuy-derisk-multiple-symbols`（同时满足多个标的 StrongBuy + DeRisk）。

2. **自动化历史复盘脚本**：
   - 将本次手动执行的命令封装为脚本，定期运行并对比不同窗口。
   - 脚本应输出：各窗口 SRD 频率、forward return 统计、Candidate Evidence 出现次数。

3. **增加主题聚类**：
   - `symbol-scoreboard` 输出已经显示主题分布（科技、周期、银行等）。
   - 建议增加自动主题标签，便于判断哪些主题在 SRD 中更容易产生正/负前向收益。

4. **引入 Failure Attribution / Regime Attribution 层**：
   - 当前 `research-analytics` 只回答"Condition X → Forward Return Y"。
   - 下一步应回答"Condition X → Forward Return Y → 在什么 Regime 下有效/失效"。
   - 需要集成的维度：Breadth、Liquidity、Volatility、Macro Regime、Economic Layer、Theme Dispersion、Crowding。
   - 输出形式：在 positive ratio 之外，输出"最佳环境"和"失效归因"。
   - 这是将 Research Layer 从"预测工具"升级为"可解释研究平台"的关键步骤。
### 6.3 对 State Layer 的评估结论

- **当前状态**：State Layer v1.0 在 2023-2024 表现良好，但在 2025 H2 出现明显过度保守。
- **是否建议调整**：在 Shadow Production 结束前**不建议调整**，因为流程上需要 90 天观察期证据。
- **但应准备 ADR 提案**：如果当前 90 天观察期也出现类似 2025 H2 的模式，应提交 ADR 提案，建议重新校准 DeRisk 阈值或引入动态风险调整。

---

## 7. 输出文件清单

| 文件 | 说明 |
|------|------|
| `shadow-production/historical-replay/analytics-srd-strong-global-h20.txt` | Global SRD-strong T+20 全历史收益统计 |
| `shadow-production/historical-replay/analytics-srd-strong-global-h60.txt` | Global SRD-strong T+60 全历史收益统计 |
| `shadow-production/historical-replay/analytics-srd-strong-cn-h20.txt` | CN SRD-strong T+20 全历史收益统计 |
| `shadow-production/historical-replay/analytics-srd-strong-hk-h20.txt` | HK SRD-strong T+20 统计（0 次触发） |
| `shadow-production/historical-replay/review-global-2023-q2.json` | 2023 Q2 季度研究综述 |
| `shadow-production/historical-replay/review-global-2024-h1.json` | 2024 H1 季度研究综述 |
| `shadow-production/historical-replay/review-global-2025-h1.json` | 2025 H1 季度研究综述 |
| `shadow-production/historical-replay/review-global-2025-h2.json` | 2025 H2 季度研究综述 |
| `shadow-production/historical-replay/review-global-2026-h1.json` | 2026 H1 季度研究综述 |
| `shadow-production/historical-replay/scoreboard-2025-12-30.txt` | 2025-12-30 全标快照 |
| `shadow-production/historical-replay/scoreboard-2025-08-15.txt` | 2025-08-15 全标快照 |
| `shadow-production/historical-replay/scoreboard-2024-05-21.txt` | 2024-05-21 全标快照 |
| `shadow-production/historical-replay/scoreboard-2023-04-20.txt` | 2023-04-20 全标快照 |
| `shadow-production/historical-replay/historical-replay-report-2026-07-09.md` | 本报告 |

---

## 8. 总结

本次历史复盘为 Shadow Production 提供了关键先验证据：

- **SRD-strong 全历史不是稳定盈利信号**（Global H20 positive ratio 51.9%，接近随机）。
- **但存在明显的 Regime Dependency（市场状态依赖）**：2025 H2 出现 SRD 频率 50%、H20 positive ratio 84.6% 的异常窗口，是重要的 **Candidate Evidence**；2023 Q2、2024 H1 则是反例。
- **State Layer v1.0 不是"对"或"错"，而是在特定市场阶段过度保守**——这是 Model Bias，而不是模型崩溃。
- 系统工具链功能正常，支持历史复盘，可直接用于 TASK-093 和后续 Failure Attribution 研究。

**最终建议**：继续执行 90 天真实观察期，但将本次历史复盘结果作为背景证据。如果当前观察期也出现类似 2025 H2 的模式，应提交 ADR 提案重新评估 State Layer 阈值，并启动 Failure Attribution 层设计。
