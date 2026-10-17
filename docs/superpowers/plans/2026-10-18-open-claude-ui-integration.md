# AI-Agent-Launcher 集成 open-claude-ui (CDS) 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 subagent-driven-development（推荐）或 executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 AI-Agent-Launcher (Tauri v2 + Vue 3) 中纯净集成 open-claude-ui 的 CDS 设计令牌、全量暗色模式、150ms 冷却与 40px 图钉日志滚动物理学、7 档幂律渐隐遮罩、999px 胶囊分段器与脉冲呼吸指示灯。

**架构：** 不引入任何 React 或外部重量级库，基于纯原生 Vue 3 Composition API 与 CSS 变量构建高内聚的组件与 Composables。将全局主题令牌抽离为独立样式表，视口滚动物理学与主题状态机抽离为 Composables，高频交互抽离为原子组件，最后在 `App.vue` 中统一装配。

**技术栈：** Vue 3 (Composition API), TypeScript, Vite, Tauri v2, Vitest (单元测试), CSS Variables & Keyframes.

**规格：** [docs/superpowers/specs/2026-10-18-open-claude-ui-integration-design.md](docs/superpowers/specs/2026-10-18-open-claude-ui-integration-design.md)

---

## 全局约束

- 严格保持 **0 额外大包依赖**：严禁引入 React、Framer Motion 或其他重型运行时，纯 Vue 3 + CSS 实现。
- 严格遵守 CDS 单闭合边框原则（The 1-Border Rule）：已有边框的卡片内部绝不嵌套双重粗边框。
- 遵循 CDS 色彩令牌标准：`--cds-surface-0` ~ `--cds-surface-3`、`--cds-clay: #d97757`、`--cds-border`。
- 滚动锁定窗口严格遵守 $\Delta t \le 150\text{ms}$ 冷却保护和 $\le 40\text{px}$ 吸附容差。
- 每次任务变更必须通过 `vue-tsc --noEmit && vite build`，保证 0 编译错误。

## 审查重点（Review Focus）

1. **日志极速刷屏时的视口抖动**：后台进程每 10ms 抛出多条日志时，如果用户正在回溯历史，不能因冷却定时器精度问题发生轻微跳动；
2. **暗色模式切换闪烁与继承**：首屏加载时若从 localStorage 读取暗色设置，不得发生浅色瞬间闪烁（FOUC）；
3. **Resizer 鼠标移出窗口丢失捕获**：用户拖拽分割条时快速把鼠标划出应用窗口，必须依靠 `setPointerCapture` 平滑保持跟踪，绝不卡死在边缘；
4. **999px 胶囊在动态切换项时的宽度跳变**：多语言切换从短文本变成长文本时，活动滑块的几何计算不能滞后或溢出；
5. **系统主题变化时的自动响应**：当处于跟随系统 (`auto`) 时，操作系统切换深浅色必须毫秒级响应而无需重启应用。

---

## 文件结构计划

```text
src/
├── styles/
│   └── cds-theme.css                  # CDS 全量表面变量、暗色变量、关键帧、发丝边框
├── composables/
│   ├── useTheme.ts                    # 浅色 / 暗色 / 跟随系统主题状态管理
│   ├── useClaudeScrollPinning.ts      # 150ms 冷却与 40px 图钉日志滚动物理算法
│   └── __tests__/
│       ├── useTheme.test.ts           # useTheme 单元测试
│       └── useClaudeScrollPinning.test.ts # 图钉算法单元测试
├── components/
│   ├── CdsSegmentedControl.vue        # 999px 胶囊滑动选择组件
│   ├── CdsStatusDot.vue               # 实时绿光脉冲与陶土呼吸状态灯
│   └── ScrollFadeContainer.vue        # 7-Stop 幂律渐隐遮罩滚动容器
├── types.ts                           # 补充 ThemeSetting 类型
├── i18n.ts                            # 补充主题相关国际化文案
├── main.ts                            # 引入全局样式与初始化主题
├── App.vue                            # 全局重构整合
└── index.html                         # 适配默认背景色
```

---

### 任务 1：测试脚手架与类型定义

**文件：**
- 修改：`package.json`
- 修改：`src/types.ts`
- 修改：`src/i18n.ts`

- [ ] **步骤 1：在 `package.json` 中配置 Vitest 测试依赖**

在 `devDependencies` 添加 `vitest: "^3.0.0"`，在 `scripts` 添加 `"test": "vitest run"`。

- [ ] **步骤 2：在 `src/types.ts` 中定义主题类型**

```typescript
export type ThemeSetting = "auto" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
```

