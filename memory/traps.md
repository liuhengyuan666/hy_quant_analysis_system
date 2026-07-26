## Trap: 

### Context


### Solution


## Trap: 

### Context


### Solution


## Trap: 

### Context


### Solution


## Trap: 

### Context


### Root Cause
ClickHouse default max_partitions_per_insert_block=100 is too low for long-history symbols spanning multiple years

### Solution


### Prevention
All new ClickHouse INSERT queries must include partition limit settings when handling multi-year historical data

## Trap: 

### Context


### Root Cause
fetch_tencent_daily_bars had hardcoded count=400 in the API request, but Tencent actually supports up to 1000 rows per request

### Solution


### Prevention
All multi-year data fetchers must implement automatic pagination; never hardcode provider row limits

## Trap: CSS position:sticky sidebar jitter when scrolling to page bottom

### Context
App.vue dashboard-research sidebar, TopStatusBar.vue header

### Solution
Root cause: sticky element's margin-box overflows container's padding-box. Fix: (1) Pin header height exactly (e.g. height:3.5rem) so sticky top aligns without gap. (2) Set sticky element height to container-bottom minus 2px safety margin (calc(100vh - 3.5rem - 2rem - 2px)) to prevent push-out at scroll end. (3) Remove overflow-y:auto from sticky container itself — let internal child handle scroll.

## Trap: CSS position:sticky sidebar jitter when scrolling to page bottom

### Context
App.vue dashboard-research sidebar, TopStatusBar.vue header

### Solution
Root cause: sticky element's margin-box overflows container's padding-box. Fix: (1) Pin header height exactly (e.g. height:3.5rem) so sticky top aligns without gap. (2) Set sticky element height to container-bottom minus 2px safety margin (calc(100vh - 3.5rem - 2rem - 2px)) to prevent push-out at scroll end. (3) Remove overflow-y:auto from sticky container itself — let internal child handle scroll.

## Trap: MemGuard: TaskUpdated superseded_by lost after parallel commit with referenced ADR/Task

### Context
During memory repair for V8 Research Asset work, committed ADR-079/080/081 and new tasks TASK-111/114 in parallel with TaskUpdated events for TASK-106/107/108 that referenced ADR-079 and TASK-111. Subsequent lookup showed TASK-107 and TASK-108 as Superseded but with superseded_by: null, and re-committing the same TaskUpdated events returned 'Task has never been created'.

### Root Cause
Parallel commit of interdependent memory events likely caused the referenced ADR/Task to not exist at the time the TaskUpdated was processed, or the runtime failed to link the superseded_by reference. The error message is misleading because task_lookup still returns the tasks as archived.

### Solution
Sequence memory commits when there are dependencies: first commit new ADRs and new tasks, wait for success, then commit TaskUpdated events with superseded_by references. If a TaskUpdated fails to link, verify via task_lookup and, if inconsistent, record the discrepancy rather than retrying blindly.

### Prevention
Avoid parallel commits for memory events that reference each other. Always commit new entities first, then update existing entities that reference them. Verify with bootstrap + task_lookup after committing interdependent state.

## Trap: market-refresh 被 shell 超时强杀后，pipeline-dates 显示所有数据阶段 complete@最新日但 dashboard_available 回退到更早日期；llm-analyze 静默使用旧快照且 adversarial 每次调用重新生成（缓存永不命中）

### Context


### Solution
根因：被 kill 的刷新在 ingest 留下缺标的（30/31），daily_bar 不完整导致 latest gate 无法推进，dashboard_latest_date 回退。检测：pipeline-dates 对比 freshest_market_date vs dashboard_latest_date；解释：explain-latest-gate。修复：完整重跑 market-refresh（增量）即可恢复。预防：绝不用短 timeout 强杀 market-refresh——要么后台跑要么给足 20 分钟；kill 后必须跑 pipeline-dates 验证数据面完整。注意症状可能首先出现在看似无关的地方（LLM 层缓存失效、分析日期静默回退）。
