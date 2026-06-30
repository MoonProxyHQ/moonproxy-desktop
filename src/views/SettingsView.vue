<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import InterfaceTab from "../components/settings/InterfaceTab.vue";
import LaunchTab from "../components/settings/LaunchTab.vue";
import UpdatesTab from "../components/settings/UpdatesTab.vue";
import LogsTab from "../components/settings/LogsTab.vue";
import AboutTab from "../components/settings/AboutTab.vue";

defineEmits<{ back: [] }>();

type TabKey = "interface" | "launch" | "updates" | "logs" | "about";

const { t: $t } = useI18n();

const tabs: { key: TabKey; labelKey: string }[] = [
  { key: "launch", labelKey: "settings_tab_launch" },
  { key: "interface", labelKey: "settings_tab_interface" },
  { key: "updates", labelKey: "settings_tab_updates" },
  { key: "logs", labelKey: "settings_tab_logs" },
  { key: "about", labelKey: "settings_tab_about" },
];

const activeTab = ref<TabKey>("launch");
</script>

<template>
  <div class="settings-view">
    <div class="segmented-bar">
      <div class="segmented">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="seg-btn"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ $t(tab.labelKey) }}
        </button>
      </div>
    </div>

    <div class="settings-body">
      <LaunchTab v-if="activeTab === 'launch'" />
      <InterfaceTab v-else-if="activeTab === 'interface'" />
      <UpdatesTab v-else-if="activeTab === 'updates'" />
      <LogsTab v-else-if="activeTab === 'logs'" />
      <AboutTab v-else-if="activeTab === 'about'" />
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.segmented-bar {
  padding: 10px 14px 6px;
  display: flex;
  justify-content: center;
}
.segmented {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  background: hsl(var(--muted));
  padding: 3px;
  border-radius: var(--radius);
  gap: 2px;
  max-width: 100%;
}
.seg-btn {
  padding: 5px 14px;
  border-radius: calc(var(--radius) - 3px);
  font-size: 12px;
  font-weight: 500;
  background: transparent;
  border: none;
  color: hsl(var(--muted-foreground));
  transition: color 0.15s, background-color 0.15s, box-shadow 0.15s;
}
.seg-btn:hover:not(.active) {
  color: hsl(var(--foreground));
}
.seg-btn.active {
  background: hsl(var(--background));
  color: hsl(var(--foreground));
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}
.settings-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  overscroll-behavior: none; /* 禁用滚动链与触底回弹 */
  padding: 8px 14px 14px;
}
</style>
