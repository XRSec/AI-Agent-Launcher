<script setup lang="ts">
import { computed } from "vue";

/**
 * Claude Design System (CDS) - CdsSegmentedControl.vue
 * 逆向还原自官方 Claude Desktop .df-pills[data-segmented]
 * 核心规范：999px 胶囊圆角、2px 几何边距、纯几何等分滑动背景、150ms 阻尼过渡
 */

export interface SegmentedItem {
  label: string;
  value: string | number;
  title?: string;
}

interface Props {
  modelValue: string | number;
  items: SegmentedItem[];
  size?: "sm" | "md";
}

const props = withDefaults(defineProps<Props>(), {
  size: "md",
});

const emit = defineEmits<{
  (e: "update:modelValue", value: string | number): void;
}>();

const activeIndex = computed(() => {
  const idx = props.items.findIndex((item) => item.value === props.modelValue);
  return idx >= 0 ? idx : 0;
});

const thumbStyle = computed(() => {
  const total = props.items.length;
  if (total === 0) return {};
  const widthPercent = 100 / total;
  const leftPercent = activeIndex.value * widthPercent;

  return {
    width: `calc(${widthPercent}% - 4px / ${total})`,
    left: `calc(2px + ${leftPercent}%)`,
  };
});

function selectItem(val: string | number) {
  emit("update:modelValue", val);
}

function handleKeydown(e: KeyboardEvent) {
  if (props.items.length <= 1) return;
  let newIdx = activeIndex.value;

  if (e.key === "ArrowRight" || e.key === "ArrowDown") {
    newIdx = (activeIndex.value + 1) % props.items.length;
    e.preventDefault();
  } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
    newIdx = (activeIndex.value - 1 + props.items.length) % props.items.length;
    e.preventDefault();
  }

  if (newIdx !== activeIndex.value) {
    selectItem(props.items[newIdx].value);
  }
}
</script>

<template>
  <div
    class="cds-segmented-control"
    :class="`size-${size}`"
    role="tablist"
    tabindex="0"
    @keydown="handleKeydown"
  >
    <!-- 滑动背景指示器 -->
    <div
      class="cds-segmented-thumb"
      :style="thumbStyle"
      aria-hidden="true"
    />

    <!-- 分段标签列表 -->
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      role="tab"
      class="cds-segmented-btn"
      :class="{ 'is-active': item.value === modelValue }"
      :title="item.title || item.label"
      :aria-selected="item.value === modelValue"
      @click="selectItem(item.value)"
    >
      <span class="cds-segmented-label">{{ item.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.cds-segmented-control {
  position: relative;
  display: inline-flex;
  align-items: stretch;
  background-color: var(--cds-segmented-track, rgba(0, 0, 0, 0.06));
  border-radius: 999px;
  padding: 2px;
  user-select: none;
  -webkit-user-select: none;
  outline: none;
  transition: background-color 150ms ease;
}

.cds-segmented-control:focus-visible {
  box-shadow: 0 0 0 1.5px var(--cds-clay);
}

.cds-segmented-control.size-sm {
  height: 24px;
}

.cds-segmented-control.size-md {
  height: 28px;
}

/* 几何滑动指示器 */
.cds-segmented-thumb {
  position: absolute;
  top: 2px;
  bottom: 2px;
  background-color: var(--cds-segmented-thumb, #ffffff);
  border-radius: 999px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08), 0 0 0 1px rgba(0, 0, 0, 0.04);
  transition: left 150ms cubic-bezier(0.2, 0, 0, 1), width 150ms cubic-bezier(0.2, 0, 0, 1);
  pointer-events: none;
  z-index: 1;
}

.dark .cds-segmented-thumb,
[data-theme="dark"] .cds-segmented-thumb {
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255, 255, 255, 0.08);
}

/* 分段选项按钮 */
.cds-segmented-btn {
  position: relative;
  z-index: 2;
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  padding: 0 8px;
  border-radius: 999px;
  color: var(--cds-text-secondary, #525252);
  cursor: pointer;
  font-family: inherit;
  font-size: 11.5px;
  font-weight: 500;
  white-space: nowrap;
  transition: color 150ms ease;
  outline: none;
}

.size-sm .cds-segmented-btn {
  font-size: 11px;
  padding: 0 6px;
}

.cds-segmented-btn:hover:not(.is-active) {
  color: var(--cds-text-primary, #151515);
}

.cds-segmented-btn.is-active {
  color: var(--cds-text-primary, #151515);
}

.cds-segmented-label {
  display: inline-block;
  line-height: 1;
}
</style>
