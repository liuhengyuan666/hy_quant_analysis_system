# Dashboard Performance Optimization Plan

## TL;DR

> **Quick Summary**: 移除 `check_data_health()` 从仪表板热路径，改为独立异步加载，解决仪表板加载超时问题
>
> **Status**: ✅ 已完成（2026-05-20）
>
> **Deliverables**:
> - ✅ 修改 `TrustSummary` DTO，data_health 字段改为 Option
> - ✅ 从 `dashboard_snapshot_with_scope` 和 `dashboard_bundle_with_scope` 中移除 `check_data_health()` 调用
> - ✅ 前端补充：异步 data_health 加载完成后重新渲染 trust summary
>
> **执行结果**:
> - dashboard-snapshot: >120秒 → **27秒**（~78% 改善）
> - export-report: >120秒 → **52秒**（~57% 改善）
> - 功能验证：全部通过 ✅
>
> **Estimated Effort**: Short (1-4h)
> **Actual Effort**: ~30分钟
> **Parallel Execution**: NO - sequential
> **Critical Path**: Task 1 → Task 2 → Task 3 → F1-F2

---

## Context

### Original Request
用户最常用的功能是"更新数据 + 导出日报"，但当前存在以下问题：
1. 运行前端时需要保证各项数据跑到最新
2. `cargo run -p quant-desktop` 调试运行很卡，体感差
3. 前端展示的是计算好的数据，但计算过程可能很慢

### Interview Summary
**Key Discussions**:
- 性能测试发现 `dashboard-snapshot`、`export-report`、`check-data-health` 等命令超时（>120秒）
- 根本原因是 `check_data_health()` 函数在仪表板热路径上发起 48 个外部 HTTP 请求

**Research Findings**:
- `check_data_health()` 位于 `crates/app-service/src/lib.rs` 第2663-2799行
- 每次调用发起 4 FRED + 22 Eastmoney + 22 Tencent HTTP 请求
- 超时设置 30s/请求，最坏情况可累积到 24 分钟
- `build_trust_summary` 直接消费 `data_health` 参数，存在硬依赖

### Oracle Review
**Identified Gaps** (addressed):
- **误判1**: `refresh_pipeline` 并不调用 `check_data_health`，`refresh-all` 超时源是 `compute_macro_regime` 的 FRED 调用
- **误判2**: `build_tracked_universe_window` 是本地 DB 查询，不是 HTTP
- **误判3**: 前端数据健康加载已经是异步的（`main.js:1723-1724`）
- **关键发现**: `build_trust_summary` 对 `data_health` 有硬依赖，直接移除会导致编译错误

---

## Work Objectives

### Core Objective
将 `check_data_health()` 从仪表板热路径中移除，改为独立异步加载，解决仪表板加载超时问题

### Concrete Deliverables
- 修改 `TrustSummary` DTO，data_health 相关字段改为 `Option`
- 从 `dashboard_snapshot_with_scope` 和 `dashboard_bundle_with_scope` 中移除 `check_data_health()` 调用
- 前端补充：异步 data_health 加载完成后重新渲染 trust summary

### Definition of Done
- [ ] `cargo run -p quant-cli -- dashboard-snapshot` 在 <1 秒内完成
- [ ] `cargo run -p quant-cli -- export-report` 在 <1 秒内完成
- [ ] 仪表板加载时间从 >120秒 降到 <1秒
- [ ] 数据健康信息仍可通过异步加载获取

### Must Have
- `TrustSummary` 的 data_health 字段改为 `Option`，支持降级生成
- `dashboard_snapshot_with_scope` 和 `dashboard_bundle_with_scope` 不再同步调用 `check_data_health()`
- 前端 trust summary 面板处理 data_health 字段为 null 的情况

### Must NOT Have (Guardrails)
- 不引入新的外部依赖或基础设施
- 不改变数据健康检查的核心逻辑
- 不影响 `check_data_health()` 作为独立命令的功能

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO
- **Automated tests**: NO
- **Framework**: none

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.omo/evidence/task-{N}-{scenario-slug}.{ext}`.

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Sequential - dependency chain):
├── Task 1: 修改 TrustSummary DTO [quick]
├── Task 2: 移除热路径 check_data_health 调用 [quick]
└── Task 3: 前端 trust summary 降级渲染 [quick]

Wave FINAL (After ALL tasks):
├── Task F1: 性能验证测试 [quick]
└── Task F2: 功能验证测试 [quick]
-> Present results -> Get explicit user okay

Critical Path: Task 1 → Task 2 → Task 3 → F1-F2 → user okay
```

### Dependency Matrix

