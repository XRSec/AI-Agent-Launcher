<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  enable as enableAutostart,
  disable as disableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import type { AgentProfile, ProfileStatus, ProfileLogMessage, RestartPolicy } from "./types";
import { languageSetting, setLanguageSetting, t, type LanguageSetting } from "./i18n";

const profiles = ref<AgentProfile[]>([]);
const selectedProfileId = ref<string>("");
const statuses = ref<Record<string, ProfileStatus>>({});
const logsMap = ref<Record<string, string[]>>({});
const autoScroll = ref(true);
const detectionMessage = ref("");
const cleanToast = ref("");
const logContainer = ref<HTMLDivElement | null>(null);

const appAutostart = ref(false);
const autostartLoading = ref(false);

const runningSnapshots = ref<Record<string, AgentProfile>>({});

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

function startResizeSidebar(e: MouseEvent) {
  isDraggingSidebar.value = true;
  const startX = e.clientX;
  const startW = sidebarWidth.value;

  function onMouseMove(moveEvent: MouseEvent) {
    const delta = moveEvent.clientX - startX;
    const newW = Math.min(Math.max(startW + delta, 52), 480);
    sidebarWidth.value = newW;
    localStorage.setItem("launcher_sidebar_width", String(newW));
  }

  function onMouseUp() {
    isDraggingSidebar.value = false;
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
  }

  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
}

function startResizeLog(e: MouseEvent) {
  isDraggingLog.value = true;
  const startY = e.clientY;
  const startH = logHeight.value;

  function onMouseMove(moveEvent: MouseEvent) {
    const delta = startY - moveEvent.clientY;
    // Allow dragging up to almost the full window height
    const maxH = Math.max(window.innerHeight - 120, 260);
    const newH = Math.min(Math.max(startH + delta, 80), maxH);
    logHeight.value = newH;
    isLogMaximized.value = false;
    localStorage.setItem("launcher_log_height", String(newH));
  }

  function onMouseUp() {
    isDraggingLog.value = false;
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
  }

  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
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

const userScrolledUp = ref(false);

function onLogScroll() {
  if (!logContainer.value) return;
  const { scrollTop, scrollHeight, clientHeight } = logContainer.value;
  // If user is more than 35px from bottom, mark as scrolled up
  const atBottom = scrollHeight - scrollTop - clientHeight <= 35;
  userScrolledUp.value = !atBottom;
}

function scrollToBottom() {
  if (!autoScroll.value || userScrolledUp.value) return;
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
  });
}

function jumpToBottom() {
  userScrolledUp.value = false;
  autoScroll.value = true;
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
  });
}

