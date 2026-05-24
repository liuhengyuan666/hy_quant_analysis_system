# 2026-05-20 Dashboard 性能优化

## 变更内容

仪表板加载超时（>120秒）问题的诊断与修复。

### 根因
`check_data_health()` 在仪表板热路径上发起 48 个外部 HTTP 请求（4 FRED + 22 Eastmoney + 22 Tencent），每次加载仪表板都触发。

### 修复
- `crates/report-engine/src/lib.rs`：TrustSummary DTO `data_health` 字段 `DataHealthSummary` → `Option<DataHealthSummary>`
- `crates/app-service/src/lib.rs`：从 `dashboard_snapshot_with_scope` / `dashboard_bundle_with_scope` 移除 `check_data_health()` 调用，`build_trust_summary` 接受 `Option`
- `apps/desktop/frontend/src/main.js`：前端降级渲染 "Data health not yet checked"

### 性能改善
| 指标 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| dashboard-snapshot | >120秒 | 27秒 | ~78% |
| export-report | >120秒 | 52秒 | ~57% |

### 遗留
- 新瓶颈：ClickHouse 日期查询（available_dates_ms: 24秒）
- 改造文档：`.omo/plans/dashboard-performance-optimization.md`
- 决策记录：`memory/decisions.md` [2026-05-20]
