// @vitest-environment happy-dom
import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  themeSetting,
  resolvedTheme,
  setThemeSetting,
  initTheme,
  getSystemTheme,
} from "../useTheme";

describe("useTheme composable", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = "";
    document.documentElement.removeAttribute("data-theme");
    vi.restoreAllMocks();
  });

  it("defaults to auto setting and detects system theme", () => {
    // Mock matchMedia returning dark = false
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    setThemeSetting("auto");
    expect(themeSetting.value).toBe("auto");
    expect(getSystemTheme()).toBe("light");
    expect(resolvedTheme.value).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("switches explicitly to dark mode", () => {
    setThemeSetting("dark");
    expect(themeSetting.value).toBe("dark");
    expect(resolvedTheme.value).toBe("dark");
    expect(localStorage.getItem("launcher_theme_setting")).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("switches explicitly to light mode", () => {
    setThemeSetting("light");
    expect(themeSetting.value).toBe("light");
    expect(resolvedTheme.value).toBe("light");
    expect(localStorage.getItem("launcher_theme_setting")).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("resolves to dark when system prefers dark in auto mode", () => {
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: true,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    setThemeSetting("auto");
    expect(resolvedTheme.value).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("initializes theme and attaches listener", () => {
    const addEventListener = vi.fn();
    window.matchMedia = vi.fn().mockImplementation((query) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener,
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    initTheme();
    expect(addEventListener).toHaveBeenCalled();
  });

  it("dynamically updates resolvedTheme when system changes theme in auto mode", () => {
    let changeHandler: (() => void) | null = null;
    let isDark = false;

    window.matchMedia = vi.fn().mockImplementation((query) => ({
      get matches() {
        return isDark;
      },
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: (_event: string, handler: () => void) => {
        changeHandler = handler;
      },
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    setThemeSetting("auto");
    initTheme();
    expect(resolvedTheme.value).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);

    // Simulate OS switching to dark mode
    isDark = true;
    if (typeof changeHandler === "function") {
      (changeHandler as () => void)();
    }

    expect(resolvedTheme.value).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });
});
