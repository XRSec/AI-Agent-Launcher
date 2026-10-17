<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import {
  enable as enableAutostart,
  disable as disableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import type {
  AgentProfile,
  ProfileStatus,
  ProfileLogMessage,
  RestartPolicy,
  ThemeSetting,
  UpdateInfo,
  DownloadProgressPayload,
} from "./types";
import { languageSetting, setLanguageSetting, t, currentLocale, type LanguageSetting } from "./i18n";
import { themeSetting, setThemeSetting } from "./composables/useTheme";
import { useClaudeScrollPinning } from "./composables/useClaudeScrollPinning";
import CdsSegmentedControl, { type SegmentedItem } from "./components/CdsSegmentedControl.vue";
import CdsStatusDot from "./components/CdsStatusDot.vue";
import ScrollFadeContainer from "./components/ScrollFadeContainer.vue";

const profiles = ref<AgentProfile[]>([]);
const selectedProfileId = ref<string>("");
const statuses = ref<Record<string, ProfileStatus>>({});
const logsMap = ref<Record<string, string[]>>({});
const autoScroll = ref(true);
const detectionMessage = ref("");
const cleanToast = ref("");
const logContainer = ref<HTMLDivElement | null>(null);

const langOptions = computed<SegmentedItem[]>(() => [
  { label: t.value.langAuto, value: "auto", title: "跟随系统 / Follow System" },
  { label: "中文", value: "zh-CN", title: "简体中文" },
  { label: "EN", value: "en-US", title: "English" },
]);

const themeOptions = computed<SegmentedItem[]>(() => [
  { label: t.value.themeAuto, value: "auto", title: t.value.themeAuto },
  { label: t.value.themeLight, value: "light", title: t.value.themeLight },
  { label: t.value.themeDark, value: "dark", title: t.value.themeDark },
]);

const restartPolicyOptions = computed<SegmentedItem[]>(() => [
  { label: t.value.restartNo, value: "no", title: t.value.restartNo },
  { label: t.value.restartAlways, value: "always", title: t.value.restartAlways },
  { label: t.value.restartOnFailure, value: "on-failure", title: t.value.restartOnFailure },
]);

// Log scroll pinning composable
const {
  containerRef: logScrollContainer,
  showJumpControl,
  markUserInput,
  handleScroll: onLogScroll,
  scrollToBottom: scrollToLogBottom,
  onNewItem: onNewLogItem,
} = useClaudeScrollPinning();

const appAutostart = ref(false);
const autostartLoading = ref(false);

const runningSnapshots = ref<Record<string, AgentProfile>>({});

// Updater states
const currentAppVersion = ref("1.0.0");
const updateInfo = ref<UpdateInfo | null>(null);
const isCheckingUpdate = ref(false);
const isUpdateModalOpen = ref(false);
const isAboutModalOpen = ref(false);
const isDownloadingUpdate = ref(false);
const downloadPercent = ref(0);
const downloadedStr = ref("");
const totalStr = ref("");
const downloadStatusText = ref("");
const downloadError = ref("");

// Drag resizer states
const sidebarWidth = ref<number>(
  Number(localStorage.getItem("launcher_sidebar_width")) || 200
);
const logHeight = ref<number>(
  Number(localStorage.getItem("launcher_log_height")) || 220
);
const isDraggingSidebar = ref(false);
const isDraggingLog = ref(false);
const isLogMaximized = ref(false);
const prevLogHeight = ref(220);

function toggleMaximizeLog() {
  if (!isLogMaximized.value) {
    prevLogHeight.value = logHeight.value;
    logHeight.value = Math.max(window.innerHeight - 120, 260);
    isLogMaximized.value = true;
  } else {
    logHeight.value = prevLogHeight.value || 220;
    isLogMaximized.value = false;
  }
  localStorage.setItem("launcher_log_height", String(logHeight.value));
}

function resetSidebarWidth() {
  sidebarWidth.value = 200;
  localStorage.setItem("launcher_sidebar_width", "200");
}

function startResizeSidebar(e: PointerEvent) {
  if (e.button !== 0) return;
  const targetEl = e.currentTarget as HTMLElement | null;
  if (targetEl && targetEl.setPointerCapture) {
    try {
      targetEl.setPointerCapture(e.pointerId);
    } catch (_) {}
  }

  const startX = e.clientX;
  const startW = sidebarWidth.value;
  let hasMovedPastDeadZone = false;

  function onPointerMove(moveEvent: PointerEvent) {
    const delta = moveEvent.clientX - startX;
    if (!hasMovedPastDeadZone) {
      if (Math.abs(delta) < 4) return;
      hasMovedPastDeadZone = true;
      isDraggingSidebar.value = true;
    }
    const newW = Math.min(Math.max(startW + delta, 52), 480);
    sidebarWidth.value = newW;
    localStorage.setItem("launcher_sidebar_width", String(newW));
  }

  function onPointerUp(upEvent: PointerEvent) {
    isDraggingSidebar.value = false;
    if (targetEl && targetEl.releasePointerCapture) {
      try {
        targetEl.releasePointerCapture(upEvent.pointerId);
      } catch (_) {}
    }
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointercancel", onPointerUp);
  }

  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerUp);
}

function startResizeLog(e: PointerEvent) {
  if (e.button !== 0) return;
  const targetEl = e.currentTarget as HTMLElement | null;
  if (targetEl && targetEl.setPointerCapture) {
    try {
      targetEl.setPointerCapture(e.pointerId);
    } catch (_) {}
  }

  const startY = e.clientY;
  const startH = logHeight.value;
  let hasMovedPastDeadZone = false;

  function onPointerMove(moveEvent: PointerEvent) {
    const delta = startY - moveEvent.clientY;
    if (!hasMovedPastDeadZone) {
      if (Math.abs(delta) < 4) return;
      hasMovedPastDeadZone = true;
      isDraggingLog.value = true;
    }
    const maxH = Math.max(window.innerHeight - 120, 260);
    const newH = Math.min(Math.max(startH + delta, 80), maxH);
    logHeight.value = newH;
    isLogMaximized.value = false;
    localStorage.setItem("launcher_log_height", String(newH));
  }

  function onPointerUp(upEvent: PointerEvent) {
    isDraggingLog.value = false;
    if (targetEl && targetEl.releasePointerCapture) {
      try {
        targetEl.releasePointerCapture(upEvent.pointerId);
      } catch (_) {}
    }
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointercancel", onPointerUp);
  }

  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerUp);
}

const currentProfile = computed(() => {
  return profiles.value.find((p) => p.id === selectedProfileId.value) || profiles.value[0];
});

const currentStatus = computed<ProfileStatus>(() => {
  if (!currentProfile.value) {
    return { profileId: "", isRunning: false, statusText: t.value.statusStopped, pid: null, lastError: null };
  }
  const raw = statuses.value[currentProfile.value.id];
  if (!raw) {
    return {
      profileId: currentProfile.value.id,
      isRunning: false,
      statusText: t.value.statusStopped,
      pid: null,
      lastError: null,
    };
  }
  return {
    ...raw,
    statusText: raw.isRunning
      ? t.value.statusRunning
      : raw.lastError
      ? t.value.statusFailed
      : t.value.statusStopped,
  };
});

const currentLogs = computed(() => {
  if (!currentProfile.value) return [];
  return logsMap.value[currentProfile.value.id] || [];
});

const runningCount = computed(() => {
  return Object.values(statuses.value).filter((s) => s.isRunning).length;
});

const hasPendingChanges = computed(() => {
  if (!currentProfile.value || !currentStatus.value.isRunning) {
    return false;
  }
  const snap = runningSnapshots.value[currentProfile.value.id];
  if (!snap) return false;
  return JSON.stringify(snap) !== JSON.stringify(currentProfile.value);
});

const logFormattedSize = computed(() => {
  const bytes = currentLogs.value.reduce((acc, l) => acc + l.length, 0);
  if (bytes > 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
});

// Shell filter presets (OS sensitive)
const isWindows = typeof navigator !== "undefined" && navigator.userAgent.toLowerCase().includes("win");

const shellPresets = computed(() => {
  if (isWindows) {
    return [
      { label: t.value.shellAuto, value: "auto" },
      { label: "PowerShell", value: "powershell.exe" },
      { label: "pwsh", value: "pwsh.exe" },
      { label: "cmd", value: "cmd.exe" },
      { label: "bash", value: "bash.exe" },
    ];
  }
  return [
    { label: t.value.shellAuto, value: "auto" },
    { label: "zsh", value: "/bin/zsh" },
    { label: "bash", value: "/bin/bash" },
    { label: "sh", value: "/bin/sh" },
    { label: "ash", value: "/bin/ash" },
  ];
});

function selectShellPreset(val: string) {
  if (!currentProfile.value) return;
  currentProfile.value.shell = val;
  persistProfiles();
}

async function loadData() {
  try {
    const list = await invoke<any[]>("get_profiles");
    profiles.value = list.map((p) => ({
      id: p.id || crypto.randomUUID(),
      name: p.name || t.value.unnamedAgent,
      command: p.command || p.script || "",
      shell: p.shell || "auto",
      workingDirectory: p.workingDirectory || "~",
      customPath: p.customPath || "",
      restartPolicy: (p.restartPolicy as RestartPolicy) || "no",
      restartDelay: p.restartDelay ?? 3,
      autoStart: !!p.autoStart,
    }));

    const allStatuses = await invoke<Record<string, ProfileStatus>>("get_all_statuses");
    statuses.value = allStatuses;

    const selId = await invoke<string | null>("get_selected_profile_id");
    if (selId && profiles.value.some((p) => p.id === selId)) {
      selectedProfileId.value = selId;
    } else if (profiles.value.length > 0) {
      selectedProfileId.value = profiles.value[0].id;
    }

    if (selectedProfileId.value) {
      await fetchLogsForProfile(selectedProfileId.value);
    }

    for (const p of profiles.value) {
      if (statuses.value[p.id]?.isRunning) {
        runningSnapshots.value[p.id] = JSON.parse(JSON.stringify(p));
      }
    }
  } catch (e) {
    console.error("Failed to load initial data:", e);
  }
}

async function fetchLogsForProfile(pid: string) {
  try {
    const lines = await invoke<string[]>("get_profile_logs", { profileId: pid });
    logsMap.value[pid] = lines;
    scrollToBottom();
  } catch (e) {
    console.error("Failed to fetch logs for profile", pid, e);
  }
}

async function persistProfiles() {
  try {
    await invoke("save_profiles", { profiles: profiles.value });
  } catch (e) {
    console.error("Failed to save profiles:", e);
  }
}

function selectProfile(id: string) {
  selectedProfileId.value = id;
  invoke("select_profile", { id }).catch(console.error);
  if (!logsMap.value[id]) {
    fetchLogsForProfile(id);
  } else {
    scrollToBottom();
  }
}

function addProfile() {
  const newName = `Agent ${profiles.value.length + 1}`;
  const newProfile: AgentProfile = {
    id: crypto.randomUUID(),
    name: newName,
    command: "# 在此输入专属于该 Agent 的启动命令或脚本\necho \"AI Agent 正在运行...\"\n",
    shell: "auto",
    workingDirectory: "~",
    customPath: "",
    restartPolicy: "no",
    restartDelay: 3,
    autoStart: false,
  };
  profiles.value.push(newProfile);
  selectProfile(newProfile.id);
  persistProfiles();
}

function duplicateProfile(id: string) {
  const target = profiles.value.find((p) => p.id === id);
  if (!target) return;
  const copy: AgentProfile = JSON.parse(JSON.stringify(target));
  copy.id = crypto.randomUUID();
  copy.name = `${target.name} (${t.value.copySuffix})`;
  const idx = profiles.value.findIndex((p) => p.id === id);
  profiles.value.splice(idx + 1, 0, copy);
  selectProfile(copy.id);
  persistProfiles();
}

async function deleteProfile(id: string) {
  if (profiles.value.length <= 1) return;
  if (statuses.value[id]?.isRunning) {
    await invoke("stop_agent_by_id", { profileId: id });
  }
  delete statuses.value[id];
  delete logsMap.value[id];
  delete runningSnapshots.value[id];

  const idx = profiles.value.findIndex((p) => p.id === id);
  profiles.value.splice(idx, 1);
  if (selectedProfileId.value === id) {
    selectedProfileId.value = profiles.value[Math.max(0, idx - 1)].id;
    fetchLogsForProfile(selectedProfileId.value);
  }
  persistProfiles();
}

async function chooseFolder() {
  if (!currentProfile.value) return;
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t.value.chooseFolderTitle,
    });
    if (selected && typeof selected === "string") {
      currentProfile.value.workingDirectory = selected;
      persistProfiles();
    }
  } catch (e) {
    console.error("Failed to open directory dialog:", e);
  }
}