watch(autoScroll, (val) => {
  if (val) {
    userScrolledUp.value = false;
    scrollToBottom();
  }
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

onMounted(async () => {
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
      scrollToBottom();
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
});
</script>

<template>
  <div class="app-layout" :class="{ 'is-resizing': isDraggingSidebar || isDraggingLog }">
    <!-- Top Header (Layer 0 Surface) -->
    <header class="app-header">
      <div class="header-left">
        <div class="brand-box">
          <img src="/app-icon.svg" alt="App Logo" class="brand-logo-img" />
        </div>
        <div class="brand-text">
          <h1 class="app-title">{{ t.appTitle }}</h1>
          <span class="app-version">v1.0.0</span>
        </div>
      </div>

      <div class="header-right">
        <!-- Toast feedback for residual cleanup and autostart -->
        <div v-if="cleanToast" class="clean-toast">
          {{ cleanToast }}
        </div>

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
          <span class="count-dot"></span>
          <span>{{ runningCount > 0 ? t.runningCount(runningCount) : t.allStopped }}</span>
        </div>

        <!-- Language Switcher (3 options: 自动/Auto, 中文, English) -->
        <div class="lang-switcher" :title="t.language">
          <button
            type="button"
            class="lang-btn"
            :class="{ active: languageSetting === 'auto' }"
            @click="setLanguageSetting('auto')"
            title="跟随系统 / Follow System"
          >
            {{ t.langAuto }}
          </button>
          <span class="lang-divider">/</span>
          <button
            type="button"
            class="lang-btn"
            :class="{ active: languageSetting === 'zh-CN' }"
            @click="setLanguageSetting('zh-CN')"
            title="简体中文"
          >
            中文
          </button>
          <span class="lang-divider">/</span>
          <button
            type="button"
            class="lang-btn"
            :class="{ active: languageSetting === 'en-US' }"
            @click="setLanguageSetting('en-US')"
            title="English"
          >
            EN
          </button>
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
              <div class="profile-avatar" :class="{ active: statuses[profile.id]?.isRunning }">
                <span v-if="statuses[profile.id]?.isRunning" class="avatar-dot active"></span>
                <span v-else class="avatar-dot"></span>
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
          <button class="footer-btn primary-add" @click="addProfile">+ {{ t.newAgent }}</button>
          <button
            v-if="sidebarWidth >= 130"
            class="footer-btn"
            :disabled="!currentProfile"
            @click="currentProfile && duplicateProfile(currentProfile.id)"
          >
            {{ t.duplicate }}
          </button>
          <button
            v-if="sidebarWidth >= 130"
            class="footer-btn danger"
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
        @mousedown="startResizeSidebar"
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
            <div class="profile-state-pill" :class="{ running: currentStatus.isRunning }">
              <span class="state-dot"></span>
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
              class="action-btn-clean"
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
              class="action-btn-danger"
              @click="stopCurrentAgent"
            >
              {{ t.btnStop }}
            </button>
            <button
              v-if="currentStatus.isRunning"
              class="action-btn-secondary"
              @click="restartCurrentAgent"
            >
              {{ t.btnRestart }}
            </button>
            <button
              v-else
              class="action-btn-primary"
              @click="startCurrentAgent"
            >
              {{ t.btnStart }}
            </button>
          </div>
        </div>

        <!-- Upper Config Scroll Area (Layer 2 Surface: Floating Cards with Depth) -->
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
                  <select
                    v-model="currentProfile.restartPolicy"
                    class="select-input"
                    @change="persistProfiles"
                  >
                    <option value="no">{{ t.restartNo }}</option>
                    <option value="always">{{ t.restartAlways }}</option>
                    <option value="on-failure">{{ t.restartOnFailure }}</option>
                  </select>
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

        <!-- Horizontal Resizer between Config & Terminal Log -->
        <div
          class="resizer-h"
          :title="t.dragResizeHeight"
          @mousedown="startResizeLog"
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

          <div
            ref="logContainer"
            class="log-body"
            @scroll="onLogScroll"
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
          </div>

          <!-- Floating Jump to Bottom Button when user scrolled up -->
          <button
            v-if="userScrolledUp"
            type="button"
            class="jump-to-bottom-pill"
            :title="t.jumpToBottom"
            @click="jumpToBottom"
          >
            <svg viewBox="0 0 24 24" width="10" height="10" stroke="currentColor" stroke-width="2.5" fill="none">
              <polyline points="6 9 12 15 18 9" />
            </svg>
            <span>{{ t.jumpToBottom }}</span>
          </button>
        </section>
      </main>
    </div>
  </div>
</template>

<style scoped>
/* Claude Code Warm Aesthetic:
   Canvas Base: #f7f4ee (Warm parchment sand)
   Card Surface: #ffffff with subtle warm stone border #e7e2d8
   Sidebar Surface: #faf7f2 with border #e7e2d8
   Brand & Amber Accent: #d97706 / #b45309 / #f59e0b
   Primary Button & Active Chips: #2b2724 (Warm deep charcoal)
   Terminal / Code Surface: #1c1917 (Warm deep espresso) with #34302c border
   Typography: Lighter font-weights (400-500), softer charcoal text #3f3b37
*/

.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: #f7f4ee;
  color: #3f3b37;
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

/* Global Light Scrollbars */
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
  background: #ded8cb;
  border-radius: 3px;
}

.scroll-content::-webkit-scrollbar-thumb:hover,
.profile-list::-webkit-scrollbar-thumb:hover {
  background: #c7c0b1;
}

/* Header */
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 18px;
  background: #ffffff;
  border-bottom: 1px solid #e7e2d8;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.02);
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
  border-radius: 7px;
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
  color: #35302b;
  letter-spacing: -0.1px;
  margin: 0;
}

