<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t: $t } = useI18n();
const { toast, showToast } = useToast();

const HOMEPAGE_URL = "https://moonproxy.app";

async function openHomepage() {
  try {
    await openUrl(HOMEPAGE_URL);
  } catch (e) {
    showToast($t("msg_open_link_failed", { err: String(e) }), "error", 3500);
  }
}
</script>

<template>
  <div class="tab-pane">
    <section class="card section-card">
      <div class="section-title">{{ $t("about_section") }}</div>
      <div class="about-grid">
        <span class="label">{{ $t("about_label_app_name") }}</span>
        <span class="about-value">{{ $t("about_value_app_name") }}</span>

        <span class="label">{{ $t("about_label_homepage") }}</span>
        <a
          class="about-value mono about-link"
          :href="HOMEPAGE_URL"
          target="_blank"
          rel="noopener"
          @click.prevent="openHomepage"
        >{{ HOMEPAGE_URL }}</a>

        <span class="label">{{ $t("about_label_maintainer") }}</span>
        <span class="about-value">{{ $t("about_value_maintainer") }}</span>
      </div>
    </section>
    <Toast :toast="toast" />
  </div>
</template>

<style scoped>
.tab-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  gap: 12px;
}
.section-card {
  padding: 14px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}
.about-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 8px 12px;
  align-items: baseline;
}
.about-grid .label {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
}
.about-value {
  font-size: 12px;
  text-align: right;
  word-break: break-all;
}
.about-value.mono {
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-weight: 500;
}
.about-link {
  color: hsl(var(--primary));
  text-decoration: none;
  cursor: pointer;
  transition: opacity 0.15s ease;
}
.about-link:hover {
  opacity: 0.8;
  text-decoration: underline;
}
</style>
