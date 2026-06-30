<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Power, Clock } from "@lucide/vue";

import { prefs } from "../../state";

const { t: $t } = useI18n();

const WEEKDAY_KEYS = [
  "home_weekday_mon",
  "home_weekday_tue",
  "home_weekday_wed",
  "home_weekday_thu",
  "home_weekday_fri",
  "home_weekday_sat",
  "home_weekday_sun",
];

/** 定时连接状态摘要（只读展示用）：未开启 / 每天 / 显式列出星期 + 起止时间。 */
const scheduleSummary = computed(() => {
  const s = prefs.schedule;
  if (!s.enabled) return $t("home_status_schedule_off");
  const days = s.weekdays
    .map((on, i) => (on ? $t(WEEKDAY_KEYS[i]) : null))
    .filter((v): v is string => v !== null);
  const dayPart = days.length === 7 ? $t("home_status_schedule_everyday") : days.join(" / ");
  return `${dayPart} ${s.start_time}-${s.stop_time}`;
});
</script>

<template>
  <section class="system-status">
    <div class="status-item">
      <Power :size="13" class="status-icon" />
      <span class="status-key">{{ $t("home_status_auto_launch") }}</span>
      <span
        class="status-val"
        :class="prefs.auto_launch ? 'on' : 'off'"
      >{{ prefs.auto_launch ? $t("home_status_auto_launch_on") : $t("home_status_auto_launch_off") }}</span>
    </div>
    <div class="status-item">
      <Clock :size="13" class="status-icon" />
      <span class="status-key">{{ $t("home_status_schedule") }}</span>
      <span
        class="status-val"
        :class="prefs.schedule.enabled ? 'on' : 'off'"
      >{{ scheduleSummary }}</span>
    </div>
  </section>
</template>

<style scoped>
.system-status {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 14px 12px;
  border-top: 1px solid hsl(var(--border) / 0.6);
  background: hsl(var(--secondary) / 0.25);
}
.status-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  line-height: 1.5;
}
.status-icon {
  color: hsl(var(--muted-foreground));
  flex-shrink: 0;
}
.status-key {
  color: hsl(var(--muted-foreground));
}
.status-val {
  margin-left: auto;
  font-weight: 500;
  user-select: text;
  -webkit-user-select: text;
}
.status-val.on {
  color: hsl(var(--success));
}
.status-val.off {
  color: hsl(var(--muted-foreground));
}
</style>
