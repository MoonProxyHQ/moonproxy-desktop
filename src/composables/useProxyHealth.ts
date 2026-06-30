import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

import type { ProxyConfig } from "../types";
import { config } from "../state";

export interface ProxyHealth {
  /** 对应 config.proxies 下标 */
  index: number;
  /** 本地端口是否可达 */
  ok: boolean;
  /** 状态文案，展示在状态点的 title 中 */
  message: string;
}

/** 每条代理本地端口的连通性检测结果，下标与 `config.proxies` 一致；未检测项为 `undefined` */
export const proxyHealth = ref<(ProxyHealth | undefined)[]>([]);

/**
 * 探测所有代理的本地端口连通性并刷新 proxyHealth。
 * - 空配置时直接清空结果。
 * - 失败时仅 console.warn，不抛错：调用方（ProxyList.vue）按指数退避节奏频繁触发，
 *   抛错会刷屏错误条；前端 UI 已经能从"未检测"态给出反馈。
 * - 用 index 落位（而非依赖返回顺序），避免后续并发策略变更影响前端。
 *
 * 注：本函数是无状态命令封装，**退避节奏由调用方调度**（见 ProxyList.vue 的
 * tickHealth：3s→6s→12s→24s，整体稳定后递增、状态翻转或配置变化时回 3s）。
 */
export async function checkProxiesHealth(): Promise<void> {
  if (config.proxies.length === 0) {
    proxyHealth.value = [];
    return;
  }
  try {
    const proxies = config.proxies.map(toProbeArg);
    const results = await invoke<ProxyHealth[]>("check_proxies_health", {
      proxies,
    });
    const map: (ProxyHealth | undefined)[] = new Array(config.proxies.length);
    for (const r of results) {
      if (r.index >= 0 && r.index < map.length) map[r.index] = r;
    }
    proxyHealth.value = map;
  } catch (e) {
    console.warn("[proxy-health] 检测失败", e);
  }
}

/** 单条代理 reactive → 后端 `check_proxies_health` 入参（union 形态）。 */
function toProbeArg(p: ProxyConfig) {
  switch (p.type) {
    case "tcp":
    case "udp":
      return {
        type: p.type,
        name: p.name,
        local_ip: p.local_ip,
        local_port: Number(p.local_port),
        remote_port: Number(p.remote_port),
      };
    case "http":
    case "https":
      return {
        type: p.type,
        name: p.name,
        local_ip: p.local_ip,
        local_port: Number(p.local_port),
        custom_domain: p.custom_domain,
      };
  }
}
