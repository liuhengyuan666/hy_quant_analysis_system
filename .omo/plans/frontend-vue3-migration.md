# Frontend Vue 3 Migration Plan

## TL;DR

> **Summary**: 将桌面前端从 Plain JS 迁移到 Vue 3 + Composition API，同时实施 breadth-ma30 面板和布局优化
>
> **Status**: 📋 Planned
>
> **Decision**: ADR-034（前端改进方案：Vue 3 迁移 + 布局优化）

---

## Context

### Current State

| 指标 | 值 |
|------|-----|
| 技术栈 | Plain JS + Vite，零框架 |
| main.js | 1818 行，16+ 渲染函数，25+ 状态字段 |
| styles.css | 1533 行，单文件暗色主题 |
| 渲染模式 | `innerHTML` 全量替换 + 末尾重绑事件 |
| 已拆分模块 | 4 个 feature slice |
| 类型安全 | 无 TypeScript |

### Problems

1. **布局杂乱拥挤**：面板堆叠未最大化利用空间
2. **组件化程度低**：无独立组件，innerHTML 全量替换
3. **状态管理分散**：单一全局 state 对象，slice 直接 mutation
4. **无类型安全**：全靠运行时

### Oracle Review Findings

- Phase 1-2 分离浪费（Plain JS 构建后重写）
- CSS 迁移是最大未识别风险
- 时间估算需调整为 4-6 周（侧项目节奏）
- 建议 breadth-ma30 作为 Vue 试点组件

---

## Decision

1. **框架选择**: Vue 3 + Composition API
2. **迁移策略**: breadth-ma30 作为第一个 Vue 试点组件
3. **UI/UX 方向**: 保持暗色主题，优化布局，简约风格，最大化空间利用
4. **CSS 策略**: 保留全局 CSS 用于布局/主题 token，Vue 组件仅消费 CSS 变量
5. **状态迁移**: `reactive()` 保持当前 state 对象结构

---

## Execution Plan

### Phase 1: Vue 基础设施 + breadth-ma30 试点（1 周）

**目标**: 验证 Vue + Tauri 集成，交付 breadth-ma30 面板

#### Task 1.1: Vue 项目脚手架（1 天）
- [ ] 添加 Vue 3 依赖（`vue`、`@vitejs/plugin-vue`）
- [ ] 创建 `vite.config.js` 配置 Vue 插件
- [ ] 创建 `App.vue` 根组件
- [ ] 修改 `index.html` 入口
- [ ] 验证 `npm run dev` 正常启动

#### Task 1.2: API 层提取（1 天）
- [ ] 创建 `src/api/tauri.js` 封装 invoke 调用
- [ ] 创建 `src/api/dashboard.js` 封装数据加载函数
- [ ] 验证 API 层在 Plain JS 和 Vue 中都能正常工作

#### Task 1.3: breadth-ma30 Vue 组件（3 天）
- [ ] 创建 `src/components/BreadthPanel.vue`
- [ ] 实现 `renderWatchlistBreadthPanel` 逻辑
- [ ] 添加 CSS 样式（消费全局 CSS 变量）
- [ ] 在 App.vue 中挂载，替代 Plain JS 版本
- [ ] 验证日期切换时数据更新
- [ ] 验证缺失数据降级显示

**交付物**:
- Vue 3 + Tauri 集成验证通过
- breadth-ma30 面板上线（Vue 版本）

---

### Phase 2: 逐面板迁移（3-4 周）

**目标**: 将所有 Plain JS 面板迁移到 Vue 组件

#### 迁移顺序（从低风险到高风险）

| 优先级 | 面板 | 复杂度 | 预计时间 |
|--------|------|--------|----------|
| 1 | MetricCard（通用组件） | 低 | 0.5 天 |
| 2 | DateSelector | 低 | 0.5 天 |
| 3 | HealthStrip | 低 | 0.5 天 |
| 4 | TimeContext | 低 | 0.5 天 |
| 5 | StatusPanel | 中 | 1 天 |
| 6 | RegimePanel | 中 | 1 天 |
| 7 | RotationPanel | 中 | 1 天 |
| 8 | BacktestPanel | 低 | 0.5 天 |
| 9 | EnvironmentPanel | 中 | 1 天（已拆分为 renderer） |
| 10 | SignalsPanel | 高 | 1.5 天 |
| 11 | SignalDetailModal | 高 | 1 天 |
| 12 | TrustSummaryPanel | 高 | 1.5 天 |
| 13 | RefreshProgress | 高 | 1.5 天 |
| 14 | Notice | 低 | 0.5 天 |

**迁移模式**:
```vue
<script setup>
import { computed } from 'vue'
import { useDashboardStore } from '@/stores/dashboard'

const store = useDashboardStore()

const panelData = computed(() => store.snapshot?.regime)
</script>

<template>
  <article class="panel panel--regime">
    <div class="panel__header">
      <span class="eyebrow">Market Regime</span>
      <h2>{{ panelData?.label }}</h2>
    </div>
    <!-- ... -->
  </article>
</template>

<style scoped>
/* 仅组件特有样式，复用全局 CSS 变量 */
</style>
```