.app-version {
  font-size: 10px;
  font-weight: 400;
  padding: 1px 6px;
  border-radius: 4px;
  background: #f4efe6;
  color: #8c8479;
  border: 1px solid #e5dfd3;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.clean-toast {
  font-size: 11px;
  color: #92400e;
  background: #fef3c7;
  border: 1px solid #fde68a;
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
  background: #f4efe6;
  border: 1px solid #e5dfd3;
  color: #7c7468;
  font-size: 11px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
  font-family: inherit;
  line-height: 1.2;
}

.autostart-pill:hover {
  background: #eae3d6;
  color: #2b2724;
  border-color: #d6cebf;
}

.autostart-pill.active {
  background: #e8f5e9;
  border-color: #c8e6c9;
  color: #2e7d32;
}

.autostart-pill.active:hover {
  background: #dcf0de;
  border-color: #b7deb8;
}

.autostart-pill.loading {
  opacity: 0.6;
  cursor: wait;
}

.autostart-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #a89f91;
  transition: background 0.15s ease;
}

.autostart-pill.active .autostart-dot {
  background: #2e7d32;
}

.running-count-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  border-radius: 9999px;
  background: #f4efe6;
  border: 1px solid #e5dfd3;
  color: #7c7468;
  font-size: 11px;
  font-weight: 450;
}

.running-count-pill.active {
  background: #fef3c7;
  border-color: #fde68a;
  color: #b45309;
}

.count-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

/* Language Switcher */
.lang-switcher {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  background: #f4efe6;
  border: 1px solid #e5dfd3;
  border-radius: 6px;
  padding: 2px 3px;
  user-select: none;
}

.lang-btn {
  background: transparent;
  border: none;
  font-size: 11px;
  font-weight: 450;
  color: #7c7468;
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
  line-height: 1.2;
}

.lang-btn:hover {
  color: #2b2724;
}

.lang-btn.active {
  background: #ffffff;
  color: #b45309;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.08);
}

.lang-divider {
  font-size: 10px;
  color: #b8b0a2;
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
  background: #faf7f2;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
  border-right: 1px solid #e7e2d8;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid #e7e2d8;
  font-size: 11px;
  font-weight: 500;
  color: #8c8479;
  letter-spacing: 0.2px;
}

.add-icon-btn {
  background: #ffffff;
  border: 1px solid #dcd6cb;
  color: #575047;
  cursor: pointer;
  padding: 3px;
  border-radius: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.02);
}

.add-icon-btn:hover {
  background: #f5f0e6;
  color: #2b2724;
  border-color: #beb7aa;
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
  border-radius: 7px;
  cursor: pointer;
  background: #ffffff;
  color: #4a453e;
  transition: all 0.15s ease;
  border: 1px solid #e8e3da;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.02);
}

.profile-item:hover {
  background: #fdfbf7;
  border-color: #dad4c8;
  color: #2d2925;
}

.profile-item.selected {
  background: #ffffff;
  border-color: #d97706;
  color: #262320;
  box-shadow: 0 2px 6px -1px rgba(217, 119, 6, 0.15);
}

.profile-item.selected::before {
  content: "";
  position: absolute;
  left: 0;
  top: 7px;
  bottom: 7px;
  width: 3px;
  border-radius: 0 3px 3px 0;
  background: #d97706;
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

.avatar-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #dcd6cb;
}

.avatar-dot.active {
  background: #10b981;
  box-shadow: 0 0 0 2px #d1fae5;
}

.profile-text-box {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex: 1;
}

.profile-name {
  font-size: 12px;
  font-weight: 400;
  color: #35302b;
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
  border-radius: 4px;
  font-weight: 450;
}

