# 2026-04-25 trust 主入口收敛

## 变更内容

- 扩展 `TrustSummary` contract，加入 freshness/data-health 的结构化 digest 字段。
- 更新 `crates/app-service/src/lib.rs` 与 `crates/report-engine/src/lib.rs`，让 trust summary 在后端和报告层拥有更明确的证据摘要。
- 更新桌面前端 `apps/desktop/frontend/src/main.js`，将 trust summary 提升为主 panel，并把 freshness/data-health 作为其直属证据层摘要展示。
- 更新前端知识文档与 memory，记录 trust summary 已成为当前主可信度入口。
- 根据 Oracle follow-up，再补充历史 snapshot/export 场景下的 trust evidence disclaimer，明确当前 freshness/data-health 属于运营证据而不是历史运营状态回放。
- 再补充到 inline trust notice，避免历史 snapshot 在 refresh 成功提示路径下重新出现相同语义歧义。

## 原因

- 之前 trust summary 已存在，但仍更像一个附属提示；无法真正承接“主可信度入口 + 两个证据层”的产品方向。

## 影响

- 用户现在更容易先读一个主 trust verdict，再决定是否下钻 freshness / data-health 细节。
- 后续如果继续做 deeper refactor，可以直接围绕 trust contract 与 transport/API 统一推进，而不是再回头重构展示优先级。
