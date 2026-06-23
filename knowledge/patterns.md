# Project Coding Idioms & Patterns (编码范式与设计方言)

> 认知护栏：记录本项目中被团队高度达成共识的“代码写法习惯”，新进 Agent 必须严格继承此编码风格。

## 1. 错误处理范式 (Error Handling)

- 使用 `anyhow` 进行错误传播，顶层函数返回 `Result<T>`
- 自定义错误使用 `thiserror` 派生
- 禁止裸 `unwrap()`，必须使用 `?` 或显式 `match`
- 数据获取失败进入 `failed_items`，不再静默产出空结果

## 2. 异步与并发范式 (Concurrency/Async)

- 使用 `tokio` 作为异步运行时
- 宏引擎和指标计算使用并行迭代（`par_iter`）
- Tauri 命令使用异步函数
- 数据刷新管线按顺序执行，不能倒序或跳过

## 3. 状态管理与数据转换 (Data Mapping)

- 核心领域模型定义在 `core-domain` crate
- 所有 DTO 使用 `serde` 序列化/反序列化
- ClickHouse JSON 反序列化字段必须携带 `#[serde(default)]`（Schema Evolution 政策）
- 状态转换使用显式状态机（Task Lifecycle、ADR Lifecycle）

## 4. 前端架构范式 (Frontend)

- Vue 3 + Composition API，使用 `reactive()` 共享状态
- 组件从 `store.js` 读取状态，通过 `event bridge` 回调 main.js
- CSS 变量桥接：全局 CSS 定义设计 token，Vue 组件消费 CSS 变量
- i18n 使用 `vue-i18n@11`，默认中文，嵌套 JSON key 结构
- **HTML/CSS 混合布局**：复杂数据可视化（如 RotationPanel）优先使用 HTML/CSS 原生布局替代纯 ECharts，提升信息密度、响应式对齐和无障碍访问；ECharts 仅用于纯图表场景
- **CSS 自定义 Tooltip**：禁用原生 `title` 和 ECharts 默认 Tooltip，使用绝对定位的 CSS 自定义 Tooltip（`position: absolute` + `visibility/opacity` 过渡），确保暗黑主题样式一致、无割裂感
- **Sticky 布局约束**：`position: sticky` 的侧边栏必须满足 (1) 顶部参考元素（如 header）固定高度，消除 `top` 旷量；(2) sticky 元素高度精确计算，使其 margin-box 不溢出 container 的 padding-box（通常 `height: calc(100vh - headerHeight - padding * 2 - 2px)` 留 2px 安全边距），避免滚动到底部时 push-out 抖动

## 5. 数据管线范式 (Data Pipeline)

- 管线顺序：ingest → indicators → macro → rotation → strategy → signals → backtest
- 各阶段日期检查通过 `pipeline-dates` 和 `explain-latest-gate`
- 数据健康检查：`check-data-health` 检查 provider 可达性、缺口、异常波动
- 默认 `export-report` 在 latest gate 落后时直接失败（fail-loud）

## 6. 配置管理范式 (Configuration)

- **TOML 配置模式**：LLM 和 FRED 配置均使用 TOML 文件 + 环境变量插值（`config/llm.toml`、`config/fred.toml`）
- **API Key 安全存储**：TOML 文件 gitignored，支持 `${ENV_VAR}` 引用，避免硬编码
- **可配置开关**：FRED 获取支持 `enabled` 开关，禁用后使用 ClickHouse 已存数据（fail-safe）
- **配置加载优先级**：CLI 参数 > TOML 文件（含环境变量插值） > 默认值
- **LLM API Key 三级回退**：TOML → Keyring → SQLite
- **FRED API Key 单级回退**：TOML 文件（低敏感度，免费政府数据 API）
- Universe 配置使用 JSON（`config/universe.json`）
- 交易日历使用静态 JSON（`config/calendars/`）

## 7. 研究验证范式 (Research Validation)

- Ground Truth 与 Predictor 必须使用完全独立的数据路径
- Regime 评估使用三层独立框架：State Layer（描述性）、Economic Layer（预测性）、Allocation Layer（决策性）
- 所有实验结论需在 `confirmation_days=1` 下重新验证
- Wave 研究阶段化：Wave 7（GT验证）、Wave 8（Insight Composer）、Wave 9（Daily Report）

## 9. 外部数据源集成范式 (External Data Provider)

- **编码检测**：HTTP 响应必须验证编码格式，不要假设所有 API 返回 UTF-8；Tencent API 返回 GB18030，需使用 `encoding_rs` 解码
- **Symbol 映射**：每个 provider 有独立的 symbol 前缀规则；上海指数 `000xxx` 必须用 `sh` 前缀，深圳指数 `399xxx` 用 `sz` 前缀
- **字段索引验证**：解析分隔符格式时，必须确认字段索引对应关系；实际验证前不要假设字段顺序（如 `parts[1]` vs `parts[2]`）
- **降级策略**：实时数据获取失败时，必须降级为 `Skip` 状态，不能 panic 或影响主系统运行

- **Shadow Production 冻结**：State Layer、Economic Layer、Signal Engine、weights、thresholds、allocation、backtest 语义全部冻结
- **Production Surface 冻结**：DashboardSnapshot、ResearchContext、Markdown Report 在观察期间不修改展示逻辑
- **Research Surface 允许新增**：观测/诊断工具（symbol-diagnostics, symbol-scoreboard, rotation-ranking）作为独立 CLI 工具，不进入主观察链路
- **Explainability Layer 约束**：可以解释现有决策，但不得生成新评分、排名或决策信号
- **ADR 优先级**：User instruction > Active ADRs > Traps > Search results > Model knowledge
