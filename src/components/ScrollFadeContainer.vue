<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, type CSSProperties } from "vue";

/**
 * Claude Design System (CDS) - ScrollFadeContainer.vue
 * 逆向还原自官方 Claude Desktop 7 档非线性 Alpha 渐变遮罩 (y ≈ x^1.5 幂律感知曲线)
 */

interface Props {
  orientation?: "vertical" | "horizontal" | "both";
  fadeSize?: number; // 默认 24px
}

const props = withDefaults(defineProps<Props>(), {
  orientation: "vertical",
  fadeSize: 24,
});

const emit = defineEmits<{
  (e: "scroll", event: Event): void;
}>();

const containerRef = ref<HTMLDivElement | null>(null);

const fadeTop = ref(0);
const fadeBottom = ref(0);
const fadeLeft = ref(0);
const fadeRight = ref(0);

function updateScrollFades(targetEl?: HTMLElement | null) {
  const el = targetEl || (containerRef.value?.firstElementChild as HTMLElement) || containerRef.value;
  if (!el) return;

  if (props.orientation === "vertical" || props.orientation === "both") {
    const { scrollTop, scrollHeight, clientHeight } = el;
    const isAtTop = scrollTop <= 2;
    const isAtBottom = scrollTop + clientHeight >= scrollHeight - 2;

    fadeTop.value = isAtTop ? 0 : props.fadeSize;
    fadeBottom.value = isAtBottom ? 0 : props.fadeSize;
  }

  if (props.orientation === "horizontal" || props.orientation === "both") {
    const { scrollLeft, scrollWidth, clientWidth } = el;
    const isAtLeft = scrollLeft <= 2;
    const isAtRight = scrollLeft + clientWidth >= scrollWidth - 2;

    fadeLeft.value = isAtLeft ? 0 : props.fadeSize;
    fadeRight.value = isAtRight ? 0 : props.fadeSize;
  }
}

function handleScroll(e: Event) {
  const target = (e.target as HTMLElement) || containerRef.value;
  updateScrollFades(target);
  emit("scroll", e);
}

const maskStyle = computed<CSSProperties>(() => {
  const styles: Record<string, string> = {
    "--cds-scroll-fade-top": `${fadeTop.value}px`,
    "--cds-scroll-fade-bottom": `${fadeBottom.value}px`,
    "--cds-scroll-fade-left": `${fadeLeft.value}px`,
    "--cds-scroll-fade-right": `${fadeRight.value}px`,
  };

  if (props.orientation === "vertical") {
    const vMask = `linear-gradient(to bottom,
      transparent,
      rgba(0, 0, 0, 0.05) calc(var(--cds-scroll-fade-top) * 0.15),
      rgba(0, 0, 0, 0.15) calc(var(--cds-scroll-fade-top) * 0.30),
      rgba(0, 0, 0, 0.30) calc(var(--cds-scroll-fade-top) * 0.45),
      rgba(0, 0, 0, 0.50) calc(var(--cds-scroll-fade-top) * 0.60),
      rgba(0, 0, 0, 0.70) calc(var(--cds-scroll-fade-top) * 0.75),
      rgba(0, 0, 0, 0.85) calc(var(--cds-scroll-fade-top) * 0.87),
      #000 var(--cds-scroll-fade-top),
      #000 calc(100% - var(--cds-scroll-fade-bottom)),
      rgba(0, 0, 0, 0.85) calc(100% - var(--cds-scroll-fade-bottom) * 0.87),
      rgba(0, 0, 0, 0.70) calc(100% - var(--cds-scroll-fade-bottom) * 0.75),
      rgba(0, 0, 0, 0.50) calc(100% - var(--cds-scroll-fade-bottom) * 0.60),
      rgba(0, 0, 0, 0.30) calc(100% - var(--cds-scroll-fade-bottom) * 0.45),
      rgba(0, 0, 0, 0.15) calc(100% - var(--cds-scroll-fade-bottom) * 0.30),
      rgba(0, 0, 0, 0.05) calc(100% - var(--cds-scroll-fade-bottom) * 0.15),
      transparent
    )`;
    styles.maskImage = vMask;
    styles.WebkitMaskImage = vMask;
  } else if (props.orientation === "horizontal") {
    const hMask = `linear-gradient(to right,
      transparent,
      #000 var(--cds-scroll-fade-left),
      #000 calc(100% - var(--cds-scroll-fade-right)),
      transparent
    )`;
    styles.maskImage = hMask;
    styles.WebkitMaskImage = hMask;
  }

  return styles as CSSProperties;
});

let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  updateScrollFades();
  if (containerRef.value && typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(() => {
      updateScrollFades();
    });
    resizeObserver.observe(containerRef.value);
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});

defineExpose({
  containerRef,
  updateScrollFades,
});
</script>

<template>
  <div
    ref="containerRef"
    class="cds-scroll-fade-container"
    :style="maskStyle"
    @scroll.capture="handleScroll"
  >
    <slot></slot>
  </div>
</template>

<style scoped>
.cds-scroll-fade-container {
  position: relative;
  overflow: auto;
  transition: mask-image 150ms ease, -webkit-mask-image 150ms ease;
  width: 100%;
  height: 100%;
}
</style>
