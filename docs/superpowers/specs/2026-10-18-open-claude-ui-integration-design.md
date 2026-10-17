# AI-Agent-Launcher 集成 open-claude-ui (Claude Design System) 设计规格

**日期**: 2026-10-18  
**状态**: Approved / In-Progress  
**作者**: 星冉 & Claude Fable 5  

---

## 1. 背景与目标

[Caho1/open-claude-ui](https://github.com/Caho1/open-claude-ui) 是一个逆向还原 Anthropic 官方 Claude Desktop (CDS) 视觉设计系统与物理交互引擎的高质量开源实现。
当前项目 `AI-Agent-Launcher` 是基于 **Tauri v2 + Vue 3 + TypeScript** 构建的 AI Agent 桌面运行与进程管控工具。

本项目目标：
1. **零多余运行时开销**：不直接引入 React 或大型 UI 框架，在原生 Vue 3 中纯净复刻 CDS 视觉令牌与工程物理算法。
2. **CDS 视觉基准与全量暗色模式**：引入暖灰表面色阶梯（Surface-0 ~ Surface-3）、陶土色主 Accent（`#d97757`）、单闭合边框原则（The 1-Border Rule）、以及高对比黑曜石暗色模式。
3. **关键交互工程物理学**：
   - 移植 150ms 用户输入冷却窗口 + 40px 底部容差图钉锁定算法，彻底消除高频日志输出下的视口抢夺；
   - 引入贴合 $y \approx x^{1.5}$ 幂律感知曲线的 7 档非线性 Alpha 渐隐滚动遮罩；
   - 引入原生 `setPointerCapture` 全局指针捕获、4px 移动死区与双击基线吸附的 Resizer 分割条；
4. **高质量原子组件封装**：
   - `CdsSegmentedControl.vue`：999px 全圆角胶囊滑动分段控制器；
   - `CdsStatusDot.vue`：1.2s 实时活动绿光脉冲与 2.4s 陶土明度呼吸灯；
   - `ScrollFadeContainer.vue`：双轴非线性渐隐滚动容器。

---

## 2. 系统架构与文件拓扑

```text
src/
├── styles/
│   └── cds-theme.css                  # CDS 核心令牌、表面色阶、单闭合边框、暗色变量、关键帧动画
├── composables/
│   ├── useTheme.ts                    # 浅色 / 暗色 / 跟随系统主题管理与持久化
│   └── useClaudeScrollPinning.ts      # 150ms 冷却时间戳与 40px 图钉日志滚动锁定算法
├── components/
│   ├── CdsSegmentedControl.vue        # 999px 全圆角胶囊滑动选择组件
│   ├── CdsStatusDot.vue               # 运行态扩散绿光脉冲与待机呼吸指示灯
│   └── ScrollFadeContainer.vue        # 7-Stop 非线性幂律 Alpha 渐隐遮罩滚动容器
├── types.ts                           # 补充 ThemeSetting 相关类型定义
├── i18n.ts                            # 补充主题相关多语言文案 (深色/浅色/跟随系统)
├── main.ts                            # 引入全局 cds-theme.css 并初始化主题
├── App.vue                            # 重构整合 CDS 样式、分栏拖拽死区、图钉日志视口
└── index.html                         # 适配 CDS 默认背景色与暗色模式基底
```

---

## 3. 详细技术实现方案

### 3.1 CDS 令牌体系与主题引擎 (`src/styles/cds-theme.css`, `useTheme.ts`)

#### 色彩与表面变量
* **浅色模式 (`:root`)**：
  * `--cds-surface-0: #f5f5f3`：全局 Canvas 基底
  * `--cds-surface-1: #fcfcfb`：侧边栏与中间层
  * `--cds-surface-2: #ffffff`：配置卡片与交互组件表面
  * `--cds-surface-3: #ffffff`：浮动面板与输入框表面
  * `--cds-clay: #d97757`：陶土品牌主强调色
  * `--cds-clay-hover: #cc5a36`：悬停加深
  * `--cds-clay-light: #fbf3ef`：极浅背景色
  * `--cds-border: hsla(0, 0%, 8%, 0.10)`：单闭合标准细边框
  * `--cds-border-strong: hsla(0, 0%, 8%, 0.20)`
  * `--cds-shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05), 0 2px 8px 0 rgba(0, 0, 0, 0.08)`
  * `--cds-shadow-md: 0 2px 4px 0 rgba(0, 0, 0, 0.05), 0 6px 16px 0 rgba(0, 0, 0, 0.08)`
* **暗色模式 (`.dark`, `[data-theme="dark"]`)**：
  * `--cds-surface-0: #0b0b0a`：黑曜石深色背景
  * `--cds-surface-1: #151515`：侧边栏表面
  * `--cds-surface-2: #1a1a19`：卡片表面
  * `--cds-surface-3: #20201e`：浮动表面
  * `--cds-clay: #d97757`，`--cds-clay-hover: #e08264`
  * `--cds-border: hsla(0, 0%, 100%, 0.09)`
  * `--cds-border-strong: hsla(0, 0%, 100%, 0.18)`
  * `--cds-shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.4), 0 2px 8px 0 rgba(0, 0, 0, 0.4)`

#### 主题持久化与同步 (`useTheme.ts`)
* 支持 `auto` | `light` | `dark`。
* 存储键：`localStorage.getItem("launcher_theme_setting")`。
* `auto` 模式下监听 `window.matchMedia("(prefers-color-scheme: dark)")`，动态给 `document.documentElement` 添加/移除 `dark` 类名并设置 `data-theme` 属性。

---

### 3.2 日志视口物理学 (`useClaudeScrollPinning.ts`)

#### 核心数学与物理公式
1. **用户输入冷却判定**：
   $$\Delta t = \text{performance.now()} - \text{lastUserInputAt}$$
   当 $\Delta t \le 150\text{ms}$ 时，判定为用户处于物理触摸或滚轮交互中，**绝不调用**程序化滚动。
2. **图钉锁定容差**：
   $$\text{distanceFromBottom} = \text{scrollHeight} - \text{scrollTop} - \text{clientHeight}$$
   当 $\text{distanceFromBottom} \le 40\text{px}$ 时，自动设为图钉锁定状态（`isPinned = true`）。
3. **回到底部控制器**：
   当用户主动往上回滚且 $\text{distanceFromBottom} > 120\text{px}$ 时，淡入显示 Claude 风格的“回到底部”悬浮气泡，点击后以平滑动画恢复图钉锁定。

---

### 3.3 7 档非线性幂律渐隐遮罩 (`ScrollFadeContainer.vue`)

#### 幂律曲线梯度与遮罩
根据 CDS 逆向分析的感知平滑曲线 $y \approx x^{1.5}$，配置如下 7 档 Stop：
* Stops: `0%, 15%, 30%, 45%, 60%, 75%, 87%, 100%`
* 对应 Alpha: `transparent`, `0.05`, `0.15`, `0.30`, `0.50`, `0.70`, `0.85`, `#000`
* 结合容器的 `scrollTop` 与 `scrollHeight` 状态：
  * 触顶（`scrollTop <= 2`）时，`--cds-scroll-fade-top = 0px`；
  * 滚入时动态生效 24px 羽化边缘；
  * 触底时，`--cds-scroll-fade-bottom = 0px`。

---

### 3.4 交互原子组件设计

#### 1. `CdsSegmentedControl.vue`
* **Props**:
  * `items`: `{ label: string; value: string | number; icon?: string }[]`
  * `modelValue`: `string | number`
  * `size`: `'sm' | 'md'`（默认 `'md'`）
* **视觉规范**:
  * 外部轨道：`border-radius: 999px; padding: 2px; background: var(--cds-segmented-track)`；
  * 内部活动滑块：纯几何等分滑动与阴影 `box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08)`，150ms 阻尼吸附。
* **应用场景**:
  * 顶栏语言切换（自动 / 中文 / EN）；
  * 顶栏主题切换（自动 / 浅色 / 深色）；
  * 进程重启策略切换（不自动 / 始终 / 失败退出时）。

#### 2. `CdsStatusDot.vue`
* **Props**:
  * `status`: `'running' | 'stopped' | 'failed' | 'breathe'`
  * `label?: string`
  * `pulse?: boolean`
* **动效机制**:
  * `running`：实时活动扩散双环绿灯（外环 `@keyframes dframe-dot-pulse 1.2s infinite ease-out`）；
  * `breathe`：陶土明度呼吸（`@keyframes dframe-clay-breathe 2.4s infinite ease-in-out`）。

#### 3. 分割条（Resizer）动力学加固
* **Pointer Stream & Capture**:
  * 使用 `@pointerdown` 配合 `setPointerCapture(e.pointerId)`，保证即便光标高速移出窗口或滑入非选区容器，依然平滑接收位移事件；
  * 拖拽期间为页面根节点标记 `is-resizing`，禁用选区与交互抖动。
* **4px 移动死区**:
  * 只有当鼠标位移绝对值超过 4px 时才判定进入实际拖拽状态，防止鼠标单纯点击时产生微小坐标漂移。
* **双击复位吸附**:
  * 双击垂直分割线：平滑归位至默认 200px 侧栏宽度；
  * 双击水平分割线：平滑归位至默认 220px 日志面板高度。

---

## 4. 验证与测试标准

1. **类型检查与前端编译**：
   - 运行 `pnpm run build`（执行 `vue-tsc --noEmit && vite build`），必须 0 警告、0 错误通过。
2. **主题适配与对比度验证**：
   - 在亮色与暗色模式下，检查文本、卡片、输入框、代码区、日志面板的视觉层次与对比度；
   - 检查高分屏下的视网膜细线渲染与光渗补偿。
3. **高频日志图钉算法验证**：
   - 启动 Agent 产生持续高速日志流（如 `while true; do echo "..."; sleep 0.05; done`）；
   - 验证用户滚轮上滑时不会被拉回底部；
   - 验证用户滚回底部 40px 内或点击“回到底部”悬浮按键时恢复自动吸附。
4. **组件功能回归验证**：
   - 验证 Agent 的添加、复制、删除、配置修改、开机自启、PATH 检测等既有核心功能完全不受影响。
