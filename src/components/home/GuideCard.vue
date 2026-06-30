<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { Layers } from "@lucide/vue";

defineEmits<{ services: [] }>();

const { t: $t } = useI18n();
</script>

<template>
  <section class="card guide-card">
    <div class="guide-icon">
      <Layers :size="22" />
    </div>
    <div class="guide-text">
      <div class="guide-title">{{ $t("home_guide_title") }}</div>
      <div class="guide-desc">{{ $t("home_guide_desc") }}</div>
    </div>
    <button class="btn btn-primary btn-sm" @click="$emit('services')">{{ $t("home_guide_btn") }}</button>
  </section>
</template>

<style scoped>
.guide-card {
  padding: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  background-color: hsl(var(--warning) / 0.06);
  border-color: hsl(var(--warning) / 0.25);
}
.guide-icon {
  color: hsl(var(--warning));
  flex-shrink: 0;
}
.guide-text {
  flex: 1;
}
.guide-title {
  font-size: 14px;
  font-weight: 600;
}
.guide-desc {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  margin-top: 2px;
}
</style>
