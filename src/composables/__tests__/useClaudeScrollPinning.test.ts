// @vitest-environment happy-dom
import { describe, it, expect, beforeEach, vi } from "vitest";
import { useClaudeScrollPinning } from "../useClaudeScrollPinning";

describe("useClaudeScrollPinning", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("initializes with isPinned true and showJumpControl false", () => {
    const { isPinned, showJumpControl } = useClaudeScrollPinning();
    expect(isPinned.value).toBe(true);
    expect(showJumpControl.value).toBe(false);
  });

  it("marks user input and unpins with cooldown timestamp", () => {
    const { isPinned, markUserInput } = useClaudeScrollPinning();
    markUserInput();
    expect(isPinned.value).toBe(false);
  });

  it("pins when scroll is within 40px from bottom", () => {
    const { containerRef, isPinned, showJumpControl, handleScroll, markUserInput } = useClaudeScrollPinning();
    const fakeEl = {
      scrollHeight: 1000,
      scrollTop: 970, // 1000 - 970 - 20 = 10px <= 40px
      clientHeight: 20,
      scrollTo: vi.fn(),
    } as unknown as HTMLElement;

    containerRef.value = fakeEl;
    markUserInput();
    expect(isPinned.value).toBe(false);

    handleScroll();
    expect(isPinned.value).toBe(true);
    expect(showJumpControl.value).toBe(false);
  });

  it("shows jump control when distance from bottom is greater than 120px", () => {
    const { containerRef, isPinned, showJumpControl, handleScroll } = useClaudeScrollPinning();
    const fakeEl = {
      scrollHeight: 1000,
      scrollTop: 800, // 1000 - 800 - 50 = 150px > 120px
      clientHeight: 50,
      scrollTo: vi.fn(),
    } as unknown as HTMLElement;

    containerRef.value = fakeEl;
    handleScroll();
    expect(isPinned.value).toBe(false);
    expect(showJumpControl.value).toBe(true);
  });

  it("does not auto-scroll during 150ms user input cooldown", () => {
    const { containerRef, markUserInput, onNewItem } = useClaudeScrollPinning();
    const scrollTo = vi.fn();
    const fakeEl = {
      scrollHeight: 1000,
      scrollTop: 500,
      clientHeight: 100,
      scrollTo,
    } as unknown as HTMLElement;

    containerRef.value = fakeEl;
    markUserInput(); // user just scrolled

    const scrolled = onNewItem();
    expect(scrolled).toBe(false);
    expect(scrollTo).not.toHaveBeenCalled();
  });

  it("auto-scrolls on new item when pinned and past cooldown", () => {
    const { containerRef, isPinned, onNewItem } = useClaudeScrollPinning();
    const scrollTo = vi.fn();
    const fakeEl = {
      scrollHeight: 1000,
      scrollTop: 900,
      clientHeight: 100,
      scrollTo,
    } as unknown as HTMLElement;

    containerRef.value = fakeEl;
    expect(isPinned.value).toBe(true);

    const scrolled = onNewItem();
    expect(scrolled).toBe(true);
    expect(scrollTo).toHaveBeenCalledWith({
      top: 1000,
      behavior: "auto",
    });
  });
});
