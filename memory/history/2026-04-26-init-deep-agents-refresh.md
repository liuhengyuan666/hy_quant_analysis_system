# 2026-04-26 `/init-deep` knowledge-base refresh

## 本次范围

- 刷新：
  - `AGENTS.md`
  - `crates/AGENTS.md`
  - `apps/desktop/AGENTS.md`
  - `apps/desktop/frontend/AGENTS.md`
- 新增：
  - `apps/desktop/src-tauri/AGENTS.md`
  - `crates/core-domain/AGENTS.md`
  - `crates/macro-engine/AGENTS.md`

## 这次写回的关键边界

- desktop `Refresh data` 是默认用户路径；stage rerun 只是恢复分支。
- `Trust summary` 是主可信度入口；`Pipeline freshness` / `Data health` 是证据层。
- `Recent reports` 已经是研究结果入口，不再只是路径列表。
- `src-tauri` 负责 command boundary、refresh coordinator、safe artifact opening，不负责量化逻辑。
- `core-domain` 是共享 contract 边界，DTO / enum 漂移会同时影响 store / report / desktop。
- `macro-engine` 是纯计算边界，fetch / persistence / fallback-to-history 仍归上游 `app-service`。

## 对后续工作的意义

- 继续做 `compare previous` 时，最近的知识入口应先看 `apps/desktop/frontend/AGENTS.md` 与 `apps/desktop/src-tauri/AGENTS.md`。
- 继续动 trust / refresh / signal freshness 语义时，先看 root `AGENTS.md` 和 `crates/AGENTS.md`，避免重新沿旧的 `main.js` 单文件心智行动。
