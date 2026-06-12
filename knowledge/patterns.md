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

## 5. 数据管线范式 (Data Pipeline)

- 管线顺序：ingest → indicators → macro → rotation → strategy → signals → backtest
- 各阶段日期检查通过 `pipeline-dates` 和 `explain-latest-gate`
- 数据健康检查：`check-data-health` 检查 provider 可达性、缺口、异常波动
- 默认 `export-report` 在 latest gate 落后时直接失败（fail-loud）

## 6. 配置管理范式 (Configuration)

- LLM 配置使用 TOML 文件 + 环境变量插值（`config/llm.toml`）
- API Key 三级回退：TOML → Keyring → SQLite
- Universe 配置使用 JSON（`config/universe.json`）
- 交易日历使用静态 JSON（`config/calendars/`）

## 7. 研究验证范式 (Research Validation)

- Ground Truth 与 Predictor 必须使用完全独立的数据路径
- Regime 评估使用三层独立框架：State Layer（描述性）、Economic Layer（预测性）、Allocation Layer（决策性）
- 所有实验结论需在 `confirmation_days=1` 下重新验证
- Wave 研究阶段化：Wave 7（GT验证）、Wave 8（Insight Composer）、Wave 9（Daily Report）
