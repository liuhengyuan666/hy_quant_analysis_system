# 2026-04-25 功能设计复盘确认项

## 变更内容

- 基于功能设计 review，确认了当前系统后续产品收敛方向。
- 将 5 项关键结论写入 `memory/decisions.md` 与 `memory/context.md`。

## 已确认事项

1. `scope-aware` 环境解释与 `GLOBAL` signal/backtest 语义裂缝是当前第一优先级问题。
2. 可信度设计采用“一个主可信度入口 + 两个证据层”的方向。
3. `Environment layer` 与 `Watchlist Breadth` 暂时保留双面板。
4. `Recent reports` 应升级为研究结果管理入口。
5. `desktop refresh` 为默认用户路径。

## 原因

- 当前系统最大的问题已经不是单点功能缺失，而是用户心智被多个功能面和历史语义残留所分裂。

## 影响

- 后续实现顺序应优先围绕语义一致性、可信度入口、研究结果管理三条主线推进。
