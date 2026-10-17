<script setup lang="ts">
/**
 * Claude Design System (CDS) - CdsStatusDot.vue
 * 逆向还原自官方 Claude Desktop 状态指示灯与脉冲动效
 * 支持：实时绿光脉冲 (1.2s)、陶土明度呼吸 (2.4s)、静止灰/红
 */

interface Props {
  status?: "running" | "stopped" | "failed" | "breathe";
  label?: string;
  pulse?: boolean;
}

withDefaults(defineProps<Props>(), {
  status: "stopped",
  pulse: true,
});
</script>

<template>
  <div class="cds-status-wrapper" :class="`status-${status}`">
    <!-- 指示灯核心容器 -->
    <div class="dot-beacon-box">
      <!-- 运行态扩散脉冲光环 (1.2s) -->
      <span
        v-if="status === 'running' && pulse"
        class="pulse-ring"
        aria-hidden="true"
      />
      <!-- 实体核心圆点 -->
      <span class="core-dot" aria-hidden="true" />
    </div>

    <!-- 文本标签 (可选) -->
    <span v-if="label" class="status-label">{{ label }}</span>
  </div>
</template>

<style scoped>
.cds-status-wrapper {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  user-select: none;
  -webkit-user-select: none;
}

.dot-beacon-box {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 10px;
  height: 10px;
  flex-shrink: 0;
}

.core-dot {
  position: relative;
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 999px;
  transition: background-color 200ms ease, box-shadow 200ms ease;
  z-index: 2;
}

/* 1. 运行态：双环绿光扩散脉冲 */
.status-running .core-dot {
  background-color: #10b981;
  box-shadow: 0 0 4px rgba(16, 185, 129, 0.4);
}

.dark .status-running .core-dot,
[data-theme="dark"] .status-running .core-dot {
  background-color: #34d399;
  box-shadow: 0 0 6px rgba(52, 211, 153, 0.5);
}

.pulse-ring {
  position: absolute;
  top: 0;
  left: 0;
  width: 10px;
  height: 10px;
  border-radius: 999px;
  background-color: #34d399;
  opacity: 0.75;
  animation: dframe-dot-pulse 1.2s cubic-bezier(0, 0, 0.2, 1) infinite;
  pointer-events: none;
  z-index: 1;
}

/* 2. 呼吸态：陶土明度呼吸 (2.4s) */
.status-breathe .core-dot {
  background-color: var(--cds-clay, #d97757);
  animation: dframe-clay-breathe 2.4s infinite ease-in-out;
}

/* 3. 停止态：静止中性灰 */
.status-stopped .core-dot {
  background-color: #a8a29e;
}

.dark .status-stopped .core-dot,
[data-theme="dark"] .status-stopped .core-dot {
  background-color: #78716c;
}

/* 4. 异常失败态：警告红 */
.status-failed .core-dot {
  background-color: var(--cds-fill-danger, #ef4444);
  box-shadow: 0 0 4px rgba(239, 68, 68, 0.4);
}

/* 状态文本 */
.status-label {
  font-size: 11.5px;
  font-weight: 500;
  line-height: 1;
  color: var(--cds-text-secondary, #525252);
  white-space: nowrap;
}

.status-running .status-label {
  color: #059669;
}

.dark .status-running .status-label,
[data-theme="dark"] .status-running .status-label {
  color: #34d399;
}

.status-failed .status-label {
  color: #dc2626;
}

.dark .status-failed .status-label,
[data-theme="dark"] .status-failed .status-label {
  color: #f87171;
}
</style>
