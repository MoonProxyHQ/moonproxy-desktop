<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Line } from "vue-chartjs";
import type { Chart } from "chart.js";
import {
  CategoryScale,
  Chart as ChartJS,
  Filler,
  LineElement,
  LinearScale,
  PointElement,
  Tooltip,
  type ChartData,
  type ChartOptions,
} from "chart.js";

import {
  formatBytes,
  formatRate,
  latestTraffic,
  totalInBytes,
  totalOutBytes,
  trafficHistory,
} from "../../composables/useTraffic";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Filler,
  Tooltip
);

const { t } = useI18n();

// 图表数据：60 秒滚动窗口，每秒一格；初始填 0 让曲线从左铺到右
const labels = computed(() => {
  const len = trafficHistory.value.length;
  return Array.from({ length: Math.max(len, 1) }, (_, i) => i);
});

const chartData = computed<ChartData<"line">>(() => {
  const hist = trafficHistory.value;
  const inData = hist.map((s) => s.in_rate);
  const outData = hist.map((s) => s.out_rate);
  return {
    labels: labels.value,
    datasets: [
      {
        label: t("home_traffic_up"),
        data: inData,
        borderColor: "hsl(142 71% 45%)",
        backgroundColor: "hsla(142, 71%, 45%, 0.12)",
        borderWidth: 1.5,
        tension: 0.35,
        fill: true,
        pointRadius: 0,
      },
      {
        label: t("home_traffic_down"),
        data: outData,
        borderColor: "hsl(217 91% 60%)",
        backgroundColor: "hsla(217, 91%, 60%, 0.12)",
        borderWidth: 1.5,
        tension: 0.35,
        fill: true,
        pointRadius: 0,
      },
    ],
  };
});

const chartOptions: ChartOptions<"line"> = {
  responsive: true,
  maintainAspectRatio: false,
  animation: false,
  plugins: {
    tooltip: {
      enabled: false,
    },
  },
  scales: {
    x: { display: false },
    y: { display: false, beginAtZero: true },
  },
  elements: { line: { capBezierPoints: true } },
};

const connText = computed(() => `${latestTraffic.value.connections}`);
const upText = computed(() => formatRate(latestTraffic.value.in_rate));
const downText = computed(() => formatRate(latestTraffic.value.out_rate));
const totalText = computed(
  () => `${formatBytes(totalInBytes.value)} / ${formatBytes(totalOutBytes.value)}`
);

// Chart.js 在 v-if 视图切换重新挂载时，new Chart() 那一帧 flex 布局可能还没
// 完成，会读到错误的容器高度，导致 canvas 用默认 aspect ratio 撑大、溢出卡片。
// 三道兜底：CSS（min-height:0 + overflow:hidden + canvas max-size）+
// nextTick 主动 resize + ResizeObserver 持续监听容器尺寸变化。
const chartRef = ref<{ chart: Chart | null } | null>(null);
const wrapRef = ref<HTMLElement | null>(null);
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  // nextTick + rAF 等浏览器完成 layout 再 resize，避免读到 flex 分配前的临时尺寸
  nextTick(() =>
    requestAnimationFrame(() => chartRef.value?.chart?.resize())
  );
  if (!wrapRef.value || typeof ResizeObserver === "undefined") return;
  resizeObserver = new ResizeObserver(() => {
    chartRef.value?.chart?.resize();
  });
  resizeObserver.observe(wrapRef.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <div class="traffic-card">
    <div class="metrics">
      <div class="metric">
        <div class="metric-label">{{ $t("home_traffic_connections") }}</div>
        <div class="metric-value">{{ connText }}</div>
      </div>
      <div class="metric">
        <div class="metric-label">{{ $t("home_traffic_up") }}</div>
        <div class="metric-value up">{{ upText }}</div>
      </div>
      <div class="metric">
        <div class="metric-label">{{ $t("home_traffic_down") }}</div>
        <div class="metric-value down">{{ downText }}</div>
      </div>
      <div class="metric metric-total">
        <div class="metric-label">{{ $t("home_traffic_total") }}</div>
        <div class="metric-value small">{{ totalText }}</div>
      </div>
    </div>
    <div class="chart-wrap" ref="wrapRef">
      <Line ref="chartRef" :data="chartData" :options="chartOptions" />
    </div>
  </div>
</template>

<style scoped>
.traffic-card {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  padding: 8px 10px;
  min-width: 0;
  flex-shrink: 0; /* 固定高度，不参与 flex 收缩，确保始终 130px */
  height: 130px; /* 两行布局：指标行 + 图表行 */
  /* overflow:hidden 兜底：即便 Chart.js 时序异常撑出 canvas，
     也裁切在卡片内部，绝不溢出到下方 ProxyList */
  overflow: hidden;
  background-color: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
  user-select: none;
}

.metrics {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-shrink: 0;
}

.metric {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  line-height: 1.2;
}

.metric-total {
  flex: 1;
  align-items: flex-end;
}

.metric-label {
  font-size: 10px;
  color: hsl(var(--muted-foreground));
}

.metric-value {
  font-size: 13px;
  font-weight: 600;
  color: hsl(var(--foreground));
  font-variant-numeric: tabular-nums;
}

.metric-value.up {
  color: hsl(142 71% 35%);
}

.metric-value.down {
  color: hsl(217 91% 45%);
}

.metric-value.small {
  font-size: 11px;
  font-weight: 500;
}

.chart-wrap {
  flex: 1;
  /* flex column 下默认 min-height:auto 会阻止收缩到内容以下，
     让 Chart.js 拿到错误高度；显式 0 让 flex 正确分配 */
  min-width: 0;
  min-height: 0;
  height: 100%;
  position: relative;
  /* 与上方指标行做视觉分隔：淡灰底 + 内描边。
     box-shadow inset 不占布局空间，不影响 Chart.js 尺寸计算 */
  background-color: hsl(var(--muted-foreground) / 0.05);
  border-radius: calc(var(--radius) - 2px);
  box-shadow: inset 0 0 0 1px hsl(var(--border) / 0.7);
}

/* canvas 自身兜底：Chart.js 直接写 style.width/height，
   万一比容器大也不会撑破 chart-wrap */
.chart-wrap :deep(canvas) {
  max-width: 100%;
  max-height: 100%;
}
</style>
