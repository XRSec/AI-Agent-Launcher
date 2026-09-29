# AI Agent Launcher

一个基于 **Tauri v2 + Rust + Vue 3** 打造的现代化跨平台 AI Agent 启动与进程管理桌面工具，支持 **macOS · Windows · Linux**。

告别在终端频繁输入复杂命令、繁琐配置环境路径以及翻查后台僵死进程——只需录入专属于每个 Agent 的启动命令或脚本，即可一键并发启动、停止、异常自动恢复，并在专属控制台中实时查看运行日志。

---

## ✨ 核心特性

- **跨平台原生支持**：
  - **macOS**：原生适配 Universal Binary 通用架构（一份安装包原生兼容 Apple Silicon M1~M4 与 Intel 芯片）。
  - **Windows**：原生适配 Windows 10 / 11（支持 NSIS 与 MSI 安装包）。
  - **Linux**：原生适配 Ubuntu、Debian 等各大主流发行版（支持 `.deb` 与 `.AppImage`）。
- **Claude Code 风格暖色美学**：
  - 采用优雅沉稳的暖沙黄（`#f7f4ee`）与暖岩色边框，告别刺眼的纯白与纯黑生硬反差。
  - 专属实时日志控制台采用深色暖焦炭黑（`#1c1917`）高对比设计，滚动条与控制台深度同步。
  - 侧边栏支持平滑拖拽缩放与极窄 Rail 导轨折叠模式。
- **三档多语言国际化（i18n）**：
  - **自动 / Auto**：智能感知操作系统与浏览器语言，中文环境显示中文，其余环境自动切换为 English。
  - **中文**：全界面强制简体中文。
  - **English**：全界面强制英文。
  - 位于界面右上角，一键即时无缝热切换，本地持久化记忆。
- **孤儿进程与端口防泄漏清理（Residual Cleaner）**：
  - 针对外部应用意外退出或终端闪退导致后台进程未退、端口被占的情况，提供专属「清理」机制。
  - 自动递归探测孤儿进程与占用端口并强制释放，确保随时干净重启。
- **多 Profile 独立配置管理**：
  - **命令 / 启动脚本**：独立编辑专属代码，支持多行输入与 Tab 缩进。
  - **执行 Shell**：支持自动识别或指定解释器（macOS/Linux: `/bin/zsh`、`/bin/bash`、`/bin/sh`；Windows: `PowerShell`、`cmd`、`pwsh`）。
  - **独立 PATH 环境变量**：可自定义隔离 PATH，内置“一键检测”当前系统环境（Homebrew、Volta、Nvm、Pyenv、Cargo、Local Bin 等）。
  - **重启策略 (Restart Policy)**：
    - `不自动重启 (no)`：手动控制启停。
    - `始终自动重启 (always)`：进程退出后按设定延迟重新拉起。
    - `仅异常退出时重启 (on-failure)`：仅在退出码非 0 时自动恢复。
    - **重启延迟**：支持自定义间隔秒数（1~60s）。
  - **开机/随软件自启**：标记随 Launcher 启动后自动拉起。
  - **工作目录**：原生目录选择弹窗，轻松绑定执行根目录。
- **专属实时日志控制台**：
  - 每个 Agent 拥有独立的日志缓冲流，互不干扰。
  - 支持智能自动滚动、大文本安全截断与一键清空日志。
- **系统托盘常驻 (System Tray)**：
  - 支持窗口最小化到托盘、托盘菜单常驻以及一键唤起主窗口。

---

## 🚀 本地开发与构建

### 前置环境

- [Node.js](https://nodejs.org/) (>= 18) 与 [pnpm](https://pnpm.io/) (>= 9)
- [Rust](https://www.rust-lang.org/) (>= 1.77)
- Linux 用户需安装 WebKitGTK 与相关开发库（见下文常见构建依赖）

### 常用命令（Makefile）

项目内置了标准 `Makefile`，开发者可直接使用以下指令：

```bash
# 查看所有可用指令与说明
make help

# 1. 安装项目依赖
make install

# 2. 启动桌面端开发模式（热重载）
make dev

# 3. 执行全套代码类型检查（Vue 3 TS + Rust 编译检查）
make check

# 4. 运行 Rust 后端单元测试
make test

# 5. 从 src-tauri/icons/icon.svg 一键重新生成多平台全尺寸应用图标并同步至 Web
make icons

# 6. 构建正式 Release 发布包
make build

# 7. 清理历史遗留资源与构建产物
make clean          # 清理 dist 与 release bundle
make clean-legacy   # 清理历史旧版 Swift/SPM 遗留缓存与无用旧资源
make clean-all      # 深度清理（包含 node_modules 与 target 缓存）
```

也可以直接使用 npm/pnpm 脚本：

```bash
# 本地开发
pnpm tauri dev

# 打包构建
pnpm tauri build
```

打包产物位于 `src-tauri/target/release/bundle/`：
- **macOS**：`macos/AI Agent Launcher.app` 与 `dmg/AI Agent Launcher_1.0.0_universal.dmg`
- **Windows**：`nsis/AI Agent Launcher_1.0.0_x64-setup.exe` 与 `msi/`
- **Linux**：`deb/` 与 `appimage/`

---

## 📁 项目架构

```text
AI-Agent-Launcher/
├── Makefile                   # 统一构建、开发与清理任务管理
├── package.json               # 前端依赖与构建脚本配置
├── vite.config.ts             # Vite 配置
├── tsconfig.json              # TypeScript 编译配置
├── index.html                 # 前端 HTML 模板与全局防选中样式
├── public/                    # 静态 Web 资源 (Vite)
│   ├── app-icon.svg           # 矢量 SVG 图标
│   └── favicon.png            # 页面微标
├── src/                       # 前端渲染层 (Vue 3 + TypeScript + Vite)
│   ├── App.vue                # 主界面：Profile 侧栏、配置卡片、实时终端与顶栏
│   ├── i18n.ts                # 自动/中文/英文三档国际化字典与响应式切换逻辑
│   ├── main.ts                # 前端主入口
│   └── types.ts               # TypeScript 数据模型与接口定义
├── src-tauri/                 # 原生后端 (Rust + Tauri v2)
│   ├── Cargo.toml             # Rust 依赖声明 (tauri, tokio, uuid, serde 等)
│   ├── tauri.conf.json        # 窗口布局、应用标识、权限与打包规格配置
│   ├── capabilities/          # Tauri v2 安全权限与功能白名单
│   ├── icons/                 # 全平台多尺寸生成的图标资源与母版 (icon.svg)
│   └── src/
│       ├── main.rs            # Rust 程序入口
│       ├── lib.rs             # 全局状态管理、生命周期钩子与插件挂载
│       ├── menu.rs            # 原生系统菜单栏 (Standard App Menu & Shortcuts)
│       ├── tray.rs            # 跨平台系统托盘常驻与菜单
│       ├── runner.rs          # 进程树管理、Shell 执行、孤儿端口查杀与自动重启
│       ├── commands.rs        # 前后端交互 IPC Command 接口
│       ├── models.rs          # 数据结构定义 (AgentProfile, ProfileStatus 等)
│       ├── path_detect.rs     # 系统全局 PATH 自动侦测逻辑
│       └── state.rs           # 本地 JSON 数据持久化与并发锁状态
└── .github/workflows/         # 持续集成与发布
    ├── ci.yml                 # 跨平台 (macOS/Ubuntu/Windows) PR 与 Push 自动化检查
    └── release.yml            # 基于 tauri-action@v1 的多平台自动构建发布工作流
```

---

## 📄 开源许可证

本项目基于 [MIT License](LICENSE) 开源。
