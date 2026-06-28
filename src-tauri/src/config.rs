//! 客户端配置生成（TOML）与持久化（`tauri-plugin-store`）。

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::types::{ProxyConfig, StartArgs};

const CONFIG_STORE_FILE: &str = "config.store.json";
const KEY_START_ARGS: &str = "start_args";

const FRPC_CONFIG_FILENAME: &str = "frpc.toml";

/// 返回 `app_config_dir/frpc.toml` 路径，必要时创建父目录。
pub fn frpc_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let cfg_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录：{e}"))?;
    std::fs::create_dir_all(&cfg_dir).map_err(|e| e.to_string())?;
    Ok(cfg_dir.join(FRPC_CONFIG_FILENAME))
}

/// 把 `StartArgs` 序列化为 frpc.toml 文本，包含字段合法性校验。
///
/// **顺序约束**：`webServer.*` 必须在 `[[proxies]]` 之前（TOML 数组表语法），
/// 否则 frpc 报 `unknown field "webServer"`。
///
/// **按 variant 分支**：frp v0.69.x 对每种代理类型有独立 schema——
/// TCP/UDP 走 `remotePort`（不接受 `customDomains`），HTTP/HTTPS 走
/// `customDomains`（不接受 `remotePort`，否则报 `unknown field "remotePort"`）。
/// 类型本身已经把字段隔离到对应 variant 上，这里只需 match 拼装。
pub fn build_toml(args: &StartArgs) -> Result<String, String> {
    if args.server_addr.is_empty() || args.server_port == 0 {
        return Err("请填写正确的服务商地址与端口".into());
    }
    if args.proxies.is_empty() {
        return Err("请至少添加一条代理规则".into());
    }

    let mut proxies_toml = String::new();
    for (i, p) in args.proxies.iter().enumerate() {
        proxies_toml.push_str(&build_proxy_toml(p, i)?);
    }

    Ok(format!(
        "serverAddr = \"{}\"\nserverPort = {}\nloginFailExit = false\ntransport.dialServerTimeout = 30\ntransport.heartbeatInterval = 30\ntransport.heartbeatTimeout = 90\nwebServer.addr = \"127.0.0.1\"\nwebServer.port = 7400\nwebServer.user = \"admin\"\nwebServer.password = \"admin\"\n{}\n{}\n{}\n",
        escape_toml(&args.server_addr),
        args.server_port,
        args.token
            .as_deref()
            .map(|t| format!("auth.token = \"{}\"\n", escape_toml(t)))
            .unwrap_or_default(),
        args.user
            .as_deref()
            .map(|u| format!("user = \"{}\"\n", escape_toml(u)))
            .unwrap_or_default(),
        proxies_toml,
    ))
}

