import { ref } from "vue";

export interface ScrollPinningOptions {
  cooldownMs?: number; // 默认 150ms 用户输入保护窗口
  bottomThreshold?: number; // 默认 40px 底部容差吸附
  jumpThreshold?: number; // 默认 120px 显示回到底部气泡
}

export function useClaudeScrollPinning(options: ScrollPinningOptions = {}) {
  const cooldownMs = options.cooldownMs ?? 150;
  const bottomThreshold = options.bottomThreshold ?? 40;
  const jumpThreshold = options.jumpThreshold ?? 120;

  const containerRef = ref<HTMLElement | null>(null);
  const isPinned = ref(true);
  const showJumpControl = ref(false);
  const lastUserInputAt = ref(0);
  const programmaticScrollUntil = ref(0);

  function markUserInput() {
    lastUserInputAt.value = typeof performance !== "undefined" ? performance.now() : Date.now();
    isPinned.value = false;
  }

  function handleScroll() {
    const now = typeof performance !== "undefined" ? performance.now() : Date.now();
    if (now < programmaticScrollUntil.value) {
      isPinned.value = true;
      showJumpControl.value = false;
      return;
    }

    const el = containerRef.value;
    if (!el) return;

    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    // 40px 底部容差图钉锁定
    isPinned.value = distanceFromBottom <= bottomThreshold;
    // 离底超过 120px 显示跳至底部悬浮按钮
    showJumpControl.value = distanceFromBottom > jumpThreshold;
  }

  function scrollToBottom(smooth = false) {
    const el = containerRef.value;
    if (!el) return;
    const now = typeof performance !== "undefined" ? performance.now() : Date.now();
    // 平滑滚动在动画期间（400ms）持续保持图钉锁定状态，瞬时滚动设置 60ms 吸收微任务事件
    programmaticScrollUntil.value = now + (smooth ? 400 : 60);

    if (typeof el.scrollTo === "function") {
      el.scrollTo({
        top: el.scrollHeight,
        behavior: smooth ? "smooth" : "auto",
      });
    } else {
      el.scrollTop = el.scrollHeight;
    }
    isPinned.value = true;
    showJumpControl.value = false;
  }

  function onNewItem(force = false): boolean {
    const now = typeof performance !== "undefined" ? performance.now() : Date.now();
    // 150ms 冷却保护窗口：用户刚触摸/滚轮时不抢夺视口
    if (!force && now - lastUserInputAt.value <= cooldownMs) {
      return false;
    }
    if (force || isPinned.value) {
      scrollToBottom(false);
      return true;
    }
    return false;
  }

  return {
    containerRef,
    isPinned,
    showJumpControl,
    markUserInput,
    handleScroll,
    scrollToBottom,
    onNewItem,
  };
}