async function detectPath() {
  if (!currentProfile.value) return;
  try {
    const detected = await invoke<string>("detect_path");
    currentProfile.value.customPath = detected;
    detectionMessage.value = t.value.detectedPathSuccess;
    persistProfiles();
  } catch (e) {
    detectionMessage.value = t.value.detectedPathFailed;
  }
}

async function startCurrentAgent() {
  if (!currentProfile.value) return;
  const pid = currentProfile.value.id;
  try {
    await persistProfiles();
    runningSnapshots.value[pid] = JSON.parse(JSON.stringify(currentProfile.value));
    await invoke("start_agent_by_id", { profileId: pid });
  } catch (e) {
    console.error("Start failed:", e);
  }
}

async function stopCurrentAgent() {
  if (!currentProfile.value) return;
  const pid = currentProfile.value.id;
  statuses.value[pid] = {
    profileId: pid,
    isRunning: false,
    statusText: t.value.statusStopped,
    pid: null,
    lastError: null,
  };
  delete runningSnapshots.value[pid];
  try {
    await invoke("stop_agent_by_id", { profileId: pid });
  } catch (e) {
    console.error("Stop failed:", e);
  }
}

async function restartCurrentAgent() {
  if (!currentProfile.value) return;
  const pid = currentProfile.value.id;
  try {
    await persistProfiles();
    runningSnapshots.value[pid] = JSON.parse(JSON.stringify(currentProfile.value));
    await invoke("restart_agent_by_id", { profileId: pid });
  } catch (e) {
    console.error("Restart failed:", e);
  }
}

async function cleanResiduals() {
  if (!currentProfile.value) return;
  const pid = currentProfile.value.id;
  statuses.value[pid] = {
    profileId: pid,
    isRunning: false,
    statusText: t.value.statusStopped,
    pid: null,
    lastError: null,
  };
  delete runningSnapshots.value[pid];
  try {
    const msg = await invoke<string>("clean_residual_processes", { profileId: pid });
    cleanToast.value = msg || t.value.cleanedToast;
    setTimeout(() => {
      cleanToast.value = "";
    }, 4000);
  } catch (e: any) {
    cleanToast.value = `Error: ${e}`;
    setTimeout(() => {
      cleanToast.value = "";
    }, 4000);
  }
}

async function clearCurrentLogs() {
  if (!currentProfile.value) return;
  const pid = currentProfile.value.id;
  logsMap.value[pid] = [];
  await invoke("clear_profile_logs", { profileId: pid });
}

function handleTabKey(e: KeyboardEvent) {
  if (e.key === "Tab") {
    e.preventDefault();
    const target = e.target as HTMLTextAreaElement;
    const start = target.selectionStart;
    const end = target.selectionEnd;
    const val = target.value;
    target.value = val.substring(0, start) + "  " + val.substring(end);
    target.selectionStart = target.selectionEnd = start + 2;
    if (currentProfile.value) {
      currentProfile.value.command = target.value;
      persistProfiles();
    }
  }
}

async function toggleAppAutostart() {
  if (autostartLoading.value) return;
  autostartLoading.value = true;
  try {
    if (appAutostart.value) {
      await disableAutostart();
      appAutostart.value = false;
      cleanToast.value = t.value.autostartDisabledToast;
    } else {
      await enableAutostart();
      appAutostart.value = true;
      cleanToast.value = t.value.autostartEnabledToast;
    }
    await emit("frontend-autostart-changed", appAutostart.value);
    setTimeout(() => {
      if (
        cleanToast.value === t.value.autostartEnabledToast ||
        cleanToast.value === t.value.autostartDisabledToast
      ) {
        cleanToast.value = "";
      }
    }, 3000);
  } catch (err: any) {
    cleanToast.value = `Autostart error: ${err}`;
    setTimeout(() => {
      cleanToast.value = "";
    }, 4000);
  } finally {
    autostartLoading.value = false;
  }
}

function scrollToBottom() {
  if (!autoScroll.value) return;
  nextTick(() => {
    scrollToLogBottom(false);
  });
}

function jumpToBottom() {
  autoScroll.value = true;
  nextTick(() => {
    scrollToLogBottom(true);
  });
}

watch(autoScroll, (val) => {
  if (val) {
    scrollToLogBottom(false);
  }
});

watch(logContainer, (el) => {
  logScrollContainer.value = el;
});

// Log Search States & Logic
const isLogSearchOpen = ref(false);
const logSearchQuery = ref("");
const logSearchCaseSensitive = ref(false);
const logSearchIndex = ref(0);
const logSearchInput = ref<HTMLInputElement | null>(null);

function escapeRegExp(string: string) {
  return string.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

interface LogMatchItem {
  lineIdx: number;
  start: number;
  end: number;
  globalIndex: number;
}

const searchMatches = computed<LogMatchItem[]>(() => {
  const query = logSearchQuery.value.trim();
  if (!isLogSearchOpen.value || !query) return [];

  const flags = logSearchCaseSensitive.value ? "g" : "gi";
  let regex: RegExp;
  try {
    regex = new RegExp(escapeRegExp(query), flags);
  } catch {
    return [];
  }

  const results: LogMatchItem[] = [];
  let count = 0;
  const lines = currentLogs.value;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    let match: RegExpExecArray | null;
    while ((match = regex.exec(line)) !== null) {
      results.push({
        lineIdx: i,
        start: match.index,
        end: match.index + match[0].length,
        globalIndex: count++,
      });
      if (regex.lastIndex === match.index) {
        regex.lastIndex++;
      }
    }
  }

  return results;
});

const lineMatchesMap = computed(() => {
  const map = new Map<number, LogMatchItem[]>();
  for (const item of searchMatches.value) {
    let list = map.get(item.lineIdx);
    if (!list) {
      list = [];
      map.set(item.lineIdx, list);
    }
    list.push(item);
  }
  return map;
});

interface LinePart {
  text: string;
  isMatch: boolean;
  isActive: boolean;
}

function getLineParts(line: string, lineIdx: number): LinePart[] {
  if (!isLogSearchOpen.value || !logSearchQuery.value.trim()) {
    return [{ text: line, isMatch: false, isActive: false }];
  }

  const matches = lineMatchesMap.value.get(lineIdx);
  if (!matches || matches.length === 0) {
    return [{ text: line, isMatch: false, isActive: false }];
  }

  const parts: LinePart[] = [];
  let lastIdx = 0;
  const currentActive = logSearchIndex.value;

  for (const m of matches) {
    if (m.start > lastIdx) {
      parts.push({
        text: line.slice(lastIdx, m.start),
        isMatch: false,
        isActive: false,
      });
    }
    parts.push({
      text: line.slice(m.start, m.end),
      isMatch: true,
      isActive: m.globalIndex === currentActive,
    });
    lastIdx = m.end;
  }

  if (lastIdx < line.length) {
    parts.push({
      text: line.slice(lastIdx),
      isMatch: false,
      isActive: false,
    });
  }

  return parts;
}

watch(logSearchQuery, () => {
  logSearchIndex.value = 0;
  if (searchMatches.value.length > 0) {
    scrollToCurrentMatch();
  }
});

watch(logSearchCaseSensitive, () => {
  logSearchIndex.value = 0;
  if (searchMatches.value.length > 0) {
    scrollToCurrentMatch();
  }
});

function scrollToCurrentMatch() {
  if (searchMatches.value.length === 0) return;
  const match = searchMatches.value[logSearchIndex.value];
  if (!match || !logContainer.value) return;

  nextTick(() => {
    const el = logContainer.value?.querySelector(`[data-line-idx="${match.lineIdx}"]`) as HTMLElement | null;
    if (el) {
      el.scrollIntoView({ block: "center", behavior: "smooth" });
    }
  });
}

function nextLogSearchMatch() {
  if (searchMatches.value.length === 0) return;
  logSearchIndex.value = (logSearchIndex.value + 1) % searchMatches.value.length;
  scrollToCurrentMatch();
}

function prevLogSearchMatch() {
  if (searchMatches.value.length === 0) return;
  logSearchIndex.value =
    (logSearchIndex.value - 1 + searchMatches.value.length) % searchMatches.value.length;
  scrollToCurrentMatch();
}

function openLogSearch() {
  isLogSearchOpen.value = true;
  nextTick(() => {
    logSearchInput.value?.focus();
    logSearchInput.value?.select();
    if (searchMatches.value.length > 0) {
      scrollToCurrentMatch();
    }
  });
}

function closeLogSearch() {
  isLogSearchOpen.value = false;
}

function formatBytes(bytes?: number | null) {
  if (!bytes || bytes <= 0) return "";
  const mb = (bytes / (1024 * 1024)).toFixed(1);
  return `${mb} MB`;
}