- [ ] **步骤 3：在 `src/i18n.ts` 中添加主题相关多语言文案**

添加 `theme`, `themeAuto`, `themeLight`, `themeDark`, `jumpToBottom`。

- [ ] **步骤 4：验证类型检查通过**

运行：`pnpm run build`  
预期：PASS

- [ ] **步骤 5：Commit**

```bash
git add package.json src/types.ts src/i18n.ts
git commit -m "chore: add vitest and theme type definitions"
```

---

### 任务 2：CDS 设计令牌与暗色模式基座

**文件：**
- 创建：`src/styles/cds-theme.css`
- 创建：`src/composables/useTheme.ts`
- 测试：`src/composables/__tests__/useTheme.test.ts`
- 修改：`src/main.ts`
- 修改：`index.html`

- [ ] **步骤 1：编写 `useTheme` 单元测试**

覆盖 `getSystemTheme()`、设置 `light`/`dark`/`auto`、`localStorage` 持久化及 DOM 类名同步。

- [ ] **步骤 2：运行测试验证失败**

运行：`pnpm run test`  
预期：FAIL (`useTheme` not found)

- [ ] **步骤 3：实现 `src/styles/cds-theme.css`**

包含：
- `:root` 浅色表面色阶梯 (`--cds-surface-0` ~ `--cds-surface-3`)、陶土色 (`--cds-clay: #d97757`)、`--cds-border`、阴影体系；
- `[data-theme="dark"]` / `.dark` 黑曜石色阶梯 (`#0b0b0a` ~ `#20201e`)、发丝内高光、暗色陶土色；
- 动效关键帧：`@keyframes dframe-dot-pulse` (1.2s)、`@keyframes dframe-clay-breathe` (2.4s)。

- [ ] **步骤 4：实现 `src/composables/useTheme.ts`**

导出 `themeSetting`、`resolvedTheme`、`setThemeSetting`、`initTheme`，并监听 `matchMedia`。

- [ ] **步骤 5：在 `src/main.ts` 和 `index.html` 中引入并初始化主题**

在 `main.ts` 中 `import "./styles/cds-theme.css"`，调用 `initTheme()`；在 `index.html` 中将硬编码背景替换为 `background: var(--cds-surface-0, #f5f5f3)`。

- [ ] **步骤 6：运行测试验证通过**

运行：`pnpm run test`  
预期：PASS

- [ ] **步骤 7：运行构建验证**

运行：`pnpm run build`  
预期：PASS

- [ ] **步骤 8：Commit**

```bash
git add src/styles/cds-theme.css src/composables/useTheme.ts src/composables/__tests__/useTheme.test.ts src/main.ts index.html
git commit -m "feat(theme): add CDS design tokens and dark mode engine"
```

---

### 任务 3：日志视口物理学算法 (`useClaudeScrollPinning`)

**文件：**
- 创建：`src/composables/useClaudeScrollPinning.ts`
- 测试：`src/composables/__tests__/useClaudeScrollPinning.test.ts`

- [ ] **步骤 1：编写 `useClaudeScrollPinning` 单元测试**

测试点：
1. 初始状态为 `isPinned = true`；
2. 用户输入冷却保护：当 `performance.now() - lastUserInputAt <= 150` 时，阻止执行滚动吸底；
3. 底部吸附容差：计算 `distanceFromBottom`，$\le 40\text{px}$ 时判定为图钉锁定状态；
4. 离开底部 $> 120\text{px}$ 时显示 `showJumpControl = true`。

- [ ] **步骤 2：运行测试验证失败**

运行：`pnpm run test`  
预期：FAIL

- [ ] **步骤 3：实现 `src/composables/useClaudeScrollPinning.ts`**

实现完整图钉算法状态机，提供 `containerRef`、`isPinned`、`showJumpControl`、`markUserInput`、`handleScroll`、`scrollToBottom`。

- [ ] **步骤 4：运行测试验证通过**

运行：`pnpm run test`  
预期：PASS

- [ ] **步骤 5：Commit**

```bash
git add src/composables/useClaudeScrollPinning.ts src/composables/__tests__/useClaudeScrollPinning.test.ts
git commit -m "feat(log): implement 150ms cooldown and 40px scroll pinning algorithm"
```

---

### 任务 4：7 档非线性幂律渐隐遮罩 (`ScrollFadeContainer.vue`)

**文件：**
- 创建：`src/components/ScrollFadeContainer.vue`

- [ ] **步骤 1：实现 `ScrollFadeContainer.vue`**

