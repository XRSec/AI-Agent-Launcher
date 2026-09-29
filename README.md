<div align="center">

<img src="public/app-icon.svg" alt="AI Agent Launcher Logo" width="96" height="96" />

# AI Agent Launcher

**跨平台轻量级 AI Agent 与本地服务进程管理桌面应用**  
*A modern, lightweight cross-platform desktop process manager for AI Agents & developer services.*

[![CI](https://github.com/XRSec/Pi-Web-Launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/XRSec/Pi-Web-Launcher/actions/workflows/ci.yml)
[![Release](https://github.com/XRSec/Pi-Web-Launcher/actions/workflows/release.yml/badge.svg)](https://github.com/XRSec/Pi-Web-Launcher/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-24C8D8?logo=tauri&logoColor=white)](https://tauri.app)
[![Vue 3](https://img.shields.io/badge/Vue-3.5-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-1.77+-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](#-跨平台原生支持)

[English](#-overview-en) · [中文文档](#-核心特性) · [快速上手](#-本地开发与构建) · [项目架构](#-项目架构)

</div>

---

## 💡 为什么需要 AI Agent Launcher？

在开发或使用各种本地 AI Agent（如 Claude Code、Aider、OpenHands、Dify、AutoGen 或自研 Agent 脚本）时，开发者常面临这些痛点：
1. **终端进程易失控**：终端窗口繁多，关闭窗口或应用闪退后，后台 Python/Node 孤儿进程未退出，导致端口被占、反复报错。
2. **环境 PATH 难对齐**：桌面环境下启动的进程常因无法继承终端的环境变量（如 Homebrew、NVM、Pyenv、Cargo）而报错“command not found”。
3. **缺乏自动容灾自愈**：长时间运行的 Agent 偶发崩溃退出时，缺少自动化重试与异常拉起机制。
4. **日志混杂难查**：不同 Agent 共享同一个终端时输出日志难以隔离回溯。

**AI Agent Launcher** 专为解决上述痛点而生：基于 **Tauri v2 + Rust + Vue 3** 构建，提供开箱即用的多 Profile 隔离管理、孤儿进程查杀释放、环境变量自动探测与独立控制台日志流。

---

## ✨ 核心特性

### 🖥️ 跨平台原生支持
- **macOS**：原生适配 Universal Binary 通用架构（一份 DMG 安装包原生免转译兼容 Apple Silicon M1~M4 与 Intel 芯片）。
- **Windows**：原生适配 Windows 10 / 11（生成轻量 NSIS 安装包与 MSI 安装包，支持开机启动与托盘常驻）。
- **Linux**：原生适配 Ubuntu、Debian 等各大主流发行版（提供 `.deb` 与 `.AppImage`）。

### 🎨 Claude Code 暖色设计美学
- 界面采用优雅沉稳的暖沙黄（`#f7f4ee`）与暖岩色边框，告别刺眼纯白与纯黑生硬反差。
- 专属实时日志控制台采用深色暖焦炭黑（`#1c1917`）高对比设计，滚动条与控制台深度同步。
- 侧边栏支持平滑拖拽调整宽度与极窄 Rail 导轨模式。

### 🌐 三档语言即时切换（i18n）
- **自动 / Auto**：智能感知操作系统与浏览器默认语言（中文环境显示中文，其余环境自动切换为 English）。
- **中文**：全界面强制简体中文。
- **English**：全界面强制英文。
- 位于右上角顶栏，无需重启即时响应，本地持久化记忆。

### 🛡️ 孤儿进程与端口防泄漏清理（Residual Cleaner）
- 一键排查并递归杀死意外退出的残留后台进程。
- 针对配置命令中声明的端口进行智能解析，强制释放被占用的端口资源，确保随时干净重启。

### ⚙️ 多 Profile 独立隔离配置
- **命令与脚本编辑**：独立编辑专属代码，支持多行输入与 Tab 缩进；脚本内直接 `export` 环境变量可覆盖外部配置。
- **执行 Shell 解释器**：支持自动识别或指定解释器（macOS/Linux: `/bin/zsh`、`/bin/bash`、`/bin/sh`；Windows: `PowerShell`、`cmd`、`pwsh`）。
- **独立 PATH 环境变量**：内置“一键检测”当前系统全局环境（自动合并 Homebrew、Volta、Nvm、Pyenv、Cargo、Local Bin 等）。
- **自愈重启策略 (Restart Policy)**：
  - `不自动重启 (no)`：手动控制启停。
  - `始终自动重启 (always)`：退出后按设定延迟重新拉起。
  - `仅异常退出时重启 (on-failure)`：仅在退出码非 0 时自动恢复。
  - **重启延迟**：支持自定义间隔秒数（1~60s）。
- **自启与工作目录**：支持设置随 Launcher 启动后自动拉起，并通过原生系统弹窗绑定工作目录。

### 📜 专属实时日志控制台
- 每个 Agent 拥有独立的日志缓冲队列与 ANSI 控制台，互不干扰。
- 支持智能自动吸底滚动、安全容量截断（超限自动清理旧日志）与一键清空日志。

### 🔲 系统托盘常驻 (System Tray)
- 关闭窗口自动收起至系统托盘，后台持续运行。
- 点击托盘图标或菜单一键唤起主窗口或优雅退出。

---

## 🌐 Overview (EN)

**AI Agent Launcher** is a cross-platform desktop process runner engineered for AI Agents and background developer services. Built with Tauri v2, Vue 3, and Rust, it replaces cluttered terminal windows with dedicated profiles, auto-restart policies, real-time logs, and automatic orphan port/process cleanup.

- **Universal macOS, Windows, and Linux support**.
- **Claude Code-inspired warm aesthetic** with responsive dark console.
- **Trilingual switcher**: `Auto` (system-detected), `中文`, and `English`.
- **Orphan process & port cleanup** to resolve "port already in use" errors after crashes.
- **Per-agent isolated logs, working directories, and custom PATHs**.

---

## 🚀 本地开发与构建

### 前置环境

- [Node.js](https://nodejs.org/) (>= 18) 与 [pnpm](https://pnpm.io/) (>= 9)
- [Rust](https://www.rust-lang.org/) (>= 1.77)
- Linux 构建依赖（Ubuntu / Debian）：
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential
  ```

### 常用命令（Makefile）

项目内置了标准 `Makefile`，开发者可直接使用以下指令：

```bash
# 查看所有可用指令与说明
make help

# 1. 安装项目依赖
make install

# 2. 启动桌面端开发模式（支持热重载）
make dev

# 3. 执行全套代码类型检查（Vue 3 TS + Rust Clippy 编译检查）
make check

# 4. 运行 Rust 后端单元测试
make test

# 5. 从 src-tauri/icons/icon.svg 一键重新生成多平台全尺寸应用图标并同步至 Web
make icons

# 6. 构建正式 Release 发布包
make build

# 7. 清理构建产物与依赖缓存
make clean          # 清理 dist 与 release bundle
make clean-legacy   # 清理历史旧版残留缓存与无用旧资源
make clean-all      # 深度清理（包含 node_modules 与 target 缓存）
```

也可以直接使用 pnpm 脚本：

```bash
pnpm dev           # 前端 Vite 调试
pnpm tauri dev     # 桌面端调试
pnpm tauri build   # 桌面端打包构建
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
├── vite.config.ts             # Vite 构建配置
├── tsconfig.json              # TypeScript 编译配置
├── index.html                 # 前端 HTML 模板与全局防选中样式
├── LICENSE                    # MIT 开源许可证
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