.status-tag.running {
  background: #fef3c7;
  color: #b45309;
  border: 1px solid #fde68a;
}

.status-tag.stopped {
  background: #f2eee5;
  color: #9c958a;
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
  border-top: 1px solid #e7e2d8;
  display: flex;
  gap: 6px;
  background: #faf7f2;
}

.footer-btn {
  flex: 1;
  background: #ffffff;
  border: 1px solid #dcd6cb;
  color: #4a453e;
  font-size: 11px;
  font-weight: 400;
  padding: 4px 0;
  border-radius: 5px;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.02);
}

.footer-btn:hover:not(:disabled) {
  background: #f5f0e6;
  color: #2b2724;
  border-color: #beb7aa;
}

.footer-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.footer-btn.danger {
  color: #dc2626;
}

.footer-btn.danger:hover:not(:disabled) {
  background: #fee2e2;
  border-color: #fca5a5;
}

/* Vertical Resizer */
.resizer-v {
  width: 5px;
  background: #e7e2d8;
  cursor: col-resize;
  position: relative;
  flex-shrink: 0;
  transition: background 0.15s ease;
  z-index: 10;
}

.resizer-v:hover,
.resizer-v:active {
  background: #d97706;
}

.resizer-thumb-v {
  position: absolute;
  top: 50%;
  left: 1px;
  transform: translateY(-50%);
  width: 3px;
  height: 24px;
  border-radius: 2px;
  background: #beb7aa;
}

/* Content Panel */
.content-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: #f7f4ee;
}

/* Detail Header */
.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 20px;
  background: #ffffff;
  border-bottom: 1px solid #e7e2d8;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.02);
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
  color: #2b2724;
  width: 220px;
  border-bottom: 1px solid transparent;
  transition: border-color 0.15s ease;
}

.agent-name-input:focus {
  border-bottom-color: #d97706;
}

.profile-state-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border-radius: 9999px;
  background: #f2eee5;
  border: 1px solid #e3ddd1;
  color: #7c7468;
  font-size: 10.5px;
  font-weight: 450;
}

.profile-state-pill.running {
  background: #fef3c7;
  border-color: #fde68a;
  color: #b45309;
}

.state-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.changes-hint {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #b45309;
  font-size: 10.5px;
  font-weight: 400;
}

.hint-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
}

.saved-hint {
  color: #9c958a;
  font-size: 10.5px;
  font-weight: 400;
}

/* Header Action Buttons (2 Characters) */
.detail-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.action-btn-clean {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: #ffffff;
  color: #575047;
  border: 1px solid #dcd6cb;
  padding: 5px 11px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: 0 1px 2px rgba(60, 50, 40, 0.02);
}

.action-btn-clean:hover {
  background: #f8fafc;
  color: #26221d;
  border-color: #beb7aa;
}

.action-btn-danger {
  background: #fee2e2;
  color: #b91c1c;
  border: 1px solid #fecaca;
  padding: 5px 16px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.15s ease;
}

.action-btn-danger:hover {
  background: #fca5a5;
  color: #991b1b;
}

.action-btn-secondary {
  background: #f4efe6;
  color: #4a453e;
  border: 1px solid #dcd6cb;
  padding: 5px 16px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.15s ease;
}

.action-btn-secondary:hover {
  background: #eae4d7;
  color: #26221d;
}

.action-btn-primary {
  background: #2b2724;
  color: #fbf9f5;
  border: none;
  padding: 5px 18px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 450;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: 0 1px 3px rgba(43, 39, 36, 0.15);
}

.action-btn-primary:hover {
  background: #3e3834;
}

.action-btn-primary:active {
  transform: scale(0.98);
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
  background: #f7f4ee;
}

.error-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  color: #b91c1c;
  border-radius: 6px;
  font-size: 11.5px;
  font-family: monospace;
}

/* Cards */
.card {
  background: #ffffff;
  border: 1px solid #e7e2d8;
  border-radius: 9px;
  padding: 13px 16px;
  box-shadow: 0 1px 3px 0 rgba(60, 50, 40, 0.03);
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
  background: #fef3c7;
  color: #d97706;
  border: 1px solid #fde68a;
}