/// TOML 字符串值转义：处理 `\` 与 `"`，并把控制字符（`\n` `\r` `\t`）
/// 替换为空格——避免用户在名称字段中粘入换行 / Tab 破坏 frpc.toml 解析。
fn escape_toml(s: &str) -> String {
    s.replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// 把单条代理规则拼成 frp `[[proxies]]` TOML 段。
///
/// 字段合法性先于此处校验（名称非空 / 端口非零 / http-https 必须至少一个域名）；
/// 类型 schema 隔离已由 `ProxyConfig` enum 在反序列化阶段保证。
fn build_proxy_toml(p: &ProxyConfig, idx: usize) -> Result<String, String> {
    let n = idx + 1;
    match p {
        ProxyConfig::Tcp {
            name,
            local_ip,
            local_port,
            remote_port,
        } => build_port_proxy("tcp", name, local_ip, *local_port, *remote_port, n),
        ProxyConfig::Udp {
            name,
            local_ip,
            local_port,
            remote_port,
        } => build_port_proxy("udp", name, local_ip, *local_port, *remote_port, n),
        ProxyConfig::Http {
            name,
            local_ip,
            local_port,
            custom_domain,
        } => build_domain_proxy("http", name, local_ip, *local_port, custom_domain, n),
        ProxyConfig::Https {
            name,
            local_ip,
            local_port,
            custom_domain,
        } => build_domain_proxy("https", name, local_ip, *local_port, custom_domain, n),
    }
}

/// TCP / UDP 代理 TOML 段：frp 接受 `remotePort`。
fn build_port_proxy(
    kind: &str,
    name: &str,
    local_ip: &str,
    local_port: u16,
    remote_port: u16,
    n: usize,
) -> Result<String, String> {
    if name.trim().is_empty() || local_ip.trim().is_empty() {
        return Err(format!("第 {n} 条代理：名称 / 本地地址不能为空"));
    }
    if local_port == 0 || remote_port == 0 {
        return Err(format!("第 {n} 条代理：端口不能为 0"));
    }
    Ok(format!(
        "[[proxies]]\nname = \"{}\"\ntype = \"{}\"\nlocalIP = \"{}\"\nlocalPort = {}\nremotePort = {}\n\n",
        escape_toml(name),
        kind,
        escape_toml(local_ip),
        local_port,
        remote_port,
    ))
}

/// HTTP / HTTPS 代理 TOML 段：frp 接受 `customDomains`（单值数组），**不接受** `remotePort`。
fn build_domain_proxy(
    kind: &str,
    name: &str,
    local_ip: &str,
    local_port: u16,
    custom_domain: &str,
    n: usize,
) -> Result<String, String> {
    if name.trim().is_empty() || local_ip.trim().is_empty() {
        return Err(format!("第 {n} 条代理：名称 / 本地地址不能为空"));
    }
    if local_port == 0 {
        return Err(format!("第 {n} 条代理：本地端口不能为 0"));
    }
    let domain = custom_domain.trim();
    if domain.is_empty() {
        return Err(format!(
            "第 {n} 条代理：HTTP/HTTPS 代理必须填写自定义域名"
        ));
    }
    if !is_valid_domain(domain) {
        return Err(format!(
            "第 {n} 条代理：自定义域名格式不合法（{}）",
            domain
        ));
    }
    Ok(format!(
        "[[proxies]]\nname = \"{}\"\ntype = \"{}\"\nlocalIP = \"{}\"\nlocalPort = {}\ncustomDomains = [\"{}\"]\n\n",
        escape_toml(name),
        kind,
        escape_toml(local_ip),
        local_port,
        escape_toml(domain),
    ))
}

/// 域名格式校验：必须形如 `label1.label2...`，每段字母/数字/连字符，
/// 不以连字符开头或结尾；至少两个 label（顶级 + 主域）。
///
/// 仅做形态校验，不验证 DNS 是否真实存在——避免依赖外部网络。
fn is_valid_domain(s: &str) -> bool {
    if s.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = s.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    labels
        .iter()
        .all(|label| is_valid_label(label))
}

fn is_valid_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }
    let bytes = label.as_bytes();
    if !(bytes[0].is_ascii_alphanumeric()) {
        return false;
    }
    if !(bytes[bytes.len() - 1].is_ascii_alphanumeric()) {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

/// 保存客户端配置到 `tauri-plugin-store`（`config.store.json`）。
#[tauri::command]
pub fn save_config(app: AppHandle, args: StartArgs) -> Result<(), String> {
    let store = app
        .store(CONFIG_STORE_FILE)
        .map_err(|e| format!("无法访问配置存储：{e}"))?;
    let value = serde_json::to_value(&args).map_err(|e| format!("序列化配置失败：{e}"))?;
    store.set(KEY_START_ARGS, value);
    store.save().map_err(|e| format!("保存配置失败：{e}"))
}

/// 读取已保存的客户端配置。
#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<Option<StartArgs>, String> {
    let store = app
        .store(CONFIG_STORE_FILE)
        .map_err(|e| format!("无法访问配置存储：{e}"))?;
    let Some(value) = store.get(KEY_START_ARGS) else {
        return Ok(None);
    };
    let args: StartArgs =
        serde_json::from_value(value).map_err(|e| format!("解析配置失败：{e}"))?;
    Ok(Some(args))
}