function formatDate(dateStr?: string) {
  if (!dateStr) return "";
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString(currentLocale.value === "zh-CN" ? "zh-CN" : "en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

async function checkUpdate(silent = true) {
  if (isCheckingUpdate.value) return;
  isCheckingUpdate.value = true;
  try {
    const res = await invoke<UpdateInfo>("check_app_update");
    updateInfo.value = res;
    if (res.current_version) {
      currentAppVersion.value = res.current_version;
    }
    if (res.has_update) {
      isUpdateModalOpen.value = true;
    } else if (!silent) {
      cleanToast.value = t.value.alreadyLatest;
      setTimeout(() => {
        if (cleanToast.value === t.value.alreadyLatest) cleanToast.value = "";
      }, 3000);
    }
  } catch (err: any) {
    if (!silent) {
      cleanToast.value = `${t.value.updateCheckFailed}: ${err}`;
      setTimeout(() => {
        cleanToast.value = "";
      }, 4000);
    }
  } finally {
    isCheckingUpdate.value = false;
  }
}

async function startDownloadUpdate() {
  if (!updateInfo.value?.asset_download_url || isDownloadingUpdate.value) return;
  isDownloadingUpdate.value = true;
  downloadPercent.value = 0;
  downloadError.value = "";
  downloadStatusText.value = t.value.downloadingUpdate;

  try {
    await invoke("download_and_install_update", {
      downloadUrl: updateInfo.value.asset_download_url,
      fileName: updateInfo.value.asset_name || "ai-agent-launcher-update",
    });
    downloadStatusText.value = t.value.downloadCompleteLaunching;
  } catch (err: any) {
    isDownloadingUpdate.value = false;
    downloadError.value = `${t.value.downloadFailed}: ${err}`;
  }
}

async function openBrowserDownload() {
  if (updateInfo.value?.asset_download_url) {
    await openUrl(updateInfo.value.asset_download_url);
  } else if (updateInfo.value?.release_url) {
    await openUrl(updateInfo.value.release_url);
  }
}

onMounted(async () => {
  try {
    currentAppVersion.value = await getVersion();
  } catch (e) {
    console.warn("Failed to get app version:", e);
  }
  await loadData();

  window.addEventListener("keydown", async (e: KeyboardEvent) => {
    if (
      ((e.metaKey || e.ctrlKey) && (e.key === "q" || e.key === "Q")) ||
      (e.altKey && e.key === "F4")
    ) {
      e.preventDefault();
      await invoke("exit_app");
      return;
    }

    // Cmd+F or Ctrl+F: Open Log Search
    if ((e.metaKey || e.ctrlKey) && (e.key === "f" || e.key === "F")) {
      e.preventDefault();
      openLogSearch();
      return;
    }

    // F3 / Shift+F3 for find next/prev
    if (e.key === "F3") {
      e.preventDefault();
      if (!isLogSearchOpen.value) {
        openLogSearch();
      } else if (e.shiftKey) {
        prevLogSearchMatch();
      } else {
        nextLogSearchMatch();
      }
      return;
    }

    // Escape to close search
    if (e.key === "Escape" && isLogSearchOpen.value) {
      e.preventDefault();
      closeLogSearch();
      return;
    }
  });

  try {
    appAutostart.value = await isAutostartEnabled();
  } catch (e) {
    console.warn("Failed to check autostart status:", e);
  }

  await listen<boolean>("autostart-changed", (event) => {
    appAutostart.value = event.payload;
    cleanToast.value = event.payload
      ? t.value.autostartEnabledToast
      : t.value.autostartDisabledToast;
    setTimeout(() => {
      if (
        cleanToast.value === t.value.autostartEnabledToast ||
        cleanToast.value === t.value.autostartDisabledToast
      ) {
        cleanToast.value = "";
      }
    }, 3000);
  });

  logScrollContainer.value = logContainer.value;

  await listen<ProfileLogMessage>("profile-log-output", (event) => {
    const { profileId, text } = event.payload;
    if (!logsMap.value[profileId]) {
      logsMap.value[profileId] = [];
    }
    logsMap.value[profileId].push(text);
    if (logsMap.value[profileId].length > 6000) {
      logsMap.value[profileId].splice(0, 1000);
    }
    if (profileId === currentProfile.value?.id) {
      if (autoScroll.value) {
        nextTick(() => {
          onNewLogItem();
        });
      }
    }
  });

  await listen<ProfileStatus>("profile-status-changed", (event) => {
    const st = event.payload;
    statuses.value[st.profileId] = st;
    if (!st.isRunning) {
      delete runningSnapshots.value[st.profileId];
    } else {
      const p = profiles.value.find((item) => item.id === st.profileId);
      if (p && !runningSnapshots.value[st.profileId]) {
        runningSnapshots.value[st.profileId] = JSON.parse(JSON.stringify(p));
      }
    }
  });

  await listen<string>("change-language", (event) => {
    if (event.payload === "auto" || event.payload === "zh-CN" || event.payload === "en-US") {
      setLanguageSetting(event.payload as LanguageSetting);
    }
  });

  // Listen for native app menu "Check for updates..."
  await listen("menu-check-update", () => {
    checkUpdate(false);
  });

  // Listen for native app menu "About AI Agent Launcher"
  await listen("menu-open-about", () => {
    isAboutModalOpen.value = true;
  });

  // Listen for update download progress
  await listen<DownloadProgressPayload>("update-download-progress", (event) => {
    const { downloaded, total, percent } = event.payload;
    downloadPercent.value = percent;
    const curMB = (downloaded / (1024 * 1024)).toFixed(1) + " MB";
    const totMB = total > 0 ? (total / (1024 * 1024)).toFixed(1) + " MB" : "--";
    downloadedStr.value = curMB;
    totalStr.value = totMB;
    downloadStatusText.value = t.value.downloadProgress(percent, curMB, totMB);
  });

  // Silent update check on startup (after 2.5s)
  setTimeout(() => {
    checkUpdate(true);
  }, 2500);
});
</script>

<template>
  <div class="app-layout" :class="{ 'is-resizing': isDraggingSidebar || isDraggingLog }">
    <!-- Top Header (Layer 0 Surface) -->
    <header class="app-header">
      <div class="header-left">
        <div class="brand-box is-clickable" :title="t.aboutTitle" @click="isAboutModalOpen = true">
          <img src="/app-icon.svg" alt="App Logo" class="brand-logo-img" />
        </div>
        <div class="brand-text">
          <h1 class="app-title is-clickable" :title="t.aboutTitle" @click="isAboutModalOpen = true">{{ t.appTitle }}</h1>
          <span class="app-version is-clickable" :title="t.aboutTitle" @click="isAboutModalOpen = true">v{{ currentAppVersion }}</span>
          <!-- Update Available Badge -->
          <button
            v-if="updateInfo && updateInfo.has_update"
            type="button"
            class="update-badge-pill"
            :title="t.updateAvailable"
            @click="isUpdateModalOpen = true"
          >
            <span class="update-ping-circle"></span>
            <span class="update-emoji">🚀</span>
            <span class="update-badge-text">{{ t.updateAvailableBadge(updateInfo.latest_version) }}</span>
          </button>
        </div>
      </div>

      <div class="header-right">
        <!-- Toast feedback for residual cleanup and autostart -->
        <div v-if="cleanToast" class="clean-toast">
          {{ cleanToast }}
        </div>

        <!-- About Button -->
        <button
          type="button"
          class="header-icon-btn"
          :title="t.aboutTitle"
          @click="isAboutModalOpen = true"
        >
          <svg class="header-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
        </button>

        <!-- Manual Check for Updates Button -->
        <button
          type="button"
          class="header-icon-btn"
          :class="{ 'is-spinning': isCheckingUpdate }"
          :title="t.checkUpdate"
          @click="() => checkUpdate(false)"
        >
          <svg class="header-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>

        <!-- App Autostart Toggle -->
        <button
          type="button"
          class="autostart-pill"
          :class="{ active: appAutostart, loading: autostartLoading }"
          :title="t.autostartAppTooltip"
          @click="toggleAppAutostart"
        >
          <span class="autostart-dot"></span>
          <span>{{ t.autostartApp }}</span>
        </button>

        <div class="running-count-pill" :class="{ active: runningCount > 0 }">
          <CdsStatusDot :status="runningCount > 0 ? 'running' : 'stopped'" :pulse="runningCount > 0" />
          <span>{{ runningCount > 0 ? t.runningCount(runningCount) : t.allStopped }}</span>
        </div>

        <!-- Theme Switcher (Auto, Light, Dark) -->
        <div class="header-control-group" :title="t.theme">
          <CdsSegmentedControl
            :model-value="themeSetting"
            :items="themeOptions"
            size="sm"
            @update:model-value="(val) => setThemeSetting(val as ThemeSetting)"
          />
        </div>

        <!-- Language Switcher (Auto, 中文, EN) -->
        <div class="header-control-group" :title="t.language">
          <CdsSegmentedControl
            :model-value="languageSetting"
            :items="langOptions"
            size="sm"
            @update:model-value="(val) => setLanguageSetting(val as LanguageSetting)"
          />
        </div>
      </div>
    </header>

    <!-- Main Body with Resizable Splitters -->
    <div class="main-body">
      <!-- Left Sidebar: Profile List (Layer 1 Surface) -->
      <aside
        class="sidebar"
        :class="{
          compact: sidebarWidth < 170,
          'ultra-compact': sidebarWidth < 90
        }"
        :style="{ width: `${sidebarWidth}px` }"
      >
        <div class="sidebar-header">
          <span v-if="sidebarWidth >= 100" class="sidebar-label">{{ t.agentList }}</span>
          <button class="add-icon-btn" :title="t.newAgent" @click="addProfile">
            <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" stroke-width="2.5" fill="none">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
        </div>

        <div class="profile-list">
          <div
            v-for="profile in profiles"
            :key="profile.id"
            class="profile-item"
            :class="{
              selected: profile.id === selectedProfileId,
              running: statuses[profile.id]?.isRunning,
            }"
            :title="profile.name || t.unnamedAgent"
            @click="selectProfile(profile.id)"
          >
            <div class="profile-info">
              <div class="profile-avatar">
                <CdsStatusDot
                  :status="statuses[profile.id]?.isRunning ? 'running' : 'stopped'"
                  :pulse="statuses[profile.id]?.isRunning"
                />
              </div>
              <div v-if="sidebarWidth >= 90" class="profile-text-box">
                <span class="profile-name" :title="profile.name">{{ profile.name || t.unnamedAgent }}</span>
              </div>
            </div>

            <div v-if="sidebarWidth >= 170" class="profile-status-badge">
              <span
                v-if="statuses[profile.id]?.isRunning"
                class="status-tag running"
                :title="statuses[profile.id]?.pid ? `PID: ${statuses[profile.id]?.pid}` : ''"
              >
                {{ t.statusRunning }}
              </span>
              <span v-else class="status-tag stopped">{{ t.statusStopped }}</span>
            </div>
          </div>
        </div>

        <div class="sidebar-footer">
          <button class="footer-btn primary-add cds-press-feedback" @click="addProfile">+ {{ t.newAgent }}</button>
          <button
            v-if="sidebarWidth >= 130"
            class="footer-btn cds-press-feedback"
            :disabled="!currentProfile"
            @click="currentProfile && duplicateProfile(currentProfile.id)"
          >
            {{ t.duplicate }}
          </button>
          <button
            v-if="sidebarWidth >= 130"
            class="footer-btn danger cds-press-feedback"
            :disabled="profiles.length <= 1 || !currentProfile"
            @click="currentProfile && deleteProfile(currentProfile.id)"
          >
            {{ t.delete }}
          </button>
        </div>
      </aside>

      <!-- Vertical Resizer between Sidebar & Content Panel -->
      <div
        class="resizer-v"
        :title="t.dragResizeWidth"
        @pointerdown="startResizeSidebar"
        @dblclick="resetSidebarWidth"
      >
        <div class="resizer-thumb-v"></div>
      </div>

      <!-- Right Content: Profile Details + Terminal Log -->
      <main class="content-panel" v-if="currentProfile">
        <!-- Profile Header with Name, Status & 2-char Action Buttons (ONLY HERE) -->
        <div class="detail-header">
          <div class="detail-title-row">
            <input
              v-model="currentProfile.name"
              class="agent-name-input"
              :placeholder="t.agentNamePlaceholder"
              @input="persistProfiles"
            />
            <div class="profile-state-pill" :class="{ running: currentStatus.isRunning, failed: !!currentStatus.lastError }">
              <CdsStatusDot
                :status="currentStatus.isRunning ? 'running' : currentStatus.lastError ? 'failed' : 'stopped'"
                :pulse="currentStatus.isRunning"
              />
              <span>
                {{ currentStatus.statusText }}
                <template v-if="currentStatus.pid">(PID: {{ currentStatus.pid }})</template>
              </span>
            </div>

            <span v-if="hasPendingChanges" class="changes-hint">
              <span class="hint-dot"></span>{{ t.configChangedHint }}
            </span>
            <span v-else class="saved-hint">{{ t.configSavedHint }}</span>
          </div>

          <!-- Quick Action Buttons: Exactly 2 characters (启动 / 停止 / 重启) + 清理残留 -->
          <div class="detail-actions">
            <!-- Residual process cleaner button (solves app crash leftover processes/port in use) -->
            <button
              class="action-btn-clean cds-press-feedback"
              :title="t.cleanResidualsTooltip"
              @click="cleanResiduals"
            >
              <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2" fill="none">
                <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
              </svg>
              <span>{{ t.btnClean }}</span>
            </button>

            <button
              v-if="currentStatus.isRunning"
              class="action-btn-danger cds-press-feedback"
              @click="stopCurrentAgent"
            >
              {{ t.btnStop }}
            </button>
            <button
              v-if="currentStatus.isRunning"
              class="action-btn-secondary cds-press-feedback"
              @click="restartCurrentAgent"
            >
              {{ t.btnRestart }}
            </button>
            <button
              v-else
              class="action-btn-primary cds-press-feedback"
              @click="startCurrentAgent"
            >
              {{ t.btnStart }}
            </button>
          </div>
        </div>

        <!-- Upper Config Scroll Area with 7-stop Fade (Layer 2 Surface: Floating Cards with Depth) -->
        <ScrollFadeContainer orientation="vertical" :fade-size="20" class="config-scroll-wrapper">
          <div class="scroll-content">
            <!-- Error banner for this specific profile -->
            <div v-if="currentStatus.lastError" class="error-banner">
            <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2" fill="none">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <span>{{ currentStatus.lastError }}</span>
          </div>

          <!-- Card 1: Shell 启动命令 (Rich Contrast Dark Code Box) -->
          <div class="card">
            <div class="card-header">
              <div class="card-title-box">
                <div class="card-icon command">
                  <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2.5" fill="none">
                    <polyline points="4 17 10 11 4 5" />
                    <line x1="12" y1="19" x2="20" y2="19" />
                  </svg>
                </div>
                <div>
                  <h3 class="card-title">{{ t.commandCardTitle }}</h3>
                  <p class="card-desc">{{ t.commandCardDesc }}</p>
                </div>
              </div>
            </div>
            <div class="code-editor-box">
              <div class="editor-header">
                <span class="editor-dot red"></span>
                <span class="editor-dot yellow"></span>
                <span class="editor-dot green"></span>
                <span class="editor-lang-tag">Shell / Script</span>
              </div>
              <textarea
                v-model="currentProfile.command"
                class="code-textarea"
                :placeholder="t.commandPlaceholder"
                spellcheck="false"
                @keydown="handleTabKey"
                @input="persistProfiles"
              ></textarea>
            </div>
          </div>

          <!-- Card 2: 运行时环境 -->
          <div class="card">
            <div class="card-header">
              <div class="card-title-box">
                <div class="card-icon runtime">
                  <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2" fill="none">
                    <rect x="2" y="4" width="20" height="16" rx="2" />
                    <path d="M7 15h10M7 9l4 3-4 3" />
                  </svg>
                </div>
                <div>
                  <h3 class="card-title">{{ t.runtimeCardTitle }}</h3>
                  <p class="card-desc">{{ t.runtimeCardDesc }}</p>
                </div>
              </div>
            </div>

            <div class="form-grid">
              <!-- Working Directory -->
              <div class="form-row">
                <label class="form-label">
                  <span class="label-title">{{ t.workingDirTitle }}</span>
                  <span class="label-desc">{{ t.workingDirDesc }}</span>
                </label>
                <div class="form-input-group">
                  <input
                    v-model="currentProfile.workingDirectory"
                    class="text-input"
                    :placeholder="t.workingDirPlaceholder"
                    @input="persistProfiles"
                  />
                  <button class="small-btn" @click="chooseFolder">{{ t.browse }}</button>
                </div>
              </div>

              <!-- Execution Shell (Left Input + Right Presets Filter Chips) -->
              <div class="form-row">
                <label class="form-label">
                  <span class="label-title">{{ t.shellTitle }}</span>
                  <span class="label-desc">{{ t.shellDesc }}</span>
                </label>
                <div class="form-input-group shell-input-group">
                  <input
                    v-model="currentProfile.shell"
                    class="text-input mono shell-text-field"
                    :placeholder="t.shellPlaceholder"
                    @input="persistProfiles"
                  />
                  <div class="shell-chips">
                    <button
                      v-for="chip in shellPresets"
                      :key="chip.value"
                      type="button"
                      class="chip-btn"
                      :class="{
                        active: currentProfile.shell === chip.value || (chip.value === 'auto' && (!currentProfile.shell || currentProfile.shell === 'auto'))
                      }"
                      @click="selectShellPreset(chip.value)"
                    >
                      {{ chip.label }}
                    </button>
                  </div>
                </div>
              </div>

              <!-- PATH -->
              <div class="form-row">
                <label class="form-label">
                  <span class="label-title">{{ t.pathTitle }}</span>
                  <span class="label-desc">{{ t.pathDesc }}</span>
                </label>
                <div class="form-input-group">
                  <input
                    v-model="currentProfile.customPath"
                    class="text-input mono"
                    :placeholder="t.pathPlaceholder"
                    @input="persistProfiles"
                  />
                  <button class="small-btn" @click="detectPath">{{ t.autoDetect }}</button>
                </div>
              </div>

              <div v-if="detectionMessage" class="hint-msg">{{ detectionMessage }}</div>
            </div>
          </div>

          <!-- Card 3: 运行与自启策略 -->
          <div class="card">
            <div class="card-header">
              <div class="card-title-box">
                <div class="card-icon policy">
                  <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2" fill="none">
                    <polyline points="23 4 23 10 17 10" />
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
                  </svg>
                </div>
                <div>
                  <h3 class="card-title">{{ t.policyCardTitle }}</h3>
                  <p class="card-desc">{{ t.policyCardDesc }}</p>
                </div>
              </div>
            </div>

            <div class="form-grid">
              <!-- Restart Policy -->
              <div class="form-row">
                <label class="form-label">
                  <span class="label-title">{{ t.restartPolicyTitle }}</span>
                  <span class="label-desc">{{ t.restartPolicyDesc }}</span>
                </label>
                <div class="form-input-group">
                  <CdsSegmentedControl
                    :model-value="currentProfile.restartPolicy"
                    :items="restartPolicyOptions"
                    size="md"
                    @update:model-value="(val) => { currentProfile.restartPolicy = val as RestartPolicy; persistProfiles(); }"
                  />
                </div>
              </div>

              <!-- Restart Delay -->
              <div v-if="currentProfile.restartPolicy !== 'no'" class="form-row">
                <label class="form-label">
                  <span class="label-title">{{ t.restartDelayTitle }}</span>
                  <span class="label-desc">{{ t.restartDelayDesc }}</span>
                </label>
                <div class="form-input-group">
                  <input
                    type="number"
                    min="1"
                    max="60"
                    v-model.number="currentProfile.restartDelay"
                    class="text-input"
                    @input="persistProfiles"
                  />
                  <span class="input-unit">{{ t.secondsUnit }}</span>
                </div>
              </div>

              <!-- Auto Start Policy -->
              <div class="toggle-list">
                <label class="toggle-item">
                  <input
                    type="checkbox"
                    v-model="currentProfile.autoStart"
                    @change="persistProfiles"
                  />
                  <span class="toggle-text">{{ t.autoStartTitle }}</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      </ScrollFadeContainer>

        <!-- Horizontal Resizer between Config & Terminal Log -->
        <div
          class="resizer-h"
          :title="t.dragResizeHeight"
          @pointerdown="startResizeLog"
          @dblclick="toggleMaximizeLog"
        >
          <div class="resizer-thumb-h"></div>
        </div>

        <!-- Per-Profile Realtime Terminal Logs (Layer 3: High-contrast Dark Console) -->
        <section class="log-section" :style="{ height: `${logHeight}px` }">
          <div class="log-header">
            <div class="log-title-box">
              <span class="log-terminal-icon">
                <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2" fill="none">
                  <polyline points="4 17 10 11 4 5" />
                  <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
              </span>
              <span class="log-title">{{ t.realtimeLogs }}</span>
              <span class="log-stats">{{ t.linesCount(currentLogs.length) }} · {{ logFormattedSize }}</span>
            </div>

            <div class="log-controls">
              <!-- Search Toggle Button -->
              <button
                type="button"
                class="log-btn"
                :class="{ active: isLogSearchOpen }"
                :title="t.searchLogsTooltip"
                @click="isLogSearchOpen ? closeLogSearch() : openLogSearch()"
              >
                <svg viewBox="0 0 24 24" width="11" height="11" stroke="currentColor" stroke-width="2.2" fill="none">
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
                <span>{{ t.searchLogs }}</span>
              </button>

              <label class="auto-scroll-label">
                <input type="checkbox" v-model="autoScroll" />
                <span>{{ t.autoScroll }}</span>
              </label>

              <!-- Maximize / Restore Toggle Button -->
              <button
                type="button"
                class="log-btn icon-only"
                :title="isLogMaximized ? t.restoreLog : t.maximizeLog"
                @click="toggleMaximizeLog"
              >
                <svg v-if="!isLogMaximized" viewBox="0 0 24 24" width="11" height="11" stroke="currentColor" stroke-width="2.2" fill="none">
                  <polyline points="15 3 21 3 21 9" />
                  <polyline points="9 21 3 21 3 15" />
                  <line x1="21" y1="3" x2="14" y2="10" />
                  <line x1="3" y1="21" x2="10" y2="14" />
                </svg>
                <svg v-else viewBox="0 0 24 24" width="11" height="11" stroke="currentColor" stroke-width="2.2" fill="none">
                  <polyline points="4 14 10 14 10 20" />
                  <polyline points="20 10 14 10 14 4" />
                  <line x1="14" y1="10" x2="21" y2="3" />
                  <line x1="3" y1="21" x2="10" y2="14" />
                </svg>
              </button>

              <button class="clear-btn" @click="clearCurrentLogs">{{ t.btnClear }}</button>
            </div>
          </div>

          <!-- Floating Search Box inside Log Section -->
          <div v-if="isLogSearchOpen" class="log-search-panel">
            <div class="search-input-box">
              <svg class="search-prefix-icon" viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.2" fill="none">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                ref="logSearchInput"
                v-model="logSearchQuery"
                type="text"
                class="search-inner-input"
                :placeholder="t.searchPlaceholder"
                @keydown.enter.prevent="nextLogSearchMatch"
                @keydown.shift.enter.prevent.stop="prevLogSearchMatch"
                @keydown.esc.prevent="closeLogSearch"
              />
            </div>

            <div class="search-actions">
              <!-- Match Count -->
              <span
                class="search-match-badge"
                :class="{
                  'has-matches': searchMatches.length > 0,
                  'no-matches': logSearchQuery.trim() && searchMatches.length === 0
                }"
              >
                {{
                  !logSearchQuery.trim()
                    ? '0 / 0'
                    : searchMatches.length === 0
                    ? t.noMatches
                    : t.matchCount(logSearchIndex + 1, searchMatches.length)
                }}
              </span>

              <!-- Case Sensitive -->
              <button
                type="button"
                class="search-sub-btn"
                :class="{ active: logSearchCaseSensitive }"
                :title="t.caseSensitive"
                @click="logSearchCaseSensitive = !logSearchCaseSensitive"
              >
                Aa
              </button>

              <!-- Prev -->
              <button
                type="button"
                class="search-sub-btn"
                :title="t.prevMatch"
                :disabled="searchMatches.length === 0"
                @click="prevLogSearchMatch"
              >
                <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none">
                  <polyline points="18 15 12 9 6 15" />
                </svg>
              </button>

              <!-- Next -->
              <button
                type="button"
                class="search-sub-btn"
                :title="t.nextMatch"
                :disabled="searchMatches.length === 0"
                @click="nextLogSearchMatch"
              >
                <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </button>

              <!-- Close -->
              <button
                type="button"
                class="search-sub-btn close"
                :title="t.closeSearch"
                @click="closeLogSearch"
              >
                <svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" stroke-width="2.5" fill="none">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          </div>

          <ScrollFadeContainer
            orientation="vertical"
            :fade-size="20"
            class="log-fade-wrap"
          >
            <div
              ref="logContainer"
              class="log-body"
              @scroll="onLogScroll"
              @wheel="markUserInput"
              @touchstart="markUserInput"
            >
              <div
                v-for="(line, idx) in currentLogs"
                :key="idx"
                :data-line-idx="idx"
                class="log-line"
                :class="{
                  'is-error': line.toLowerCase().includes('error') || line.toLowerCase().includes('failed'),
                  'is-info': line.startsWith('[服务]') || line.startsWith('[系统]') || line.startsWith('[重启策略]')
                }"
              >
                <template v-if="!isLogSearchOpen || !logSearchQuery.trim()">
                  {{ line }}
                </template>
                <template v-else>
                  <span
                    v-for="(part, pIdx) in getLineParts(line, idx)"
                    :key="pIdx"
                    :class="{
                      'search-match': part.isMatch,
                      'search-match-active': part.isActive
                    }"
                  >{{ part.text }}</span>
                </template>
              </div>
              <div v-if="currentLogs.length === 0" class="log-empty">
                {{ t.emptyLog }}
              </div>
              <!-- Spacer to ensure bottom log line has full breathing room and is never cut in half -->
              <div class="log-bottom-spacer"></div>
            </div>
          </ScrollFadeContainer>

          <!-- Floating Jump to Bottom Button when user scrolled up -->
          <Transition name="fade-slide">
            <button
              v-if="showJumpControl"
              type="button"
              class="jump-to-bottom-pill cds-press-feedback"
              :title="t.jumpToBottom"
              @click="jumpToBottom"
            >
              <svg viewBox="0 0 24 24" width="10" height="10" stroke="currentColor" stroke-width="2.5" fill="none">
                <polyline points="6 9 12 15 18 9" />
              </svg>
              <span>{{ t.jumpToBottom }}</span>
            </button>
          </Transition>
        </section>
      </main>
    </div>

    <!-- Version Update Modal Dialog -->
    <Transition name="update-modal">
      <div
        v-if="isUpdateModalOpen && updateInfo"
        class="update-modal-backdrop"
        @click.self="!isDownloadingUpdate && (isUpdateModalOpen = false)"
      >
        <div class="update-modal-card">
          <!-- Modal Header -->
          <div class="update-modal-header">
            <div class="update-header-info">
              <span class="update-rocket-badge">🚀</span>
              <div>
                <h3 class="update-modal-title">{{ t.newVersionTitle }}</h3>
                <p class="update-modal-subtitle">
                  {{ updateInfo.release_name || `AI Agent Launcher v${updateInfo.latest_version}` }}
                </p>
              </div>
            </div>
            <button
              v-if="!isDownloadingUpdate"
              type="button"
              class="update-modal-close"
              :title="t.closeSearch"
              @click="isUpdateModalOpen = false"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2.2" fill="none">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <!-- Modal Body -->
          <div class="update-modal-body">
            <!-- Version Comparison Banner -->
            <div class="update-version-row">
              <div class="version-chip current">
                <span class="chip-label">{{ t.currentVersionLabel }}</span>
                <span class="chip-val">v{{ updateInfo.current_version }}</span>
              </div>
              <div class="version-arrow">➜</div>
              <div class="version-chip latest">
                <span class="chip-label">{{ t.latestVersionLabel }}</span>
                <span class="chip-val">v{{ updateInfo.latest_version }}</span>
              </div>
              <div v-if="updateInfo.published_at" class="update-date-badge">
                {{ formatDate(updateInfo.published_at) }}
              </div>
            </div>

            <!-- Matched Asset Tag -->
            <div v-if="updateInfo.asset_name" class="update-asset-pill">
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" stroke-width="2" fill="none">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
                <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
                <line x1="12" y1="22.08" x2="12" y2="12"/>
              </svg>
              <span class="asset-name">{{ updateInfo.asset_name }}</span>
              <span v-if="updateInfo.asset_size" class="asset-size">{{ formatBytes(updateInfo.asset_size) }}</span>
            </div>

            <!-- Building Notice if asset not yet published on GitHub -->
            <div v-else class="update-building-notice">
              <span class="building-icon">⏳</span>
              <span>{{ t.installerBuildingHint }}</span>
            </div>

            <!-- Release Notes Card -->
            <div class="update-notes-card">
              <div class="notes-header">{{ t.releaseNotesLabel }}</div>
              <div class="notes-content">
                {{ updateInfo.release_notes || "暂无特别说明 / No release notes provided." }}
              </div>
            </div>

            <!-- Download Progress Display -->
            <div v-if="isDownloadingUpdate" class="update-download-box">
              <div class="progress-bar-track">
                <div class="progress-bar-fill" :style="{ width: `${downloadPercent}%` }"></div>
              </div>
              <div class="progress-info-row">
                <span class="progress-text">{{ downloadStatusText }}</span>
                <span class="progress-percent">{{ downloadPercent }}%</span>
              </div>
            </div>

            <!-- Error Banner if any -->
            <div v-if="downloadError" class="update-error-banner">
              {{ downloadError }}
            </div>
          </div>

          <!-- Modal Footer Actions -->
          <div class="update-modal-footer">
            <button
              type="button"
              class="update-action-btn secondary browser"
              @click="openBrowserDownload"
            >
              <svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" stroke-width="2" fill="none">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
                <polyline points="15 3 21 3 21 9"/>
                <line x1="10" y1="14" x2="21" y2="3"/>
              </svg>
              <span>{{ t.btnDownloadBrowser }}</span>
            </button>

            <div class="footer-right-actions">
              <button
                v-if="!isDownloadingUpdate"
                type="button"
                class="update-action-btn ghost"
                @click="isUpdateModalOpen = false"
              >
                {{ t.btnRemindLater }}
              </button>

              <button
                type="button"
                class="update-action-btn primary"
                :disabled="isDownloadingUpdate"
                @click="updateInfo.asset_download_url ? startDownloadUpdate() : openBrowserDownload()"
              >
                <span v-if="isDownloadingUpdate" class="btn-spinner"></span>
                <svg v-else-if="updateInfo.asset_download_url" viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2.2" fill="none">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                  <polyline points="7 10 12 15 17 10" />
                  <line x1="12" y1="15" x2="12" y2="3" />
                </svg>
                <svg v-else viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2.2" fill="none">
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>
                <span>
                  {{
                    isDownloadingUpdate
                      ? t.downloadingUpdate
                      : updateInfo.asset_download_url
                      ? t.btnDownloadUpdate
                      : t.installerBuilding
                  }}
                </span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- About Modal Dialog -->
    <Transition name="update-modal">
      <div
        v-if="isAboutModalOpen"
        class="update-modal-backdrop"
        @click.self="isAboutModalOpen = false"
      >
        <div class="about-modal-card">
          <!-- Modal Header -->
          <div class="about-modal-header">
            <div class="about-header-brand">
              <img src="/app-icon.svg" alt="App Logo" class="about-app-logo" />
              <div class="about-header-titles">
                <div class="about-title-row">
                  <h3 class="about-app-name">{{ t.appTitle }}</h3>
                  <span class="about-version-badge">v{{ currentAppVersion }}</span>
                </div>
                <p class="about-app-subtitle">{{ t.aboutSubtitle }}</p>
              </div>
            </div>
            <button
              type="button"
              class="update-modal-close"
              :title="t.closeBtn"
              @click="isAboutModalOpen = false"
            >
              <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2.2" fill="none">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <!-- Modal Body -->
          <div class="about-modal-body">
            <!-- Author Card -->
            <div class="about-author-card">
              <div class="author-left">
                <img src="/xrsec-logo.png" alt="XRSec Logo" class="author-avatar-img" />
                <div class="author-info">
                  <div class="author-name-row">
                    <span class="author-name">XRSec</span>
                    <span class="author-badge">Author</span>
                  </div>
                  <span class="author-desc">Cyber Security & AI Tooling</span>
                </div>
              </div>
              <button
                type="button"
                class="author-github-link"
                :title="t.githubProfileLabel"
                @click="openUrl('https://github.com/XRSec')"
              >
                <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/>
                </svg>
                <span>@XRSec</span>
              </button>
            </div>

            <!-- Description -->
            <div class="about-desc-card">
              <p class="about-desc-text">{{ t.aboutDesc }}</p>
            </div>

            <!-- Links & Metadata -->
            <div class="about-info-grid">
              <div class="about-info-item full" @click="openUrl('https://github.com/XRSec/AI-Agent-Launcher')">
                <span class="info-label">{{ t.githubRepoLabel }}</span>
                <div class="info-val-link">
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor">
                    <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/>
                  </svg>
                  <span>XRSec / AI-Agent-Launcher</span>
                  <svg viewBox="0 0 24 24" width="11" height="11" stroke="currentColor" stroke-width="2" fill="none">
                    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
                    <polyline points="15 3 21 3 21 9"/>
                    <line x1="10" y1="14" x2="21" y2="3"/>
                  </svg>
                </div>
              </div>

              <div class="about-info-item">
                <span class="info-label">{{ t.licenseLabel }}</span>
                <span class="info-val">{{ t.licenseValue }}</span>
              </div>

              <div class="about-info-item">
                <span class="info-label">{{ t.architectureLabel }}</span>
                <span class="info-val">Tauri 2 · Rust · Vue 3</span>
              </div>
            </div>
          </div>

          <!-- Modal Footer -->
          <div class="about-modal-footer">
            <button
              type="button"
              class="about-action-btn secondary"
              :class="{ 'is-spinning': isCheckingUpdate }"
              @click="() => checkUpdate(false)"
            >
              <svg class="header-icon-svg" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              <span>{{ t.checkUpdateNow }}</span>
            </button>

            <button
              type="button"
              class="about-action-btn primary"
              @click="isAboutModalOpen = false"
            >
              {{ t.closeBtn }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
/* Claude Design System (CDS) Integration:
   - Surface Tokens: var(--cds-surface-0) ~ var(--cds-surface-3)
   - Accent: var(--cds-clay: #d97757) & Hover #cc5a36
   - Border: var(--cds-border) with Single Enclosing Border Principle
   - Dark Mode: Automatic via .dark / [data-theme="dark"]
*/

.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: var(--cds-surface-0, #f5f5f3);
  color: var(--cds-text-primary, #151515);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-weight: 400;
  user-select: none;
  -webkit-user-select: none;
  overflow: hidden;
}

.app-layout :deep(*),
.app-layout * {
  user-select: none;
  -webkit-user-select: none;
}

.app-layout :deep(input),
.app-layout :deep(textarea),
.app-layout :deep(select),
.app-layout input,
.app-layout textarea,
.app-layout select {
  user-select: text;
  -webkit-user-select: text;
}

.app-layout :deep(.log-body),
.app-layout :deep(.log-line),
.app-layout :deep(.log-body *),
.log-body,
.log-line {
  user-select: text !important;
  -webkit-user-select: text !important;
}

.app-layout.is-resizing {
  user-select: none;
  cursor: col-resize;
}

/* Global Light/Dark Scrollbars */
.scroll-content::-webkit-scrollbar,
.profile-list::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}

.scroll-content::-webkit-scrollbar-track,
.profile-list::-webkit-scrollbar-track {
  background: transparent;
}

.scroll-content::-webkit-scrollbar-thumb,
.profile-list::-webkit-scrollbar-thumb {
  background: var(--cds-border-strong);
  border-radius: 3px;
}

.scroll-content::-webkit-scrollbar-thumb:hover,
.profile-list::-webkit-scrollbar-thumb:hover {
  background: var(--cds-text-muted);
}

/* Header */
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: var(--cds-surface-1, #fcfcfb);
  border-bottom: 1px solid var(--cds-border);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.02);
  z-index: 20;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-box {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-lg, 8px);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
  background: transparent;
  flex-shrink: 0;
}