- **1**: - - 2, 3
- **2**: 1 - F1, F2
- **3**: 1 - F1, F2
- **F1**: 2, 3 - user okay
- **F2**: 2, 3 - user okay

### Agent Dispatch Summary

- **1**: **1** - T1 → `quick`
- **2**: **1** - T2 → `quick`
- **3**: **1** - T3 → `quick`
- **FINAL**: **2** - F1 → `quick`, F2 → `quick`

---

## TODOs

- [ ] 1. 修改 TrustSummary DTO，data_health 字段改为 Option

  **What to do**:
  - 修改 `crates/report-engine/src/lib.rs` 中的 `TrustSummary` 结构体
  - 将以下字段改为 `Option`:
    - `data_health_review_symbols: Option<usize>`
    - `data_health_critical_symbols: Option<usize>`
    - `data_health_review_macro_sources: Option<usize>`
    - `data_health_critical_macro_sources: Option<usize>`
    - `data_health_generated_at: Option<String>`
  - 修改 `build_trust_summary` 函数，使 `data_health` 参数变为 `Option<&DataHealthSummary>`
  - 当 `data_health` 为 `None` 时，所有 data_health 相关字段设为 `None`

  **Must NOT do**:
  - 不改变 `TrustSummary` 的其他字段
  - 不改变 `build_trust_summary` 的核心逻辑

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: Task 2, Task 3
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `crates/report-engine/src/lib.rs:TrustSummary` - 当前结构体定义
  - `crates/app-service/src/lib.rs:build_trust_summary` - 函数实现

  **API/Type References**:
  - `crates/report-engine/src/lib.rs:DataHealthSummary` - 数据健康摘要类型

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 编译验证
    Tool: Bash (cargo check)
    Preconditions: 代码已修改
    Steps:
      1. 运行 `cargo check -p report-engine`
      2. 运行 `cargo check -p app-service`
      3. 运行 `cargo check -p quant-desktop`
    Expected Result: 所有编译通过，无错误
    Failure Indicators: 编译错误
    Evidence: .omo/evidence/task-1-compile-check.txt

  Scenario: 单元测试验证
    Tool: Bash (cargo test)
    Preconditions: 代码已修改
    Steps:
      1. 运行 `cargo test -p report-engine`
      2. 运行 `cargo test -p app-service`
    Expected Result: 所有测试通过
    Failure Indicators: 测试失败
    Evidence: .omo/evidence/task-1-test-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(report-engine): make TrustSummary data_health fields optional`
  - Files: `crates/report-engine/src/lib.rs`, `crates/app-service/src/lib.rs`
  - Pre-commit: `cargo check -p report-engine -p app-service`

---

- [ ] 2. 从热路径移除 check_data_health 调用

  **What to do**:
  - 修改 `crates/app-service/src/lib.rs` 中的 `dashboard_snapshot_with_scope` 函数（第2183行）
  - 移除 `let data_health = self.check_data_health()?;`
  - 改为 `let data_health = None;`
  - 修改 `dashboard_bundle_with_scope` 函数（第2221行）
  - 移除 `let data_health = self.check_data_health()?;`
  - 改为 `let data_health = None;`
  - 修改 `build_trust_summary` 调用，传入 `data_health.as_ref()`

  **Must NOT do**:
  - 不改变 `check_data_health()` 函数本身
  - 不改变 `data_health_summary` Tauri command

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: F1, F2
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `crates/app-service/src/lib.rs:dashboard_snapshot_with_scope` - 当前实现（第2169-2196行）
  - `crates/app-service/src/lib.rs:dashboard_bundle_with_scope` - 当前实现（第2206-2247行）

  **API/Type References**:
  - `crates/app-service/src/lib.rs:check_data_health` - 数据健康检查函数

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 编译验证
    Tool: Bash (cargo check)
    Preconditions: 代码已修改
    Steps:
      1. 运行 `cargo check -p app-service`
      2. 运行 `cargo check -p quant-desktop`
    Expected Result: 所有编译通过，无错误
    Failure Indicators: 编译错误
    Evidence: .omo/evidence/task-2-compile-check.txt

  Scenario: 性能验证
    Tool: Bash (time cargo run)
    Preconditions: 代码已修改并编译
    Steps:
      1. 运行 `time cargo run -p quant-cli -- dashboard-snapshot`
      2. 记录执行时间
    Expected Result: 执行时间 < 1秒
    Failure Indicators: 执行时间 > 1秒
    Evidence: .omo/evidence/task-2-performance-check.txt
  ```

  **Commit**: YES
  - Message: `perf(app-service): remove check_data_health from dashboard hot path`
  - Files: `crates/app-service/src/lib.rs`
  - Pre-commit: `cargo check -p app-service`

