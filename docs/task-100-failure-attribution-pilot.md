# TASK-100 失败归因试点参考文档

> 本文件是 TASK-100（失败归因试点，Failure Attribution Pilot）第一轮收敛后的**被跟踪持久参考**。
>
> - 记录日期：2026-09-07
> - 任务状态：TASK-100 仍为 InProgress（未宣布完成）
> - 治理来源：ADR-067 / ADR-077 / ADR-078（条款以 `memory/decisions.md` 为准）
> - 复核结论：方法论 PASS；事实可追溯性 PASS（修正 leverage 字面语义后）
>
> 生成型试点报告见 `reports/attribution/attribution-cn-strongbuy-de-risk-2026-02-25.md`。但 `reports/` 是 gitignored 运行产物，不进入版本库；**本文件才是进入版本库的持久记录**。

---

## 1. 试点目标

TASK-100 试点研究一类已知张力：scope 处于 DE_RISK（降风险）状态时，部分标的仍发出 StrongBuy 信号（下称 divergence 事件）。试点想弄清楚两件事：

1. 这类事件的后续表现能否被**可追溯地记录**：用确定性引擎产出的前向收益，按固定窗口记账，形成可复核的 ledger。
2. 有哪些 context / 环境字段**真的支撑** symbol 级归因，哪些只是字面读数、episode 级代理或完全不可用。

试点是方法论试点，不是结论试点。它不判定任何单条记录"成功/失败"，不给买卖建议，不定义失败类别词汇，不调整任何阈值。

---

## 2. 治理来源与边界（ADR-067 / 077 / 078）

试点在 ADR-067、ADR-077、ADR-078 划定的研究治理边界内运行。各条 ADR 的具体条款见 `memory/decisions.md`，本文档只记录试点如何受其约束：

- **ADR-077（V7 Research Platform 1.0）**是确定性研究平台的依据：所有进入 ledger 的证据必须来自确定性引擎计算；试点只做下游观察，不修改平台语义。
- **ADR-067 / ADR-078** 提供试点邻近的研究与证据边界约束：试点的职责是**观察与记录**，不是产生新的决策语义，也不把单一试点的结果当作可推广规律。
- 三条 ADR 的共同落点：试点是"积累证据并暴露问题"的手段；任何类别化、评分或可复用词汇的结论都必须走独立证据复核与 ADR 程序，不能由本试点直接冻结。

---

## 3. 确定性 divergence ledger 契约

试点使用的 divergence ledger 契约如下（记录口径，非代码级规范）：

- **命中谓词**：scope 当日状态为 DE_RISK，且标的当日信号为 StrongBuy。
- **记账维度**：精确日期、scope、symbol。divergence ledger 是逐标的台账，与 `shadow-master.csv` 日主台账并存。
- **outcome**：前向收益按 T20 / T60 / T120 三档记录，全部来自确定性计算，非 LLM 产出。
- **两条正交的状态轴（不混为一谈）**：
  - **outcome 成熟度（按 symbol × 窗口，即单个 horizon outcome）**：`Pending`（前向窗口未走完）/ `Filled`（前向数据齐备、outcome 已计算）/ `Unavailable`（前向数据不可得）。
  - **case 分类（按 ledger 记录，即某 symbol 在该 episode 的一条记录）**：初始一律 **unclassified**（字段状态 `unclassified / null / null`）；只有后续人工/复核流程才可能赋予分类。试点当前不引入任何分类。
- **落盘位置**：`workspace/divergence-ledger/`（gitignored 运行目录）；版本库内的持久参考以本文档为准。
- **写入边界**：只有确定性引擎输出可入 ledger；LLM 叙事与人工解读不是证据，不写入。

---

## 4. 首批队列身份与状态（8 条 ledger 记录 × 24 个 horizon outcome）

