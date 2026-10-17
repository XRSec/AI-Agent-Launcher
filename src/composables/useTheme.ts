import { ref, computed } from "vue";
import type { ThemeSetting, ResolvedTheme } from "../types";

const STORAGE_KEY = "launcher_theme_setting";

export function getSystemTheme(): ResolvedTheme {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
}

export const systemTheme = ref<ResolvedTheme>(getSystemTheme());

export const themeSetting = ref<ThemeSetting>(
  (typeof localStorage !== "undefined" && (localStorage.getItem(STORAGE_KEY) as ThemeSetting)) || "auto"
);

export const resolvedTheme = computed<ResolvedTheme>(() => {
  if (themeSetting.value === "auto") {
    return systemTheme.value;
  }
  return themeSetting.value;
});

export function applyThemeToDom(theme: ResolvedTheme) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.setAttribute("data-theme", theme);
  if (theme === "dark") {
    root.classList.add("dark");
  } else {
    root.classList.remove("dark");
  }
}

export function setThemeSetting(setting: ThemeSetting) {
  themeSetting.value = setting;
  if (setting === "auto") {
    systemTheme.value = getSystemTheme();
  }
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, setting);
  }
  applyThemeToDom(resolvedTheme.value);
}

let activeMediaQuery: MediaQueryList | null = null;
let activeHandler: (() => void) | null = null;

export function initTheme() {
  systemTheme.value = getSystemTheme();
  applyThemeToDom(resolvedTheme.value);

  if (typeof window !== "undefined" && window.matchMedia) {
    if (activeMediaQuery && activeHandler) {
      if (activeMediaQuery.removeEventListener) {
        activeMediaQuery.removeEventListener("change", activeHandler);
      } else if ((activeMediaQuery as any).removeListener) {
        (activeMediaQuery as any).removeListener(activeHandler);
      }
    }

    activeMediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    activeHandler = () => {
      systemTheme.value = getSystemTheme();
      if (themeSetting.value === "auto") {
        applyThemeToDom(resolvedTheme.value);
      }
    };

    if (activeMediaQuery.addEventListener) {
      activeMediaQuery.addEventListener("change", activeHandler);
    } else if ((activeMediaQuery as any).addListener) {
      (activeMediaQuery as any).addListener(activeHandler);
    }
  }
}
