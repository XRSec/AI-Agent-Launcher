import { ref, computed } from "vue";

export type LanguageSetting = "auto" | "zh-CN" | "en-US";
export type ResolvedLocale = "zh-CN" | "en-US";

export const languageSetting = ref<LanguageSetting>(
  (localStorage.getItem("launcher_language_setting") as LanguageSetting) || "auto"
);

export function getSystemLocale(): ResolvedLocale {
  if (typeof navigator !== "undefined" && navigator.language) {
    const lang = navigator.language.toLowerCase();
    if (lang.startsWith("zh")) {
      return "zh-CN";
    }
  }
  return "en-US";
}

export const currentLocale = computed<ResolvedLocale>(() => {
  if (languageSetting.value === "auto") {
    return getSystemLocale();
  }
  return languageSetting.value;
});

export function setLanguageSetting(setting: LanguageSetting) {
  languageSetting.value = setting;
  localStorage.setItem("launcher_language_setting", setting);
}

export const messages = {
  "zh-CN": {
    appTitle: "AI Agent Launcher",
    runningCount: (n: number) => `${n} 个运行中`,
    allStopped: "全部已停止",
    cleanResiduals: "清理",
    cleanResidualsTooltip: "若因上次应用闪退导致端口被占或无法启动，点击强制杀死孤儿进程并释放端口",
    cleanedToast: "已清理残留进程与被占端口",
    agentList: "Agent 列表",
    newAgent: "新建",
    duplicate: "复制",
    delete: "删除",
    copySuffix: "副本",
    unnamedAgent: "未命名 Agent",
    statusRunning: "运行中",
    statusStopped: "已停止",
    statusFailed: "异常退出",
    btnStart: "启动",
    btnStop: "停止",
    btnRestart: "重启",
    btnClean: "清理",
    btnClear: "清空",
    autoScroll: "自动滚动",
    realtimeLogs: "专属实时日志",
    emptyLog: "暂无日志输出，点击顶部「启动」开始运行",
    agentName: "Agent 名称",
    agentNamePlaceholder: "Agent 名称",
    configChangedHint: "配置已修改，重启生效",
    configSavedHint: "配置已保存",
    commandCardTitle: "Shell 启动命令",
    commandCardDesc: "多行启动命令，支持 Tab 缩进；脚本内直接 export 环境变量或 PATH 可覆盖外部环境",
    commandPlaceholder: "# 输入启动命令或脚本，例如：\nexport PORT=8080\nexport PATH=\"/opt/homebrew/bin:$PATH\"\npython -m agent --port $PORT",
    runtimeCardTitle: "运行时环境",
    runtimeCardDesc: "配置进程工作目录、执行 Shell 与环境变量 PATH",
    workingDirTitle: "工作目录",
    workingDirDesc: "进程执行时的根目录（支持 ~ 用户主目录）",
    workingDirPlaceholder: "~",
    browse: "选择…",
    chooseFolderTitle: "选择 Agent 工作目录",
    shellTitle: "执行 Shell",
    shellDesc: "输入解释器路径或使用右侧快捷选项",
    shellPlaceholder: "例如 /bin/zsh 或 powershell.exe",
    shellAuto: "自动识别",
    pathTitle: "环境变量 PATH",
    pathDesc: "运行时环境变量 PATH（启动命令中 export 可对其进行覆盖）",
    pathPlaceholder: "留空自动探测系统环境 PATH",
    autoDetect: "一键检测",
    detectedPathSuccess: "已自动检测并填入当前系统环境 PATH（可继续在命令中自行覆盖）",
    detectedPathFailed: "PATH 检测失败",
    policyCardTitle: "重启策略与自启策略",
    policyCardDesc: "配置当前 Agent 意外退出时的重启机制与软件启动时的自动拉起规则",
    restartPolicyTitle: "进程重启策略",
    restartPolicyDesc: "控制该 Agent 退出或崩溃后是否自动拉起",
    restartNo: "不自动重启 (手动控制)",
    restartAlways: "始终自动重启 (退出后自动重新拉起)",
    restartOnFailure: "仅异常退出时重启 (退出码非 0 自动拉起)",
    restartDelayTitle: "重启延迟 (秒)",
    restartDelayDesc: "进程退出后重新启动的间隔秒数",
    secondsUnit: "秒",
    autoStartTitle: "软件启动后自动启动此 Agent",
    linesCount: (n: number) => `${n} 行`,
    dragResizeWidth: "拖拽调整列表宽度",
    dragResizeHeight: "拖拽调整日志面板高度",
    language: "语言切换",
    langAuto: "自动",
    langZh: "中文",
    langEn: "English",
  },
  "en-US": {
    appTitle: "AI Agent Launcher",
    runningCount: (n: number) => (n === 1 ? "1 Running" : `${n} Running`),
    allStopped: "All Stopped",
    cleanResiduals: "Clean",
    cleanResidualsTooltip: "Force kill orphan processes and release occupied ports after crashes",
    cleanedToast: "Residual processes & orphan ports cleared",
    agentList: "Agent List",
    newAgent: "New",
    duplicate: "Copy",
    delete: "Delete",
    copySuffix: "Copy",
    unnamedAgent: "Unnamed Agent",
    statusRunning: "Running",
    statusStopped: "Stopped",
    statusFailed: "Failed",
    btnStart: "Start",
    btnStop: "Stop",
    btnRestart: "Restart",
    btnClean: "Clean",
    btnClear: "Clear",
    autoScroll: "Auto Scroll",
    realtimeLogs: "Real-time Logs",
    emptyLog: "No logs yet. Click 'Start' above to launch process",
    agentName: "Agent Name",
    agentNamePlaceholder: "Agent Name",
    configChangedHint: "Modified (restarts to apply)",
    configSavedHint: "Config Saved",
    commandCardTitle: "Shell Startup Command",
    commandCardDesc: "Multi-line startup commands with Tab support. In-script export PATH overrides global environment",
    commandPlaceholder: "# Enter startup command or script, e.g.:\nexport PORT=8080\nexport PATH=\"/opt/homebrew/bin:$PATH\"\npython -m agent --port $PORT",
    runtimeCardTitle: "Runtime Environment",
    runtimeCardDesc: "Configure working directory, shell interpreter, and PATH variable",
    workingDirTitle: "Working Directory",
    workingDirDesc: "Process root execution directory (supports ~ home directory)",
    workingDirPlaceholder: "~",
    browse: "Browse…",
    chooseFolderTitle: "Select Agent Working Directory",
    shellTitle: "Execution Shell",
    shellDesc: "Enter interpreter path or select a preset chip on the right",
    shellPlaceholder: "e.g. /bin/zsh or powershell.exe",
    shellAuto: "Auto",
    pathTitle: "Environment Variable PATH",
    pathDesc: "Runtime PATH variable (can be overridden by export in script)",
    pathPlaceholder: "Leave blank to auto-detect system PATH",
    autoDetect: "Auto Detect",
    detectedPathSuccess: "Auto-detected and filled system PATH",
    detectedPathFailed: "Failed to detect system PATH",
    policyCardTitle: "Restart & Startup Policy",
    policyCardDesc: "Configure auto-recovery on crash and startup rules on app launch",
    restartPolicyTitle: "Process Restart Policy",
    restartPolicyDesc: "Controls automatic restart behavior when agent terminates or crashes",
    restartNo: "No Auto-Restart (Manual)",
    restartAlways: "Always Restart (Auto resurrect upon exit)",
    restartOnFailure: "Restart on Failure Only (Non-zero exit code)",
    restartDelayTitle: "Restart Delay (seconds)",
    restartDelayDesc: "Interval seconds before restarting after process exit",
    secondsUnit: "sec",
    autoStartTitle: "Auto-start this Agent when software launches",
    linesCount: (n: number) => `${n} lines`,
    dragResizeWidth: "Drag to resize sidebar width",
    dragResizeHeight: "Drag to resize log height",
    language: "Language",
    langAuto: "Auto",
    langZh: "中文",
    langEn: "English",
  },
};

export const t = computed(() => messages[currentLocale.value]);