- 队列 = **一个** episode：CN scope，2026-02-25，scope 状态 DE_RISK。
- 该 episode 内嵌套 8 个命中标的，合计 **8 个符号**。
- 关键口径：这是 **1 个 episode + 8 个嵌套 symbol**，**不是 8 个独立 episode**。统计上不能按 8 个独立样本来推断。
- ledger 记录共 **8 条**（每 symbol 一条）。每条 ledger 记录含 T20 / T60 / T120 三个 horizon outcome，合计 **24 个 horizon outcome**。
- 24 个 horizon outcome 全部 **Filled**：前向数据齐备，outcome 均已计算；无 Pending、无 Unavailable。
- 分类按 8 条 ledger 记录进行：全部 8 条保持**未分类（unclassified / null / null）**，不标注成功/失败，不套用任何失败模式标签。horizon outcome 的 Filled 只表示数据齐备，不等于已分类。

---

## 5. 试点局限（回顾式 / 非预注册 / 非盲）

首批 cohort 存在三类方法论局限，试点发现只在局限范围内有效：

1. **回顾式**：episode 与 outcome 都已发生后才做归因分析，不是事前声明假设。
2. **非预注册**：观察口径与判定规则没有在 outcome 成熟前冻结。
3. **非盲**：执行者在记录 outcome 时知道事件背景与假设方向。

因此本批结果的定位是**有界观察**，不是可推广证据。

---

## 6. 有界发现

以下发现只描述首批 cohort（单 episode），不推广：

- **T20**：8 个 T20 outcome 中 2 个正向、6 个负向。
- **T60**：8 个 T60 outcome 中 4 个正向、4 个负向。
- **T120**：8 个 T120 outcome 中 3 个正向、5 个负向。
- **score / rank / RS120 与 T120 结果无单调映射**：策略评分、轮动排名、120 日相对强弱读数的高低，不能用来线性推断 T120 表现好坏。
- 三档窗口的正向计数（2/8、4/8、3/8）各不相同，且样本是单 episode 嵌套结构；方向性解读到此为止，不做统计推断。

---

## 7. 候选假设状态

试点产生了一些"什么可能解释失败"的候选假设（例如排序型读数的区分能力、crowding 等 episode 因子的解释力）。它们全部停留在**候选（candidate）且未验证**状态：

- 单 episode、非盲、无独立复核的证据，不足以确认或证伪任何假设。
- 候选假设只有在多独立 episode、冻结协议、独立复核都满足后，才允许进入评估。

---

## 8. 已定案语义问题（settled semantics）

这一节记录"字段语义是否可用于归因"的定案结论，是试点最有复用价值的部分。试点对这些差异**不强行对账**，而是把它们明确为已知语义，防止后续误用：

| # | 语义 | 定案结论 |
|---|------|----------|
| 1 | breadth 58.6% | Attribution MVP 读作 `Elevated`，Stretch 层读作 `Normal`。两套分档阈值并存，试点**保留差异、不做阈值对账**（preserved without threshold reconciliation）。 |
| 2 | leverage | 打印状态为 `Normal`，但证据显示保证金（margin）数据不可用。**字面语义修正**：打印 Normal 不等于数据可用；该字段对归因**不可用（unusable）**。 |
| 3 | crowding | 只是 **episode 级代理**，不能当作 symbol 级归因字段。 |
| 4 | Momentum Exhaustion | 只有命名，**没有操作性指标**，不能参与归因。 |
| 5 | Theme / Macro / Volatility | 缺少操作性的 **symbol 级归因字段**（只有 episode/scope 级或命名层）。 |
| 6 | `srd-strong` 条件 | research observe / analytics 的 `srd-strong` 条件**不等同于**逐标的 ledger 命中谓词（StrongBuy + scope DE_RISK），两者不能互换使用。 |
| 7 | 前向收益 | 是 **outcome（结果）**，不是因果证据；方向计数与相关关系不构成因果归因。 |
| 8 | LLM 叙事 | **不是证据**；只作解释层背景，不进入 ledger，不被归因引用。 |

---

