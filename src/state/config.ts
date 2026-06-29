import { reactive } from "vue";

import type { FrpcConfig, ProxyConfig } from "../types";

/** 用户配置：服务商 + 代理规则。由 `commands/config.ts` 从 `config.store.json` 加载 / 写回。 */
export const config = reactive<FrpcConfig>({
  custom_name: "",
  server_addr: "",
  server_port: 7000,
  token: "",
  user: "",
  proxies: [],
});

/** 是否已完成初始配置（有服务端地址且至少一条代理）。 */
export function isConfigured(): boolean {
  return (
    config.server_addr.trim().length > 0 &&
    config.server_port > 0 &&
    config.proxies.length > 0
  );
}

/**
 * 序列化为 Rust 端 StartArgs 格式：trim / Number 化，空字符串 → null。
 *
 * `proxies` 按 `type` 分支输出对应字段——TCP/UDP 走 `remote_port`，
 * HTTP/HTTPS 走 `custom_domains`。`ProxyConfig` union 在编译期保证分支合法，
 * 但 React 的 reactive 容器把 union 拍扁为 widest type，所以这里仍需显式 switch。
 */
export function toArgs() {
  return {
    custom_name: (config.custom_name ?? "").trim() || null,
    server_addr: (config.server_addr ?? "").trim(),
    server_port: Number(config.server_port),
    token: (config.token ?? "").trim() || null,
    user: (config.user ?? "").trim() || null,
    proxies: config.proxies.map(toProxyArg),
  };
}

/** 单条代理 reactive → Rust 端 ProxyConfig union 序列化形态。 */
function toProxyArg(p: ProxyConfig) {
  switch (p.type) {
    case "tcp":
    case "udp":
      return {
        type: p.type,
        name: p.name.trim(),
        local_ip: p.local_ip.trim(),
        local_port: Number(p.local_port),
        remote_port: Number(p.remote_port),
      };
    case "http":
    case "https":
      return {
        type: p.type,
        name: p.name.trim(),
        local_ip: p.local_ip.trim(),
        local_port: Number(p.local_port),
        custom_domain: p.custom_domain.trim(),
      };
  }
}
