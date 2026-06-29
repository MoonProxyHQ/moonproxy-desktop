<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { frpcStatus } from "../../state/runtime";
import type { FrpcStatus } from "../../types";

defineProps<{ disabled: boolean }>();
defineEmits<{ click: [] }>();

const { t: $t } = useI18n();

const STATUS_KEYS: Record<FrpcStatus, { label: string; hint: string; aria: string }> = {
  stopped:    { label: "home_btn_stopped",    hint: "home_btn_hint_stopped",    aria: "home_btn_aria_stopped" },
  connecting: { label: "home_btn_connecting", hint: "home_btn_hint_connecting", aria: "home_btn_aria_connecting" },
  connected:  { label: "home_btn_connected",  hint: "home_btn_hint_connected",  aria: "home_btn_aria_connected" },
  error:      { label: "home_btn_error",      hint: "home_btn_hint_error",      aria: "home_btn_aria_error" },
};

const buttonClass = computed(() => `toggle-${frpcStatus.value}`);
const statusLabel = computed(() => $t(STATUS_KEYS[frpcStatus.value].label));
const statusHint  = computed(() => $t(STATUS_KEYS[frpcStatus.value].hint));
const toggleAria  = computed(() => $t(STATUS_KEYS[frpcStatus.value].aria));
</script>

<template>
  <section class="control-bar">
    <button
      class="status-pill"
      :class="buttonClass"
      :disabled="disabled"
      :aria-label="toggleAria"
      @click="$emit('click')"
    >
      <svg
        class="pill-icon"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 256 256"
        fill="none"
        width="18"
        height="18"
        aria-hidden="true"
      >
        <path d="M128.15 15.4738C133.798 15.4738 138.377 20.0527 138.377 25.7012L138.377 66.6096C138.377 72.2581 133.798 76.837 128.15 76.837C122.502 76.837 117.923 72.2581 117.923 66.6096L117.923 25.7012C117.923 20.0527 122.502 15.4738 128.15 15.4738Z" fill="currentColor"/>
        <path d="M128.15 179.042C133.798 179.042 138.377 183.621 138.377 189.269L138.377 230.177C138.377 235.826 133.798 240.405 128.15 240.405C122.502 240.405 117.923 235.826 117.923 230.177L117.923 189.269C117.923 183.621 122.502 179.042 128.15 179.042Z" fill="currentColor"/>
        <path d="M48.3858 48.0749C52.3796 44.0811 58.8551 44.0811 62.8494 48.0749L91.7923 77.0178C95.786 81.0121 95.786 87.4876 91.7923 91.4813C87.7985 95.4751 81.323 95.4751 77.3287 91.4813L48.3858 62.5385C44.3921 58.5442 44.3921 52.0687 48.3858 48.0749Z" fill="currentColor"/>
        <path d="M164.585 164.274C168.579 160.281 175.055 160.281 179.049 164.274L207.992 193.217C211.985 197.212 211.985 203.687 207.992 207.681C203.998 211.675 197.522 211.675 193.528 207.681L164.585 178.738C160.592 174.744 160.592 168.268 164.585 164.274Z" fill="currentColor"/>
        <path d="M15.7847 127.839C15.7847 122.191 20.3636 117.612 26.0121 117.612L66.9205 117.612C72.5689 117.612 77.1479 122.191 77.1479 127.839C77.1479 133.487 72.5689 138.066 66.9205 138.066L26.0121 138.066C20.3636 138.066 15.7847 133.487 15.7847 127.839Z" fill="currentColor"/>
        <path d="M179.353 127.839C179.353 122.191 183.932 117.612 189.58 117.612L230.488 117.612C236.137 117.612 240.716 122.191 240.716 127.839C240.716 133.487 236.137 138.066 230.488 138.066L189.58 138.066C183.932 138.066 179.353 133.487 179.353 127.839Z" fill="currentColor"/>
        <path d="M91.7923 164.274C95.786 168.268 95.786 174.744 91.7923 178.738L62.8494 207.681C58.8551 211.675 52.3796 211.675 48.3858 207.681C44.3921 203.687 44.3921 197.212 48.3858 193.217L77.3287 164.274C81.323 160.281 87.7985 160.281 91.7923 164.274Z" fill="currentColor"/>
        <path d="M207.992 48.0749C211.985 52.0687 211.985 58.5442 207.992 62.5385L179.049 91.4813C175.055 95.4751 168.579 95.4751 164.585 91.4813C160.592 87.4876 160.592 81.0121 164.585 77.0178L193.528 48.0749C197.522 44.0811 203.998 44.0811 207.992 48.0749Z" fill="currentColor"/>
        <ellipse cx="150.881" cy="128.095" rx="18.592" ry="18.485" fill="currentColor"/>
        <ellipse cx="105.522" cy="128.095" rx="18.592" ry="18.485" fill="currentColor"/>
      </svg>
      <span class="pill-label">{{ statusLabel }}</span>
    </button>
    <span class="control-hint">{{ statusHint }}</span>
  </section>
</template>

<style scoped>
.control-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 12px 12px 0;
  padding: 10px 50px 10px 14px;
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
  flex-shrink: 0;
}

.status-pill {
  flex-shrink: 0;
  height: 36px;
  padding: 0 14px 0 12px;
  border-radius: 18px;
  border: none;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: opacity 0.15s, filter 0.15s;
  font-family: inherit;
  color: hsl(var(--primary-foreground));
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.3px;
}
.status-pill:hover:not(:disabled) {
  filter: brightness(1.08);
}
.status-pill:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle-stopped    { background: hsl(var(--muted-foreground)); }
.toggle-connecting { background: hsl(var(--warning)); }
.toggle-connected  { background: hsl(var(--success)); }
.toggle-error      { background: hsl(var(--destructive)); }

.toggle-connected {
  animation: glow-pulse-success 3s ease-in-out infinite;
}
.toggle-connecting {
  animation: glow-pulse-warning 1s ease-in-out infinite;
}

@keyframes glow-pulse-success {
  0%, 100% { box-shadow: 0 0 0 0 hsl(var(--success) / 0.5); }
  50%      { box-shadow: 0 0 0 10px hsl(var(--success) / 0); }
}
@keyframes glow-pulse-warning {
  0%, 100% { box-shadow: 0 0 0 0 hsl(var(--warning) / 0.6); }
  50%      { box-shadow: 0 0 0 6px hsl(var(--warning) / 0); }
}

.pill-icon {
  color: inherit;
  flex-shrink: 0;
}

.control-hint {
  font-size: 11px;
  font-weight: 500;
  color: hsl(var(--muted-foreground));
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>