.card-icon.runtime {
  background: #fffbeb;
  color: #b45309;
  border: 1px solid #fef3c7;
}

.card-icon.policy {
  background: #fef7ee;
  color: #c2410c;
  border: 1px solid #ffedd5;
}

.card-title {
  margin: 0;
  font-size: 12.5px;
  font-weight: 500;
  color: #35302b;
}

.card-desc {
  margin: 2px 0 0 0;
  font-size: 10.5px;
  font-weight: 400;
  color: #8c8479;
}

/* Code Editor (Warm Claude Code Dark IDE Box) */
.code-editor-box {
  border-radius: 8px;
  border: 1px solid #383430;
  overflow: hidden;
  background: #1c1917;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
}

.editor-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  background: #24201d;
  border-bottom: 1px solid #34302c;
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
  background: #1c1917;
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
  scrollbar-color: #44403c #1c1917;
}

.code-textarea::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.code-textarea::-webkit-scrollbar-track {
  background: #1c1917;
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
  font-weight: 450;
  color: #35302b;
}

.label-desc {
  font-size: 10.5px;
  font-weight: 400;
  color: #8c8479;
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
  background: #ffffff;
  border: 1px solid #dcd6cb;
  border-radius: 6px;
  padding: 5px 9px;
  color: #35302b;
  font-size: 11.5px;
  font-weight: 400;
  outline: none;
  box-sizing: border-box;
  transition: all 0.15s ease;
}

.select-input {
  cursor: pointer;
}

.text-input:focus,
.select-input:focus {
  border-color: #d97706;
  box-shadow: 0 0 0 3px rgba(217, 119, 6, 0.12);
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
  background: #f4efe6;
  border: 1px solid #e3ddd1;
  color: #575047;
  padding: 4px 8px;
  border-radius: 5px;
  font-size: 10.5px;
  font-weight: 400;
  cursor: pointer;
  transition: all 0.12s ease;
}

.chip-btn:hover {
  background: #eae4d7;
  color: #26221d;
}

.chip-btn.active {
  background: #2b2724;
  border-color: #2b2724;
  color: #fbf9f5;
  font-weight: 450;
  box-shadow: 0 1px 2px rgba(43, 39, 36, 0.1);
}

.input-unit {
  font-size: 11px;
  color: #8c8479;
}

.small-btn {
  background: #ffffff;
  border: 1px solid #dcd6cb;
  color: #4a453e;
  padding: 5px 11px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 400;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.small-btn:hover {
  background: #f5f0e6;
  color: #26221d;
  border-color: #beb7aa;
}

.hint-msg {
  font-size: 10.5px;
  color: #8c8479;
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
  color: #4a453e;
}

/* Horizontal Resizer between Config & Terminal Log */
.resizer-h {
  height: 5px;
  background: #e7e2d8;
  cursor: row-resize;
  position: relative;
  flex-shrink: 0;
  transition: background 0.15s ease;
  z-index: 10;
}

.resizer-h:hover,
.resizer-h:active {
  background: #d97706;
}

.resizer-thumb-h {
  position: absolute;
  left: 50%;
  top: 1px;
  transform: translateX(-50%);
  width: 24px;
  height: 3px;
  border-radius: 2px;
  background: #beb7aa;
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
  overflow-y: auto;
  padding: 8px 16px;
  font-family: "SF Mono", "Fira Code", Menlo, Monaco, Consolas, monospace;
  font-size: 11px;
  font-weight: 400;
  line-height: 1.55;
  color: #e7e5e4;
  user-select: text;
  background: #1c1917;
  scrollbar-width: thin;
  scrollbar-color: #44403c #1c1917;
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
  color: #f59e0b;
  font-size: 10.5px;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  cursor: pointer;
  transition: all 0.15s ease;
  z-index: 10;
  user-select: none;
}

.jump-to-bottom-pill:hover {
  background: #2e2824;
  border-color: #d97706;
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.5);
}
</style>