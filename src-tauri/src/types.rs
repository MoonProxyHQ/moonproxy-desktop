//! 客户端启动参数与代理规则——跨模块共享的数据载体。
//!
//! 同时承担两个角色：
//! 1. `tauri::command` 的入参 / 返回值类型，与前端 `src/types.ts` 对齐
//! 2. `config.rs` 生成 TOML、`process.rs` 启动 frpc 时的数据载体

use serde::{Deserialize, Serialize};

/// 客户端启动参数：与服务商建立连接 + 一组穿透规则。
///
/// `Option<String>` 字段在前端 `toArgs()` 中由空字符串转换而来，后端据此
/// 决定是否写入对应 TOML 字段（详见 `config::build_toml`）。
#[derive(Deserialize, Serialize, Clone)]
pub struct StartArgs {
    /// 内置服务商 id（"builtin:..."）或 "custom"
    #[serde(default)]
    pub provider_id: Option<String>,
    /// 自定义服务商显示名（仅当 `provider_id == "custom"` 时有意义）
    #[serde(default)]
    pub custom_name: Option<String>,
    /// FRP 服务端地址，例如 frp.example.com
    pub server_addr: String,
    /// FRP 服务端端口，例如 7000
    pub server_port: u16,
    /// 客户端与 FRP 服务端建立连接时使用的身份验证密钥
    pub token: Option<String>,
    /// 客户端唯一标识，需在服务端唯一
    pub user: Option<String>,
    /// 要穿透的代理规则列表（TCP/UDP/HTTP/HTTPS）
    pub proxies: Vec<ProxyConfig>,
}

/// 单条代理规则——按 frp 官方各类型 schema 拆分为 enum variant。
///
/// 设计动机：frp v0.69.x 对每种代理类型有独立的 TOML schema：
/// - `tcp` / `udp`：`remotePort` 必填，不接受 `customDomains`
/// - `http` / `https`：`customDomains` 必填，**不接受** `remotePort`
///   （`remotePort` 会让 frpc 报 `json: unknown field "remotePort"`）
///
/// 聚合在同一个扁平结构里会让 `build_toml` / `probe_proxy` / URL 生成路径
/// 都需要按 `proxy_type` 字符串运行期分叉，且无法在编译期排除非法字段。
/// 用 `#[serde(tag = "type")]` 的内部标签 enum 后，frp 不接受的字段在
/// 类型层面就不可能出现在错的 variant 上。
///
/// 序列化形态：`{ "type": "tcp", "name": "...", "local_ip": "...", ... }`
/// 与前端 `ProxyConfig` discriminated union 一一对应。
#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ProxyConfig {
    #[serde(rename = "tcp")]
    Tcp {
        name: String,
        local_ip: String,
        local_port: u16,
        remote_port: u16,
    },
    #[serde(rename = "udp")]
    Udp {
        name: String,
        local_ip: String,
        local_port: u16,
        remote_port: u16,
    },
    #[serde(rename = "http")]
    Http {
        name: String,
        local_ip: String,
        local_port: u16,
        /// 单条代理绑定的公网域名（frp v0.69.x 必填，仅取数组首项）
        custom_domain: String,
    },
    #[serde(rename = "https")]
    Https {
        name: String,
        local_ip: String,
        local_port: u16,
        custom_domain: String,
    },
}