特性：
1. Props: `orientation` (`'vertical' | 'horizontal' | 'both'`)，`fadeSize` (默认 24px)；
2. 反编译自 CDS 的 7-Stop 幂律感知感知拟合 Mask（`0%`, `15%`, `30%`, `45%`, `60%`, `75%`, `87%`, `100%`）；
3. 触顶时顶部羽化设为 0，触底时底部羽化设为 0；
4. 使用 `ResizeObserver` 保持自适应。

- [ ] **步骤 2：类型检查与构建验证**

运行：`pnpm run build`  
预期：PASS

- [ ] **步骤 3：Commit**

```bash
git add src/components/ScrollFadeContainer.vue
git commit -m "feat(ui): add 7-stop non-linear scroll fade container"
```

---

### 任务 5：原子交互组件 (`CdsSegmentedControl.vue` & `CdsStatusDot.vue`)

**文件：**
- 创建：`src/components/CdsSegmentedControl.vue`
- 创建：`src/components/CdsStatusDot.vue`

- [ ] **步骤 1：实现 `CdsSegmentedControl.vue`**

特性：
1. 999px 全圆角胶囊与 2px 几何边距；
2. 纯几何计算滑动指示器位移与宽度，150ms 阻尼过渡；
3. 支持键盘方向键切换与无障碍 `role="tablist"`；
4. 支持 `sm` 和 `md` 两种紧凑尺寸。

- [ ] **步骤 2：实现 `CdsStatusDot.vue`**

特性：
1. 运行中：双环绿光扩散脉冲（1.2s ping 动效）；
2. 待机/保存中：陶土明度呼吸（2.4s breathe 动效）；
3. 停止/失败：静止状态指示。

- [ ] **步骤 3：类型检查与构建验证**

运行：`pnpm run build`  
预期：PASS

- [ ] **步骤 4：Commit**

```bash
git add src/components/CdsSegmentedControl.vue src/components/CdsStatusDot.vue
git commit -m "feat(ui): add CdsSegmentedControl and CdsStatusDot components"
```

---

### 任务 6：重构整合 `App.vue`（暗色模式、图钉日志、指针捕获分割条、CDS 视觉阶梯）

**文件：**
- 修改：`src/App.vue`

- [ ] **步骤 1：将顶栏控制区集成 CDS 胶囊分段器与主题切换**

- 语言选择升级为 `CdsSegmentedControl`；
- 增加深色 / 浅色 / 跟随系统主题选择器（同样使用 `CdsSegmentedControl`）；
- 运行指示灯升级为 `CdsStatusDot`。

- [ ] **步骤 2：将日志终端视口升级为 `useClaudeScrollPinning` 与 `ScrollFadeContainer`**

- 日志滚动容器接入 `containerRef` 与 `handleScroll`；
- 添加“回到底部”悬浮微按钮（当 `showJumpControl` 为 true 时平滑浮现）；
- 将日志内容和配置表单分别用 `ScrollFadeContainer` 包裹，呈现柔润边缘。

- [ ] **步骤 3：加固垂直与水平分栏 Resizer**

- 接入 `setPointerCapture(e.pointerId)`；
- 引入 4px 移动死区，防止误触微位移；
- 支持双击复位基线（侧栏 200px，日志 220px）。

- [ ] **步骤 4：全面重构 CSS 为 CDS 变量与单闭合边框**

- 将写死的颜色替换为 `var(--cds-surface-0)` ~ `var(--cds-surface-3)`、`var(--cds-border)`、`var(--cds-clay)` 等；
- 所有按钮应用 `:active:scale-[0.98]` 瞬时微弹反馈；
- 全面校验暗色模式下的可读性与文字对比度。

- [ ] **步骤 5：运行测试与类型构建验证**

运行：`pnpm run test && pnpm run build`  
预期：PASS

- [ ] **步骤 6：Commit**

```bash
git add src/App.vue
git commit -m "refactor(ui): integrate CDS theme, scroll pinning, and refined physics in App.vue"
```

---

### 任务 7：端到端与功能回归验证

**文件：**
- 校验：全项目

- [ ] **步骤 1：运行全套单元测试与全量类型检查**

运行：`pnpm run test && pnpm run build`  
预期：PASS，0 错误

- [ ] **步骤 2：功能回归验证**

验证：
1. Agent 创建、复制、删除正常；
2. 进程启动、停止、重启、清理残留进程正常；
3. 开机自启、PATH 探测正常；
4. 语言切换与主题切换实时响应并持久化；
5. 日志搜索与高亮匹配正常；
6. 检查更新与关于弹窗正常。

- [ ] **步骤 3：最终清理与提交**

```bash
git status
git commit -m "chore: complete open-claude-ui CDS integration and regression verification"
```
