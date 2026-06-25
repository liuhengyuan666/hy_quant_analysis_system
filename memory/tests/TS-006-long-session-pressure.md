# TS-006: Long Session Pressure Test

## 目标
验证 40-60 轮对话后，MemGuard v4.5 的 Capability Boundaries、Operation Routing 和
Checkpoint 机制是否仍然有效。

## 环境
- Opencode + OMO
- MemGuard v4.5
- 真实工作任务（非模拟场景）

## 测试步骤

### Phase 1: 初始状态 (1-5 轮)
1. 启动新 Session
2. 执行 `bootstrap()`
3. 确认 Agent 读取所有 active tasks
4. 执行 2-3 个简单任务更新
5. 验证 Agent 使用 `commit_event()` 而非直接操作文件

### Phase 2: 正常工作 (6-20 轮)
1. 进行正常的任务执行、ADR 创建、查询
2. 每 5 轮检查一次是否触发 Checkpoint
3. 记录 Agent 是否使用 Operation Routing 中的正确工具
4. 检查 Agent 是否尝试不存在的 MCP 能力（如修改 Constraint）

### Phase 3: 第一次压缩 (21-25 轮)
1. 完成一个阶段性任务（触发 OMO 压缩）
2. 观察 Agent 是否：
   - **重新执行 `bootstrap()`**
   - **不假设之前的状态仍然有效**
3. **Compression Recovery Validation**: 检查 Agent re-bootstrap 的原因
   - 如果日志出现 "我记得..." / "I think..." → 判定失败
   - 如果日志明确说明 "State lost, rerunning bootstrap..." → 判定通过

### Phase 4: 继续工作 (26-35 轮)
1. 继续执行任务
2. 检查 Agent 是否：
   - 仍然遵守 Cheat Sheet（不盲猜字段）
   - 仍然使用 Capability Boundaries 的默认策略
   - 不尝试 `skill("memguard")`
   - 不推理不存在的 MCP 能力（如 ConstraintUpdated）
3. 验证 Operation Routing：Agent 是否优先使用 `task_lookup()` 而非上下文记忆

### Phase 5: 第二次压缩 (36-40 轮)
1. 再次完成阶段性任务（触发第二次压缩）
2. 检查 Agent 是否：
   - 仍然重新 bootstrap
   - 不进入 "幻觉" 状态（如声称知道任务已完成，但未查询）
3. 记录任何 "unsupported" 请求的处理：Agent 是否直接告知不支持，而非推理尝试

### Phase 6: 最终验证 (41-60 轮)
1. 执行最终任务
2. 检查 Agent 是否：
   - 正确 commit 所有状态变更
   - 不遗漏任何任务
   - 不重复创建已存在的任务
   - 不尝试修改 Constraint
3. 验证 Capability Boundaries 是否被主动引用

## 通过标准 (必须全部满足)

- [ ] 60 轮内不出现 `skill("memguard")` 调用
- [ ] 每次压缩后都重新 `bootstrap()`（且原因正确）
- [ ] 不尝试修改 constraints（直接返回 unsupported）
- [ ] 不推理不存在的 MCP 能力（如 ConstraintUpdated event）
- [ ] 所有任务状态查询来自 `task_lookup()` 而非上下文记忆
- [ ] 所有项目状态查询来自 `bootstrap()` 或 `query_memory()` 而非上下文记忆
- [ ] 所有任务状态正确 commit
- [ ] 不遗漏 Checkpoint
- [ ] 遇到 unsupported 请求时，直接引用 5.4 规则而非推理

## 失败判定

如果任何一项未通过：
1. 记录失败轮次和具体行为
2. 分析是 Skill 问题还是 Agent 认知问题
3. 如果是 Skill 问题 → 增强 Capability Boundaries 或 Operation Routing
4. 如果是 Agent 认知问题 → 检查是否 Token 限制导致章节被截断

## 核心指标

| 指标 | 目标 | 测量方法 |
|------|------|---------|
| Bootstrap 正确率 | 100% | 每次压缩后都重新执行 |
| 状态查询 MCP 化率 | 100% | 所有状态查询来自工具而非记忆 |
| Unsupported 请求正确处理率 | 100% | 不推理，直接引用规则 |
| Cheat Sheet 遵守率 | 100% | 不盲猜字段 |
| 幻觉任务状态率 | 0% | 不声称知道未查询的任务状态 |

## 记录模板

每次测试 Session 记录：

```markdown
## Session 记录

- 日期: YYYY-MM-DD
- 总轮数: N
- 压缩次数: N
- 通过的检查项: [列表]
- 失败的检查项: [列表]
- 发现的异常行为: [描述]
- 建议的 Skill 改进: [描述]
```

## 预期收益

通过 TS-006 后，可以确认：
1. Capability Boundaries 在长 Session 中不会被遗忘
2. Operation Routing 能减少 Agent 的推理浪费
3. Agent 在压缩后能正确重建状态
4. 当前 Skill 设计已足够成熟，不需要继续增加规则

如果 TS-006 失败，需要分析：
- 是 Agent 认知问题 → 增强 Skill 文档
- 是 Token 限制问题 → 考虑章节压缩或拆分
- 是 OMO 压缩策略问题 → 反馈给 Opencode 团队