.brand-logo-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.brand-text {
  display: flex;
  align-items: center;
  gap: 8px;
}

.app-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--cds-text-primary, #151515);
  letter-spacing: -0.1px;
  margin: 0;
}

.app-version {
  font-size: 10px;
  font-weight: 500;
  padding: 1px 6px;
  border-radius: var(--radius-sm, 4px);
  background: var(--cds-fill-secondary);
  color: var(--cds-text-secondary);
  border: 1px solid var(--cds-border);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.clean-toast {
  font-size: 11px;
  color: var(--cds-clay, #d97757);
  background: var(--cds-fill-brand-subtle);
  border: 1px solid rgba(217, 119, 87, 0.3);
  padding: 3px 10px;
  border-radius: 6px;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-2px); }
  to { opacity: 1; transform: translateY(0); }
}

.autostart-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  border-radius: 9999px;
  background: var(--cds-fill-secondary);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
  font-family: inherit;
  line-height: 1.2;
}

.autostart-pill:hover {
  background: var(--cds-fill-secondary-hover);
  color: var(--cds-text-primary);
  border-color: var(--cds-border-strong);
}

.autostart-pill.active {
  background: rgba(16, 185, 129, 0.12);
  border-color: rgba(16, 185, 129, 0.3);
  color: #059669;
}

