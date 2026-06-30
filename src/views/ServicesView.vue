<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import ProviderTab from "../components/settings/ProviderTab.vue";
import ProxyTab from "../components/settings/ProxyTab.vue";

defineEmits<{ back: [] }>();

type TabKey = "provider" | "proxy";

const { t: $t } = useI18n();

const tabs: { key: TabKey; labelKey: string }[] = [
  { key: "provider", labelKey: "settings_tab_provider" },
  { key: "proxy", labelKey: "settings_tab_proxy" },
];

const activeTab = ref<TabKey>("provider");
</script>

<template>
  <div class="services-view">
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

    <div class="services-body">
      <ProviderTab v-if="activeTab === 'provider'" />
      <ProxyTab v-else-if="activeTab === 'proxy'" />
    </div>
  </div>
</template>

<style scoped>
.services-view {
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
.services-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 8px 14px 14px;
}
</style>
