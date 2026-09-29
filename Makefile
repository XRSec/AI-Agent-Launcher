# ==============================================================================
# AI Agent Launcher - Makefile
# ==============================================================================

SHELL := /bin/bash
export PATH := $(HOME)/.cargo/bin:$(PATH)
ROOT_DIR := $(shell pwd)
TAURI_DIR := $(ROOT_DIR)/src-tauri
ICON_SOURCE := $(TAURI_DIR)/icons/icon.svg

.PHONY: help install dev build check test icons clean clean-all clean-legacy

## help: 显示所有可用命令
help:
	@echo ""
	@echo "AI Agent Launcher - 开发与构建命令："
	@echo ""
	@awk '/^[a-zA-Z\-\_0-9]+:/ { \
		helpMessage = match(lastLine, /^## (.*)/); \
		if (helpMessage) { \
			helpCommand = substr($$1, 0, index($$1, ":")-1); \
			helpDesc = substr(lastLine, index(lastLine, ":") + 2); \
			printf "  \033[36m%-16s\033[0m %s\n", helpCommand, helpDesc; \
		} \
	} \
	{ lastLine = $$0 }' $(MAKEFILE_LIST)
	@echo ""

## install: 安装前端依赖与初始化环境
install:
	pnpm install

## dev: 启动本地桌面端开发模式（热重载）
dev:
	pnpm tauri dev

## build: 构建正式 Release 发布包（.app / .dmg / .deb / .exe）
build:
	pnpm tauri build

## check: 代码类型检查（Vue 3 TS + Rust 编译检查）
check:
	pnpm build
	cd $(TAURI_DIR) && cargo check --all-targets

## test: 运行 Rust 后端单元测试
test:
	cd $(TAURI_DIR) && cargo test

## icons: 从 src-tauri/icons/icon.svg 重新生成全平台应用图标并同步至 Web
icons:
	@if [ -f "$(ICON_SOURCE)" ]; then \
		echo "==> 生成全平台应用图标..."; \
		pnpm tauri icon $(ICON_SOURCE); \
		cp $(ICON_SOURCE) $(ROOT_DIR)/public/app-icon.svg; \
		cp $(TAURI_DIR)/icons/32x32.png $(ROOT_DIR)/public/favicon.png; \
		echo "==> 图标生成并同步完成！"; \
	else \
		echo "错误: 未找到 $(ICON_SOURCE)"; \
		exit 1; \
	fi

## clean: 清理构建输出产物（dist / release bundle）
clean:
	@echo "==> 清理构建产物..."
	rm -rf $(ROOT_DIR)/dist
	rm -rf $(TAURI_DIR)/target/release/bundle
	@echo "==> 完成！"

## clean-legacy: 清理历史 Swift / PiWeb 遗留的无用旧资源与构建缓存
clean-legacy:
	@echo "==> 清理历史遗留资源..."
	rm -rf $(ROOT_DIR)/.build
	rm -rf $(ROOT_DIR)/.swiftpm
	rm -rf $(ROOT_DIR)/Resources
	rm -rf $(ROOT_DIR)/Sources
	find . -name ".DS_Store" -depth -exec rm -f {} \;
	@echo "==> 历史旧资源已彻底清理干净！"

## clean-all: 深度清理（包含 node_modules 与 target 缓存）
clean-all: clean clean-legacy
	@echo "==> 深度清理依赖与缓存..."
	rm -rf $(ROOT_DIR)/node_modules
	rm -rf $(TAURI_DIR)/target
	rm -f /tmp/tauri_run*.log
	@echo "==> 深度清理完成！"