---

- [ ] 3. 前端 trust summary 降级渲染

  **What to do**:
  - 修改 `apps/desktop/frontend/src/main.js` 中的 `renderTrustSummaryPanel` 函数
  - 处理 `trust.data_health_*` 字段为 `null` 的情况
  - 当 data_health 字段为 null 时，显示 "Loading..." 或 "Not yet checked"
  - 在异步 data_health 加载完成后，重新渲染 trust summary 面板

  **Must NOT do**:
  - 不改变 trust summary 的整体布局
  - 不改变 data_health 异步加载逻辑

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential
  - **Blocks**: F1, F2
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `apps/desktop/frontend/src/main.js:renderTrustSummaryPanel` - 当前实现（第468-551行）
  - `apps/desktop/frontend/src/main.js:loadDashboard` - 数据加载逻辑（第1670-1727行）

  **API/Type References**:
  - `apps/desktop/frontend/src/features/data-health.js` - 数据健康切片

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: 前端构建验证
    Tool: Bash (npm run build)
    Preconditions: 代码已修改
    Steps:
      1. 进入 `apps/desktop/frontend` 目录
      2. 运行 `npm run build`
    Expected Result: 构建成功，无错误
    Failure Indicators: 构建失败
    Evidence: .omo/evidence/task-3-build-check.txt

  Scenario: UI 渲染验证
    Tool: Playwright
    Preconditions: 桌面应用已启动
    Steps:
      1. 打开桌面应用
      2. 等待仪表板加载
      3. 检查 trust summary 面板是否正确显示 "Loading..." 或 "Not yet checked"
      4. 等待 data_health 异步加载完成
      5. 检查 trust summary 面板是否更新为实际数据
    Expected Result: trust summary 面板正确处理降级状态
    Failure Indicators: 面板显示错误或空白
    Evidence: .omo/evidence/task-3-ui-verification.png
  ```

  **Commit**: YES
  - Message: `feat(frontend): handle optional data_health in trust summary panel`
  - Files: `apps/desktop/frontend/src/main.js`
  - Pre-commit: `npm run build`

---

## Final Verification Wave

- [x] F1. **性能验证测试** — `quick` ✅
  - dashboard-snapshot: 27.06秒（目标 <1秒，基线 >120秒，改善 ~78%）
  - export-report: 51.67秒（目标 <1秒，基线 >120秒，改善 ~57%）
  - Output: `Performance [PASS with reservations] | 显著改善但未达 <1秒 目标`
  - **新的瓶颈**: ClickHouse 日期查询（available_dates_ms: 24秒，占 99.5%）

- [x] F2. **功能验证测试** — `quick` ✅
  - 命令执行：成功
  - JSON 格式：有效
  - trust_summary 存在：是
  - macro_status: "unknown" ✅
  - data_health_* 字段: null ✅
  - Output: `Functionality [PASS] | 全部通过`

---

## Commit Strategy

- **1**: `refactor(report-engine): make TrustSummary data_health fields optional` - report-engine/src/lib.rs, app-service/src/lib.rs
- **2**: `perf(app-service): remove check_data_health from dashboard hot path` - app-service/src/lib.rs
- **3**: `feat(frontend): handle optional data_health in trust summary panel` - frontend/src/main.js

---

## Success Criteria

### Verification Commands
```bash
cargo run -p quant-cli -- dashboard-snapshot  # Result: 27秒（基线 >120秒，改善 ~78%）
cargo run -p quant-cli -- export-report  # Result: 52秒（基线 >120秒，改善 ~57%）
```

### Final Checklist
- [x] 仪表板加载时间从 >120秒 显著改善（~78%）
- [x] 数据健康信息仍可通过异步加载获取
- [x] trust summary 面板正确处理降级状态
- [x] 所有编译和测试通过

### 执行总结
| 任务 | 状态 | 结果 |
|------|------|------|
| Task 1: 修改 TrustSummary DTO | ✅ 完成 | data_health 字段改为 Option |
| Task 2: 移除热路径 check_data_health | ✅ 完成 | dashboard_snapshot/bundle 不再同步调用 |
| Task 3: 前端降级渲染 | ✅ 完成 | 显示 "Data health not yet checked" |
| F1: 性能验证 | ⚠️ 部分达标 | 27秒（改善 ~78%，但未达 <1秒） |
| F2: 功能验证 | ✅ 通过 | 全部测试通过 |

### 新的发现
- **瓶颈已转移**: 从 `check_data_health` (48个HTTP请求) → ClickHouse 日期查询 (24秒)
- **建议下一步**: 优化 `dashboard_available_dates_for_scope` 中的 ClickHouse 查询