.dark .autostart-pill.active,
[data-theme="dark"] .autostart-pill.active {
  color: #34d399;
}

.autostart-pill.loading {
  opacity: 0.6;
  cursor: wait;
}

.autostart-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--cds-text-muted);
  transition: background 0.15s ease;
}

.autostart-pill.active .autostart-dot {
  background: #10b981;
}

.running-count-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  border-radius: 9999px;
  background: var(--cds-fill-secondary);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  font-size: 11px;
  font-weight: 500;
}

.running-count-pill.active {
  background: var(--cds-fill-brand-subtle);
  border-color: rgba(217, 119, 87, 0.3);
  color: var(--cds-clay, #d97757);
}

.header-control-group {
  display: inline-flex;
  align-items: center;
}

/* Main Body */
.main-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

/* Sidebar */
.sidebar {
  background: var(--cds-surface-1, #fcfcfb);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
  border-right: 1px solid var(--cds-border);
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid var(--cds-border);
  font-size: 11px;
  font-weight: 500;
  color: var(--cds-text-muted, #8a8a8a);
  letter-spacing: 0.2px;
}

.add-icon-btn {
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  cursor: pointer;
  padding: 3px;
  border-radius: var(--radius-sm, 4px);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  box-shadow: var(--cds-shadow-sm);
}

.add-icon-btn:hover {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
  border-color: var(--cds-border-strong);
}

.profile-list {
  flex: 1;
  overflow-y: auto;
  padding: 7px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.profile-item {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 10px;
  border-radius: var(--radius-lg, 8px);
  cursor: pointer;
  background: var(--cds-surface-2, #ffffff);
  color: var(--cds-text-secondary, #525252);
  transition: all 0.15s ease;
  border: 1px solid var(--cds-border);
  box-shadow: var(--cds-shadow-sm);
}

.profile-item:hover {
  background: var(--cds-fill-ghost-hover);
  border-color: var(--cds-border-strong);
  color: var(--cds-text-primary);
}

.profile-item.selected {
  background: var(--cds-surface-2, #ffffff);
  border-color: var(--cds-clay, #d97757);
  color: var(--cds-text-primary);
  box-shadow: 0 2px 6px -1px var(--cds-clay-glow, rgba(217, 119, 87, 0.25));
}

.profile-item.selected::before {
  content: "";
  position: absolute;
  left: 0;
  top: 7px;
  bottom: 7px;
  width: 3px;
  border-radius: 0 3px 3px 0;
  background: var(--cds-clay, #d97757);
}

.profile-info {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
  flex: 1;
}

.profile-avatar {
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.profile-text-box {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex: 1;
}

.profile-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--cds-text-primary, #151515);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.profile-status-badge {
  flex-shrink: 0;
  margin-left: 6px;
}

.status-tag {
  font-size: 9.5px;
  padding: 2px 6px;
  border-radius: var(--radius-sm, 4px);
  font-weight: 500;
}

.status-tag.running {
  background: rgba(16, 185, 129, 0.12);
  color: #059669;
  border: 1px solid rgba(16, 185, 129, 0.25);
}

.dark .status-tag.running,
[data-theme="dark"] .status-tag.running {
  color: #34d399;
}

.status-tag.stopped {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-muted);
}

/* Compact sidebar styles when collapsed < 170px */
.sidebar.compact .sidebar-header {
  padding: 8px 8px;
}

.sidebar.compact .profile-list {
  padding: 4px;
  gap: 3px;
}

.sidebar.compact .profile-item {
  padding: 5px 6px;
}

.sidebar.compact .profile-info {
  gap: 6px;
}

.sidebar.compact .sidebar-footer {
  padding: 6px 4px;
  gap: 3px;
}

/* Ultra-compact rail mode when collapsed < 90px (down to 52px) */
.sidebar.ultra-compact .sidebar-header {
  justify-content: center;
  padding: 8px 0;
}

.sidebar.ultra-compact .profile-item {
  justify-content: center;
  padding: 8px 0;
}

.sidebar.ultra-compact .profile-info {
  justify-content: center;
  gap: 0;
}

.sidebar.ultra-compact .sidebar-footer {
  justify-content: center;
  padding: 6px 4px;
}

.sidebar-footer {
  padding: 9px 10px;
  border-top: 1px solid var(--cds-border);
  display: flex;
  gap: 6px;
  background: var(--cds-surface-1, #fcfcfb);
}

.footer-btn {
  flex: 1;
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  font-size: 11px;
  font-weight: 500;
  padding: 4px 0;
  border-radius: var(--radius-md, 6px);
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: var(--cds-shadow-sm);
}

.footer-btn:hover:not(:disabled) {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
  border-color: var(--cds-border-strong);
}

.footer-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.footer-btn.primary-add {
  background: var(--cds-fill-brand-subtle);
  border-color: rgba(217, 119, 87, 0.3);
  color: var(--cds-clay, #d97757);
  font-weight: 600;
}

.footer-btn.primary-add:hover:not(:disabled) {
  background: var(--cds-clay, #d97757);
  color: #ffffff;
  border-color: var(--cds-clay, #d97757);
}

.footer-btn.danger {
  color: var(--cds-fill-danger, #ef4444);
}

.footer-btn.danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.12);
  border-color: rgba(239, 68, 68, 0.3);
}

/* Vertical Resizer */
.resizer-v {
  width: 5px;
  background: transparent;
  cursor: col-resize;
  position: relative;
  flex-shrink: 0;
  transition: background-color 0.15s ease;
  z-index: 10;
  touch-action: none;
}

.resizer-v:hover,
.resizer-v:active {
  background: var(--cds-clay, #d97757);
}

.resizer-thumb-v {
  position: absolute;
  top: 50%;
  left: 1.5px;
  transform: translateY(-50%);
  width: 2px;
  height: 24px;
  border-radius: 2px;
  background: var(--cds-border-strong);
}

/* Content Panel */
.content-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--cds-surface-0, #f5f5f3);
}

/* Detail Header */
.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 20px;
  background: var(--cds-surface-1, #fcfcfb);
  border-bottom: 1px solid var(--cds-border);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.02);
}

.detail-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.agent-name-input {
  background: transparent;
  border: none;
  outline: none;
  font-size: 15px;
  font-weight: 500;
  color: var(--cds-text-primary, #151515);
  width: 220px;
  border-bottom: 1.5px solid transparent;
  transition: border-color 0.15s ease;
}

.agent-name-input:focus {
  border-bottom-color: var(--cds-clay, #d97757);
}

.profile-state-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 9px;
  border-radius: 9999px;
  background: var(--cds-fill-secondary);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  font-size: 11px;
  font-weight: 500;
}

.profile-state-pill.running {
  background: rgba(16, 185, 129, 0.12);
  border-color: rgba(16, 185, 129, 0.3);
  color: #059669;
}

.dark .profile-state-pill.running,
[data-theme="dark"] .profile-state-pill.running {
  color: #34d399;
}

.profile-state-pill.failed {
  background: rgba(239, 68, 68, 0.12);
  border-color: rgba(239, 68, 68, 0.3);
  color: #dc2626;
}

.dark .profile-state-pill.failed,
[data-theme="dark"] .profile-state-pill.failed {
  color: #f87171;
}

.changes-hint {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--cds-clay, #d97757);
  font-size: 10.5px;
  font-weight: 500;
}

.hint-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
}

.saved-hint {
  color: var(--cds-text-muted);
  font-size: 10.5px;
  font-weight: 400;
}

/* Header Action Buttons */
.detail-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.action-btn-clean {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--cds-surface-2, #ffffff);
  color: var(--cds-text-secondary);
  border: 1px solid var(--cds-border);
  padding: 5px 11px;
  border-radius: var(--radius-md, 6px);
  font-size: 11.5px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: var(--cds-shadow-sm);
}

.action-btn-clean:hover {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
  border-color: var(--cds-border-strong);
}

.action-btn-danger {
  background: var(--cds-fill-danger, #ef4444);
  color: #ffffff;
  border: none;
  padding: 5px 16px;
  border-radius: var(--radius-md, 6px);
  font-size: 11.5px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: var(--cds-shadow-sm);
}

.action-btn-danger:hover {
  background: #dc2626;
}

.action-btn-secondary {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
  border: 1px solid var(--cds-border);
  padding: 5px 16px;
  border-radius: var(--radius-md, 6px);
  font-size: 11.5px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.action-btn-secondary:hover {
  background: var(--cds-fill-secondary-hover);
}

.action-btn-primary {
  background: var(--cds-clay, #d97757);
  color: #ffffff;
  border: none;
  padding: 5px 18px;
  border-radius: var(--radius-md, 6px);
  font-size: 11.5px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: var(--cds-shadow-sm);
}

.action-btn-primary:hover {
  background: var(--cds-clay-hover, #cc5a36);
}

.config-scroll-wrapper {
  flex: 1;
  min-height: 0;
  height: 100%;
}

/* Scrollable Config Area */
.scroll-content {
  flex: 1;
  overflow-y: auto;
  padding: 14px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  background: transparent;
}

.error-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: var(--cds-fill-danger, #ef4444);
  border-radius: var(--radius-md, 6px);
  font-size: 11.5px;
  font-family: monospace;
}

/* Cards */
.card {
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  border-radius: var(--cds-panel-radius, 10px);
  padding: 13px 16px;
  box-shadow: var(--cds-panel-shadow);
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.card:hover {
  border-color: var(--cds-border-strong);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 11px;
}

.card-title-box {
  display: flex;
  align-items: center;
  gap: 10px;
}

.card-icon {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-icon.command {
  background: var(--cds-fill-brand-subtle);
  color: var(--cds-clay, #d97757);
  border: 1px solid rgba(217, 119, 87, 0.25);
}

.card-icon.runtime {
  background: var(--cds-fill-brand-subtle);
  color: var(--cds-clay, #d97757);
  border: 1px solid rgba(217, 119, 87, 0.25);
}

.card-icon.policy {
  background: var(--cds-fill-brand-subtle);
  color: var(--cds-clay, #d97757);
  border: 1px solid rgba(217, 119, 87, 0.25);
}

.card-title {
  margin: 0;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--cds-text-primary, #151515);
}

.card-desc {
  margin: 2px 0 0 0;
  font-size: 10.5px;
  font-weight: 400;
  color: var(--cds-text-muted, #8a8a8a);
}

/* Code Editor (Claude Dark Terminal Box) */
.code-editor-box {
  border-radius: var(--radius-lg, 8px);
  border: 1px solid var(--cds-border-strong);
  overflow: hidden;
  background: #141413;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}

.editor-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  background: #1c1c1a;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.editor-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.editor-dot.red { background: #ef4444; }
.editor-dot.yellow { background: #f59e0b; }
.editor-dot.green { background: #10b981; }

.editor-lang-tag {
  margin-left: auto;
  font-family: monospace;
  font-size: 10px;
  font-weight: 400;
  color: #a8a29e;
}

.code-textarea {
  width: 100%;
  height: 120px;
  background: #141413;
  color: #f5f5f4;
  font-family: "SF Mono", "Fira Code", Menlo, Monaco, Consolas, monospace;
  font-size: 11px;
  font-weight: 400;
  line-height: 1.6;
  padding: 10px 12px;
  border: none;
  outline: none;
  resize: vertical;
  box-sizing: border-box;
  scrollbar-width: thin;
  scrollbar-color: #44403c #141413;
}

.code-textarea::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.code-textarea::-webkit-scrollbar-track {
  background: #141413;
}

.code-textarea::-webkit-scrollbar-thumb {
  background: #44403c;
  border-radius: 3px;
}

.code-textarea::-webkit-scrollbar-thumb:hover {
  background: #57534e;
}

.code-textarea:focus {
  outline: none;
}

/* Form Grid */
.form-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.form-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.form-label {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.label-title {
  font-size: 11.5px;
  font-weight: 500;
  color: var(--cds-text-primary, #151515);
}

.label-desc {
  font-size: 10.5px;
  font-weight: 400;
  color: var(--cds-text-muted, #8a8a8a);
}

.form-input-group {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 360px;
}

.text-input,
.select-input {
  flex: 1;
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  border-radius: var(--radius-md, 6px);
  padding: 5px 9px;
  color: var(--cds-text-primary, #151515);
  font-size: 11.5px;
  font-weight: 400;
  outline: none;
  box-sizing: border-box;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.select-input {
  cursor: pointer;
}

.text-input:focus,
.select-input:focus {
  border-color: var(--cds-clay, #d97757);
  box-shadow: 0 0 0 2px var(--cds-clay-glow, rgba(217, 119, 87, 0.25));
}

.text-input.mono {
  font-family: "SF Mono", "Fira Code", Menlo, Monaco, Consolas, monospace;
}

/* Shell Row */
.shell-input-group {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 360px;
}

.shell-text-field {
  width: 135px;
  flex: none;
}

.shell-chips {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.chip-btn {
  background: var(--cds-fill-secondary);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  padding: 4px 8px;
  border-radius: var(--radius-sm, 4px);
  font-size: 10.5px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.12s ease;
}

.chip-btn:hover {
  background: var(--cds-fill-secondary-hover);
  color: var(--cds-text-primary);
  border-color: var(--cds-border-strong);
}

.chip-btn.active {
  background: var(--cds-clay, #d97757);
  border-color: var(--cds-clay, #d97757);
  color: #ffffff;
  font-weight: 500;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}

.input-unit {
  font-size: 11px;
  color: var(--cds-text-muted);
}

.small-btn {
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  padding: 5px 11px;
  border-radius: var(--radius-md, 6px);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.small-btn:hover {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
  border-color: var(--cds-border-strong);
}

.hint-msg {
  font-size: 10.5px;
  color: var(--cds-text-muted);
  text-align: right;
}

/* Toggles */
.toggle-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 2px;
}

.toggle-item {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  cursor: pointer;
  color: var(--cds-text-primary, #151515);
}

/* Horizontal Resizer between Config & Terminal Log */
.resizer-h {
  height: 5px;
  background: transparent;
  cursor: row-resize;
  position: relative;
  flex-shrink: 0;
  transition: background-color 0.15s ease;
  z-index: 10;
  touch-action: none;
}

.resizer-h:hover,
.resizer-h:active {
  background: var(--cds-clay, #d97757);
}

.resizer-thumb-h {
  position: absolute;
  left: 50%;
  top: 1.5px;
  transform: translateX(-50%);
  width: 24px;
  height: 2px;
  border-radius: 2px;
  background: var(--cds-border-strong);
}

/* Log Section (Warm Claude Code Dark Console) */
.log-section {
  position: relative;
  background: #1c1917;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  min-height: 90px;
  overflow: hidden;
  box-shadow: inset 0 2px 6px rgba(0, 0, 0, 0.25);
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px;
  background: #24201d;
  border-bottom: 1px solid #34302c;
}

.log-title-box {
  display: flex;
  align-items: center;
  gap: 7px;
  color: #a8a29e;
  font-size: 11px;
  font-weight: 450;
}

.log-terminal-icon {
  color: #f59e0b;
  display: flex;
}

.log-title {
  color: #f5f5f4;
}

.log-stats {
  font-family: monospace;
  font-size: 10px;
  color: #8c8479;
}

.log-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 10.5px;
  color: #a8a29e;
}

.log-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: #2a2622;
  border: 1px solid #3c3732;
  color: #a8a29e;
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 10.5px;
  cursor: pointer;
  transition: all 0.15s ease;
  line-height: 1.2;
}

.log-btn:hover {
  background: #342f2a;
  color: #f5f5f4;
  border-color: #4a443e;
}

.log-btn.active {
  background: #d97706;
  border-color: #b45309;
  color: #ffffff;
  font-weight: 500;
}

.log-btn.icon-only {
  padding: 3px 6px;
}

.auto-scroll-label {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}

.clear-btn {
  background: transparent;
  border: none;
  color: #a8a29e;
  cursor: pointer;
  font-size: 10.5px;
  padding: 2px 4px;
  transition: color 0.15s ease;
}

.clear-btn:hover {
  color: #f5f5f4;
}

/* Log Search Panel */
.log-search-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 14px;
  background: #221e1b;
  border-bottom: 1px solid #383430;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.25);
  animation: searchSlideIn 0.15s ease;
  z-index: 5;
}

@keyframes searchSlideIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.search-input-box {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  background: #171513;
  border: 1px solid #383430;
  border-radius: 5px;
  padding: 3px 8px;
  transition: border-color 0.15s ease;
}

.search-input-box:focus-within {
  border-color: #d97706;
  box-shadow: 0 0 0 2px rgba(217, 119, 6, 0.2);
}

.search-prefix-icon {
  color: #8c8479;
  flex-shrink: 0;
}

.search-inner-input {
  width: 100%;
  background: transparent;
  border: none;
  outline: none;
  font-family: inherit;
  font-size: 11px;
  color: #f5f5f4;
}

.search-inner-input::placeholder {
  color: #6b645c;
}

.search-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.search-match-badge {
  font-family: monospace;
  font-size: 10px;
  color: #8c8479;
  padding: 2px 6px;
  background: #171513;
  border-radius: 4px;
  border: 1px solid #34302c;
  white-space: nowrap;
}

.search-match-badge.has-matches {
  color: #f59e0b;
  border-color: #6d4812;
  background: #2b1f13;
}

.search-match-badge.no-matches {
  color: #f87171;
  border-color: #5c2020;
  background: #2b1414;
}

.search-sub-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  background: #2a2622;
  border: 1px solid #383430;
  border-radius: 4px;
  color: #a8a29e;
  font-size: 10.5px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.12s ease;
}

.search-sub-btn:hover:not(:disabled) {
  background: #383430;
  color: #f5f5f4;
}

.search-sub-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.search-sub-btn.active {
  background: #d97706;
  border-color: #b45309;
  color: #ffffff;
}

.search-sub-btn.close:hover {
  background: #451a1a;
  color: #fca5a5;
  border-color: #7f1d1d;
}

/* Dedicated Realtime Console Body */
.log-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 16px 20px 16px;
  box-sizing: border-box;
  font-family: "SF Mono", "Fira Code", Menlo, Monaco, Consolas, monospace;
  font-size: 11px;
  font-weight: 400;
  line-height: 1.6;
  color: #e7e5e4;
  user-select: text;
  background: #1c1917;
  scrollbar-width: thin;
  scrollbar-color: #44403c #1c1917;
}

.log-bottom-spacer {
  height: 16px;
  flex-shrink: 0;
}

/* Dark Scrollbar for Terminal Console */
.log-body::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.log-body::-webkit-scrollbar-track {
  background: #1c1917;
}

.log-body::-webkit-scrollbar-thumb {
  background: #44403c;
  border-radius: 3px;
}

.log-body::-webkit-scrollbar-thumb:hover {
  background: #57534e;
}

.log-line {
  white-space: pre-wrap;
  word-break: break-all;
}

.log-line.is-error {
  color: #f87171;
}

.log-line.is-info {
  color: #fbbf24;
}

/* Search Highlights */
.search-match {
  background: #854d0e;
  color: #fef3c7;
  border-radius: 2px;
  padding: 0 1px;
}

.search-match-active {
  background: #f59e0b;
  color: #1c1917;
  font-weight: 600;
  border-radius: 2px;
  padding: 0 2px;
  outline: 2px solid #fde68a;
}

.log-empty {
  color: #78716c;
  font-style: italic;
  padding: 8px 0;
}

/* Floating Jump to Bottom Button */
.log-fade-wrap {
  flex: 1;
  min-height: 0;
  height: calc(100% - 32px);
  width: 100%;
}

.jump-to-bottom-pill {
  position: absolute;
  bottom: 14px;
  right: 24px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  border-radius: 9999px;
  background: #24201d;
  border: 1px solid #44403c;
  color: var(--cds-clay, #d97757);
  font-size: 11px;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  cursor: pointer;
  transition: all 0.15s ease;
  z-index: 10;
  user-select: none;
}

.jump-to-bottom-pill:hover {
  background: #2e2824;
  border-color: var(--cds-clay, #d97757);
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.5);
}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

/* Update Available Header Badge Pill */
.update-badge-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-left: 8px;
  padding: 2px 9px 2px 7px;
  border-radius: 9999px;
  background: var(--cds-fill-brand-subtle);
  border: 1px solid rgba(217, 119, 87, 0.38);
  color: var(--cds-clay, #d97757);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  box-shadow: 0 1px 3px var(--cds-clay-glow);
  position: relative;
  user-select: none;
}

.update-badge-pill:hover {
  background: rgba(217, 119, 87, 0.25);
  border-color: var(--cds-clay, #d97757);
  transform: translateY(-1px);
  box-shadow: 0 3px 8px var(--cds-clay-glow);
}

.update-ping-circle {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--cds-clay, #d97757);
  box-shadow: 0 0 0 0 rgba(217, 119, 87, 0.7);
  animation: pulse-ping 2s cubic-bezier(0, 0, 0.2, 1) infinite;
}

@keyframes pulse-ping {
  0% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 rgba(217, 119, 87, 0.7);
  }
  70% {
    transform: scale(1);
    box-shadow: 0 0 0 6px rgba(217, 119, 87, 0);
  }
  100% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 rgba(217, 119, 87, 0);
  }
}

.update-emoji {
  font-size: 11px;
  line-height: 1;
}

.update-badge-text {
  letter-spacing: -0.01em;
}

/* Header Action Icon Button */
.header-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-md, 6px);
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.header-icon-btn:hover {
  background: var(--cds-fill-secondary);
  border-color: var(--cds-border-strong);
  color: var(--cds-text-primary);
}

.header-icon-svg {
  width: 14px;
  height: 14px;
}

.header-icon-btn.is-spinning .header-icon-svg {
  animation: spin 1s linear infinite;
  color: var(--cds-clay, #d97757);
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* Update Modal Backdrop & Card */
.update-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.update-modal-card {
  width: 100%;
  max-width: 520px;
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  border-radius: 14px;
  box-shadow: var(--cds-shadow-popover);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: modal-pop 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  color: var(--cds-text-primary);
}

@keyframes modal-pop {
  0% {
    opacity: 0;
    transform: scale(0.95) translateY(8px);
  }
  100% {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.update-modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 18px 20px 14px 20px;
  border-bottom: 1px solid var(--cds-border);
  background: var(--cds-surface-1);
}

.update-header-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.update-rocket-badge {
  font-size: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 10px;
  background: var(--cds-fill-brand-subtle);
  border: 1px solid rgba(217, 119, 87, 0.25);
}

.update-modal-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--cds-text-primary);
  letter-spacing: -0.01em;
}

.update-modal-subtitle {
  margin: 3px 0 0 0;
  font-size: 12px;
  color: var(--cds-text-muted);
}

.update-modal-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--cds-text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.update-modal-close:hover {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
}

.update-modal-body {
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-height: calc(80vh - 140px);
  overflow-y: auto;
  background: var(--cds-surface-2);
}

/* Version Comparison Row */
.update-version-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: var(--radius-lg, 8px);
  background: var(--cds-surface-1);
  border: 1px solid var(--cds-border);
}

.version-chip {
  display: flex;
  flex-direction: column;
}

.version-chip .chip-label {
  font-size: 10px;
  color: var(--cds-text-muted);
  text-transform: uppercase;
  font-weight: 500;
  letter-spacing: 0.04em;
}

.version-chip .chip-val {
  font-size: 13px;
  font-weight: 600;
  color: var(--cds-text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.version-chip.latest .chip-val {
  color: var(--cds-clay, #d97757);
}

.version-arrow {
  color: var(--cds-border-strong);
  font-size: 13px;
}

.update-date-badge {
  margin-left: auto;
  font-size: 11px;
  color: var(--cds-text-muted);
}

/* Matched Asset Pill */
.update-asset-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  border-radius: 6px;
  background: var(--cds-surface-1);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-secondary);
  font-size: 11.5px;
}

.update-asset-pill .asset-name {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-weight: 500;
  color: var(--cds-text-primary);
  word-break: break-all;
}

.update-asset-pill .asset-size {
  margin-left: auto;
  font-size: 11px;
  color: var(--cds-text-muted);
  background: var(--cds-surface-2);
  padding: 1px 6px;
  border-radius: 4px;
  border: 1px solid var(--cds-border);
}

/* Notes Card */
.update-notes-card {
  border-radius: var(--radius-lg, 8px);
  border: 1px solid var(--cds-border);
  background: var(--cds-surface-1);
  overflow: hidden;
}

.notes-header {
  padding: 8px 12px;
  background: var(--cds-surface-1);
  font-size: 11px;
  font-weight: 600;
  color: var(--cds-text-secondary);
  border-bottom: 1px solid var(--cds-border);
}

.notes-content {
  padding: 12px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--cds-text-primary);
  max-height: 180px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

/* Download Progress Box */
.update-download-box {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border-radius: var(--radius-lg, 8px);
  background: var(--cds-surface-1);
  border: 1px solid var(--cds-border);
}

.progress-bar-track {
  height: 8px;
  border-radius: 9999px;
  background: var(--cds-border);
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  border-radius: 9999px;
  background: linear-gradient(90deg, #f59e0b, var(--cds-clay, #d97757));
  transition: width 0.2s ease;
  box-shadow: 0 0 8px var(--cds-clay-glow);
}

.progress-info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11.5px;
}

.progress-text {
  color: var(--cds-text-muted);
}

.progress-percent {
  font-weight: 600;
  color: var(--cds-clay, #d97757);
  font-family: ui-monospace, SFMono-Regular, monospace;
}

/* Error Banner */
.update-error-banner {
  padding: 8px 12px;
  border-radius: 6px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: var(--cds-fill-danger, #ef4444);
  font-size: 12px;
  line-height: 1.4;
}

/* Modal Footer Actions */
.update-modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-top: 1px solid var(--cds-border);
  background: var(--cds-surface-1);
}

.footer-right-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.update-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: var(--radius-md, 6px);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
}

.update-action-btn.secondary {
  background: var(--cds-surface-2);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-primary);
}

.update-action-btn.secondary:hover {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
}

.update-action-btn.ghost {
  background: transparent;
  border: 1px solid transparent;
  color: var(--cds-text-muted);
}

.update-action-btn.ghost:hover {
  background: var(--cds-fill-secondary);
  color: var(--cds-text-primary);
}

.update-action-btn.primary {
  background: var(--cds-clay, #d97757);
  border: 1px solid var(--cds-clay, #d97757);
  color: #ffffff;
  box-shadow: var(--cds-shadow-sm);
}

.update-action-btn.primary:hover:not(:disabled) {
  background: var(--cds-clay-hover, #cc5a36);
  transform: translateY(-1px);
  box-shadow: var(--cds-shadow-md);
}

.update-action-btn.primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-spinner {
  width: 13px;
  height: 13px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.update-building-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 12px;
  border-radius: 6px;
  background: var(--cds-fill-brand-subtle);
  border: 1px solid rgba(217, 119, 87, 0.25);
  color: var(--cds-clay, #d97757);
  font-size: 11.5px;
  line-height: 1.45;
}

.update-building-notice .building-icon {
  font-size: 14px;
  flex-shrink: 0;
}

/* About Modal Card & Components */
.about-modal-card {
  width: 100%;
  max-width: 480px;
  background: var(--cds-surface-2, #ffffff);
  border: 1px solid var(--cds-border);
  border-radius: 14px;
  box-shadow: var(--cds-shadow-popover);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: modal-pop 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  color: var(--cds-text-primary);
}

.about-modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 18px 20px 14px 20px;
  border-bottom: 1px solid var(--cds-border);
  background: var(--cds-surface-1);
}

.about-header-brand {
  display: flex;
  align-items: center;
  gap: 13px;
}

.about-app-logo {
  width: 46px;
  height: 46px;
  border-radius: 11px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
  flex-shrink: 0;
}

.about-header-titles {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.about-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.about-app-name {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--cds-text-primary);
  letter-spacing: -0.01em;
}

.about-version-badge {
  font-size: 11px;
  font-weight: 600;
  color: var(--cds-clay, #d97757);
  background: var(--cds-fill-brand-subtle);
  border: 1px solid rgba(217, 119, 87, 0.3);
  padding: 1px 7px;
  border-radius: 6px;
}

.about-app-subtitle {
  margin: 2px 0 0 0;
  font-size: 11.5px;
  color: var(--cds-text-muted);
  line-height: 1.4;
}

.about-modal-body {
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--cds-surface-2);
}

.about-author-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 11px 14px;
  background: var(--cds-surface-1);
  border: 1px solid var(--cds-border);
  border-radius: 10px;
}

.author-left {
  display: flex;
  align-items: center;
  gap: 11px;
}

.author-avatar-img {
  width: 38px;
  height: 38px;
  border-radius: 8px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.author-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.author-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.author-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--cds-text-primary);
}

.author-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--cds-clay, #d97757);
  background: var(--cds-fill-brand-subtle);
  padding: 0 5px;
  border-radius: 4px;
}

.author-desc {
  font-size: 11px;
  color: var(--cds-text-muted);
}

.author-github-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  color: var(--cds-text-primary);
  background: var(--cds-surface-2);
  border: 1px solid var(--cds-border);
  padding: 5px 11px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.author-github-link:hover {
  background: var(--cds-fill-secondary);
  border-color: var(--cds-border-strong);
}

.about-desc-card {
  padding: 12px 14px;
  background: var(--cds-surface-1);
  border: 1px solid var(--cds-border);
  border-radius: 10px;
}

.about-desc-text {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--cds-text-secondary);
}

.about-info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 9px;
}

.about-info-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 9px 12px;
  background: var(--cds-surface-1);
  border: 1px solid var(--cds-border);
  border-radius: 8px;
}

.about-info-item.full {
  grid-column: 1 / -1;
  cursor: pointer;
  transition: all 0.15s ease;
}

.about-info-item.full:hover {
  background: var(--cds-fill-secondary);
  border-color: var(--cds-border-strong);
}

.info-label {
  font-size: 10.5px;
  color: var(--cds-text-muted);
  font-weight: 500;
}

.info-val {
  font-size: 11.5px;
  color: var(--cds-text-primary);
  font-weight: 500;
}

.info-val-link {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--cds-text-primary);
  font-weight: 600;
}

.about-modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 13px 20px;
  border-top: 1px solid var(--cds-border);
  background: var(--cds-surface-1);
}

.about-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 500;
  padding: 6px 14px;
  border-radius: var(--radius-md, 6px);
  cursor: pointer;
  transition: all 0.15s ease;
}

.about-action-btn.secondary {
  background: var(--cds-surface-2);
  border: 1px solid var(--cds-border);
  color: var(--cds-text-primary);
}

.about-action-btn.secondary:hover {
  background: var(--cds-fill-secondary);
  border-color: var(--cds-border-strong);
}

.about-action-btn.primary {
  background: var(--cds-clay, #d97757);
  border: 1px solid var(--cds-clay, #d97757);
  color: #ffffff;
}

.about-action-btn.primary:hover {
  background: var(--cds-clay-hover, #cc5a36);
}

.brand-box.is-clickable,
.app-title.is-clickable,
.app-version.is-clickable {
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.brand-box.is-clickable:hover,
.app-title.is-clickable:hover,
.app-version.is-clickable:hover {
  opacity: 0.8;
}

/* Modal Transition */
.update-modal-enter-active,
.update-modal-leave-active {
  transition: opacity 0.2s ease;
}

.update-modal-enter-from,
.update-modal-leave-to {
  opacity: 0;
}
</style>