**状态迁移**:
```javascript
// src/stores/dashboard.js
import { reactive } from 'vue'

const state = reactive({
  loading: false,
  refreshing: false,
  error: null,
  status: null,
  snapshot: null,
  availableDates: [],
  selectedScope: 'global',
  selectedReportDate: null,
  // ... 保持当前结构
})

export function useDashboardStore() {
  return state
}
```

**交付物**:
- 所有面板迁移为 Vue 组件
- main.js 从 1818 行缩减到 ~200 行（仅初始化逻辑）

---

### Phase 3: 布局优化 + UI 细节（1-2 周）

**目标**: 简约风格，最大化空间利用

#### 设计原则

1. **简约风格**：
   - 减少视觉噪音（过多的边框、阴影、渐变）
   - 统一间距和对齐
   - 清晰的信息层级

2. **空间利用**：
   - 响应式网格布局（CSS Grid）
   - 面板大小自适应内容
   - 关键信息优先展示

3. **布局方案**：
   ```
   ┌─────────────────────────────────────────────────┐
   │  Trust Summary (全宽)                            │
   ├─────────────────────────────────────────────────┤
   │  Health Strip + Date Selector (全宽)             │
   ├─────────────────────────────────────────────────┤
   │  ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │
   │  │  Regime     │ │  Breadth    │ │  Time      │ │
   │  │             │ │             │ │  Context   │ │
   │  └─────────────┘ └─────────────┘ └────────────┘ │
   ├─────────────────────────────────────────────────┤
   │  ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │
   │  │  Rotation   │ │  Signals    │ │  Backtest  │ │
   │  │             │ │             │ │            │ │
   │  └─────────────┘ └─────────────┘ └────────────┘ │
   ├─────────────────────────────────────────────────┤
   │  ┌─────────────┐ ┌─────────────┐                │
   │  │  Environment│ │  Recent     │                │
   │  │             │ │  Reports    │                │
   │  └─────────────┘ └─────────────┘                │
   └─────────────────────────────────────────────────┘
   ```

#### Task 3.1: 响应式网格布局（3 天）
- [ ] 设计 CSS Grid 布局系统
- [ ] 实现面板大小自适应
- [ ] 优化断点（1080px、720px）

#### Task 3.2: 视觉细节优化（3 天）
- [ ] 统一间距系统（4px 基准）
- [ ] 简化边框和阴影
- [ ] 优化排版层级

#### Task 3.3: 交互优化（2 天）
- [ ] 信号详情弹窗 → 侧边面板
- [ ] 添加过渡动画（面板切换、数据加载）

**交付物**:
- 简约风格上线
- 响应式布局优化
- 空间利用率提升

---

## CSS Migration Strategy

### 当前 CSS 结构

```
styles.css (1533 行)
├── :root CSS 变量 (60+ 个)
├── 全局重置
├── 布局系统
├── 面板样式
├── 组件样式
└── 响应式断点
```

### 迁移策略

1. **保留全局 CSS**：
   - `:root` 变量（颜色、间距、字体）
   - 布局系统（`.dashboard-grid`）
   - 基础组件样式（`.panel`、`.metric-card`）

2. **Vue 组件样式**：
   - 使用 `<style scoped>` 避免冲突
   - 消费全局 CSS 变量
   - 仅定义组件特有样式

3. **迁移步骤**：
   - Phase 1：保持全局 CSS 不变，Vue 组件消费变量
   - Phase 2：逐步提取组件特有样式到 `<style scoped>`
   - Phase 3：清理未使用的全局样式

---

## Risk Mitigation

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Vue + Tauri 集成问题 | 构建失败 | Phase 1 验证，失败则回退 Plain JS |
| CSS 特异性冲突 | 样式错乱 | 使用 CSS 变量 + scoped 样式 |
| 性能回退 | 用户体验下降 | 监控渲染性能，必要时优化 |
| 迁移期间功能中断 | 用户无法使用 | 渐进式迁移，新旧并存 |

---

## Success Criteria

- [ ] breadth-ma30 面板上线（Vue 版本）
- [ ] 所有面板迁移为 Vue 组件
- [ ] 布局优化上线，空间利用率提升
- [ ] 无功能回退
- [ ] 无性能回退（渲染时间 <100ms）

---

## Timeline

| Phase | 时间 | 交付物 |
|-------|------|--------|
| Phase 1 | 1 周 | Vue 基础设施 + breadth-ma30 试点 |
| Phase 2 | 3-4 周 | 逐面板迁移 |
| Phase 3 | 1-2 周 | 布局优化 + UI 细节 |
| **总计** | **5-7 周** | Vue 3 组件化架构 + 简约布局 |
