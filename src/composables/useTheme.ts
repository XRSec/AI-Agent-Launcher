import { ref, computed } from "vue";
import type { ThemeSetting, ResolvedTheme } from "../types";

const STORAGE_KEY = "launcher_theme_setting";

export function getSystemTheme(): ResolvedTheme {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
}

export const themeSetting = ref<ThemeSetting>(
  (typeof localStorage !== "undefined" && (localStorage.getItem(STORAGE_KEY) as ThemeSetting)) || "auto"
);

export const resolvedTheme = computed<ResolvedTheme>(() => {
  if (themeSetting.value === "auto") {
    return getSystemTheme();
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
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, setting);
  }
  applyThemeToDom(resolvedTheme.value);
}

let mediaQueryListenerAttached = false;

export function initTheme() {
  applyThemeToDom(resolvedTheme.value);

  if (typeof window !== "undefined" && window.matchMedia && !mediaQueryListenerAttached) {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (themeSetting.value === "auto") {
        applyThemeToDom(resolvedTheme.value);
      }
    };
    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener("change", handler);
    } else if ((mediaQuery as any).addListener) {
      (mediaQuery as any).addListener(handler);
    }
    mediaQueryListenerAttached = true;
  }
}
