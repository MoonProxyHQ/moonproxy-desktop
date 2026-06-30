<script setup lang="ts">
import { ref, shallowReactive, watch, nextTick, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { Trash2, ExternalLink } from "@lucide/vue";
import { openLogs } from "../../composables/useLogsWindow";

interface LogEntry {
  stream: string;
  line: string;
}

const { t: $t } = useI18n();

const LOG_BUFFER_LIMIT = 500;

const logBox = ref<HTMLDivElement | null>(null);
const logs = shallowReactive<LogEntry[]>([]);
let unlistenLog: UnlistenFn | null = null;

const ANSI_COLORS: Record<string, string> = {
  "30": "#5c6370", "31": "#e06c75", "32": "#98c379", "33": "#e5c07b",
  "34": "#61afef", "35": "#c678dd", "36": "#56b6c2", "37": "#abb2bf",
  "90": "#5c6370", "91": "#f48882", "92": "#b5e890", "93": "#ffd87a",
  "94": "#84bfff", "95": "#d8a9ff", "96": "#4fd6e9", "97": "#ffffff",
};

const CSI_RE = /\x1b\[([\d;]*)([A-Za-z])/;

function ansiToHtml(text: string): string {
  if (!text) return "";
  let html = "";
  let i = 0;
  let color: string | undefined;
  let bold = false;
  let inSpan = false;

  const closeSpan = () => {
    if (inSpan) { html += "</span>"; inSpan = false; }
  };
  const openSpan = () => {
    const styles: string[] = [];
    if (color) styles.push(`color:${color}`);
    if (bold) styles.push("font-weight:600");
    if (styles.length) {
      html += `<span style="${styles.join(";")}">`;
      inSpan = true;
    }
  };

  while (i < text.length) {
    const m = CSI_RE.exec(text.slice(i));
    if (m) {
      if (m[2] === "m") {
        closeSpan();
        const codes = m[1] ? m[1].split(";").filter(Boolean) : ["0"];
        for (const c of codes) {
          if (c === "0") { color = undefined; bold = false; }
          else if (c === "1") bold = true;
          else if (ANSI_COLORS[c]) color = ANSI_COLORS[c];
        }
        openSpan();
      }
      i += m[0].length;
      continue;
    }
    const ch = text[i];
    if (ch === "&") html += "&amp;";
    else if (ch === "<") html += "&lt;";
    else if (ch === ">") html += "&gt;";
    else if (ch !== "\x1b") html += ch;
    i++;
  }
  closeSpan();
  return html;
}

function clearLogs() {
  logs.length = 0;
}

function logClass(stream: string): string {
  if (stream === "stderr") return "log-err";
  if (stream === "system") return "log-sys";
  return "";
}

function scrollToBottom() {
  if (logBox.value) logBox.value.scrollTop = logBox.value.scrollHeight;
}

onMounted(async () => {
  try {
    const history = await invoke<LogEntry[]>("get_logs");
    if (history?.length) {
      logs.push(...history);
    }
  } catch (e) {
    console.warn("[logs-tab] 拉取历史失败", e);
  }
  unlistenLog = await listen<LogEntry>("frpc://log", (event) => {
    logs.push(event.payload);
    while (logs.length > LOG_BUFFER_LIMIT) logs.shift();
  });
  await nextTick();
  scrollToBottom();
});

onUnmounted(() => {
  unlistenLog?.();
});

watch(
  () => logs.length,
  () => nextTick(scrollToBottom)
);
</script>

<template>
  <div class="tab-pane">
    <section class="card section-card logs-card">
      <header class="logs-tab-header">
        <span class="logs-tab-title">{{ $t("logs_section_title") }}</span>
        <div class="logs-tab-meta">
          <span class="log-count" v-if="logs.length">{{ logs.length }}</span>
          <button
            class="btn btn-ghost btn-sm btn-icon"
            @click="openLogs"
            :title="$t('logs_open_external')"
            :aria-label="$t('logs_open_external')"
          >
            <ExternalLink :size="14" />
          </button>
          <button
            class="btn btn-ghost btn-sm btn-icon"
            @click="clearLogs"
            v-if="logs.length"
            :title="$t('logs_clear')"
            :aria-label="$t('logs_clear')"
          >
            <Trash2 :size="14" />
          </button>
        </div>
      </header>
      <div ref="logBox" class="logbox">
        <div
          v-for="(l, i) in logs"
          :key="i"
          class="logline"
          :class="logClass(l.stream)"
          v-html="ansiToHtml(l.line)"
        ></div>
        <div v-if="logs.length === 0" class="log-empty">{{ $t("logs_empty") }}</div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.tab-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.section-card {
  padding: 14px;
}
.logs-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.logs-tab-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  font-size: 13px;
  font-weight: 600;
  flex-shrink: 0;
  user-select: none;
}
.logs-tab-title {
  color: hsl(var(--foreground));
}
.logs-tab-meta {
  display: flex;
  align-items: center;
  gap: 4px;
}
.log-count {
  font-size: 11px;
  color: hsl(var(--muted-foreground));
  background: hsl(var(--muted));
  padding: 1px 7px;
  border-radius: 9999px;
}
.logbox {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 10px 12px;
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
  background: hsl(var(--muted) / 0.3);
  border-radius: var(--radius);
  overscroll-behavior: none;
}
.logline {
  white-space: pre-wrap;
  word-break: break-all;
  color: hsl(var(--foreground));
  contain: layout paint style;
}
.logline.log-err {
  color: hsl(var(--destructive));
}
.logline.log-sys {
  color: hsl(var(--primary));
}
.log-empty {
  color: hsl(var(--muted-foreground));
  text-align: center;
  padding: 12px;
  font-style: italic;
}
</style>