## 9. 基础设施教训（可复现性前提）

- 精确日期研究依赖 **ClickHouse**：试点需要按精确日期读取确定性快照与后续行情，SQLite 侧不具备等价口径。
- 故障现象：Docker Desktop 处于停止状态时，ClickHouse 不可达，端口 18123 拒绝连接（refusal）。
- 恢复步骤：重新拉起 Docker daemon 与容器；**先验证** `http://127.0.0.1:18123/ping` 返回正常，再重跑研究命令。
- 定性：这是**可复现性前提**，不是产品缺陷。环境未就绪时不运行、不降级采样、不用缓存冒充。

---

## 10. 分类/标注推迟决定（annotation deferral）

本试点**决定推迟**所有分类/标注动作：

- 8 条 ledger 记录的分类全部推迟：保持未分类（`unclassified / null / null`），不引入成功/失败标签，不套用失败模式词汇。24 个 horizon outcome 全部 Filled 只表示前向数据齐备，与分类推迟相互独立。
- 理由：单 episode、回顾式、非盲的证据不足以支撑定义可复用类别；提前标注会把一次性样本的偶然模式固化成词汇表（可重复性陷阱）。
- 解除条件：见第 11 节的前瞻协议与验收门。只有协议冻结、多独立 episode、独立复核都满足后，分类工作才允许启动。

---

## 11. 前瞻：下一轮迭代协议、验收门与停止条件

下一轮迭代必须是**前瞻式（prospective）**的。协议要点与后续执行顺序（follow-up order）如下：

1. **继续前瞻采集**：在 outcome 尚不成熟（immature）阶段，继续对 GLOBAL / CN / HK 做前瞻性收集；不等待、不回填、不挑样本。
2. **揭盲前冻结协议**：在任何揭盲/判读（unblinding）之前，先冻结 observation / review 协议：口径、记录规则、谁在何时看什么，先写下来。
3. **精确日期确定性源字段与源哈希**：每一条记录都带精确日期的确定性源字段，并记录 source hash，保证可复现、可查证。
4. **独立复核证据与方法**：证据与方法由独立于执行方的人复核，之后再谈解读。
5. **多独立 episode 之后才考虑类别词汇**：只有积累多个独立 episode、且经独立复核后，才允许通过 ADR 程序讨论可复用的类别词汇（category vocabulary）。
6. **绝不用本试点调整阈值**：本试点不构成任何 State / Signal 阈值调整的依据；未来阈值变更必须来自独立于本试点的证据。

### 验收门（acceptance gates，协议级）

- 下一轮进入"判读/标注"之前必须同时满足：多独立 episode 已收集；observation / review 协议已冻结；源字段与 source hash 齐备；证据与方法已独立复核。
- 任何"可复用类别词汇 / 评分 / 置信度 / 阈值"的引入都属于新语义决策，必须走 ADR 程序；试点本身不冻结任何此类语义。

### 停止条件（stop conditions）

- outcome 不成熟时：只采集、不判读、不标注。
- 样本是单 episode（或嵌套样本被误当独立样本）时：不做任何推广解读。
- ClickHouse / 环境未就绪时：先停止并恢复环境、验证 `/ping` 后再续跑，不降级运行。
- 任何情况下：不从本试点调整 State / Signal 阈值。

---

## 12. 相关产物

| 产物 | 路径 | 角色 |
|------|------|------|
| 本参考文档 | `docs/task-100-failure-attribution-pilot.md` | 版本库内的持久记录（本文档） |
| 生成型试点报告 | `reports/attribution/attribution-cn-strongbuy-de-risk-2026-02-25.md` | 运行产物；`reports/` 为 gitignored，不进入版本库 |
| divergence ledger | `workspace/divergence-ledger/` | gitignored 运行目录；逐标的台账 |
| 任务 / ADR 权威状态 | `memory/context.md` + `memory/decisions.md` | TASK 状态与 ADR 条款以 MemGuard 记录为准 |
