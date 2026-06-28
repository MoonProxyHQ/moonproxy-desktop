//! 代理本地端口连通性探测。
//!
//! 在主页代理行前置一个健康状态点，告诉用户"本地端口是否能联通"，
//! 避免启动 frpc 后才发现穿透失败。

use std::time::Duration;

use serde::Serialize;

use crate::types::ProxyConfig;

#[derive(Serialize, Clone)]
pub struct ProxyHealth {
    /// 与传入代理列表的下标一一对应
    pub index: usize,
    /// 本地端口是否可达
    pub ok: bool,
    /// 给用户看的状态文案
    pub message: String,
}

/// 检测单条代理的本地端口连通性。
///
/// - TCP/HTTP/HTTPS 用 `TcpStream::connect_timeout` 阻塞探测
/// - UDP 用 `UdpSocket::send_to(&[])` 探测链路可写
///
/// 注意：
/// - 多 IP 主机上 `to_socket_addrs` 可能返回多个地址；**任意一个 TCP 连接成功
///   即视为可达**——与 frp 客户端自身解析顺序语义对齐。
/// - UDP 的"探测已发送"**只代表链路可写，不代表对端应用在监听**；这点在
///   `message` 文案里已明确，UI 不要把它当作"应用存活"。
fn probe_proxy(p: &ProxyConfig) -> (bool, String) {
    let (local_ip, local_port) = match p {
        ProxyConfig::Tcp { local_ip, local_port, .. }
        | ProxyConfig::Udp { local_ip, local_port, .. }
        | ProxyConfig::Http { local_ip, local_port, .. }
        | ProxyConfig::Https { local_ip, local_port, .. } => (local_ip.as_str(), *local_port),
    };
    let addr_str = format!("{}:{}", local_ip, local_port);
    let addrs = match std::net::ToSocketAddrs::to_socket_addrs(&addr_str) {
        Ok(iter) => iter.collect::<Vec<_>>(),
        Err(e) => return (false, format!("地址解析失败：{e}")),
    };
    if addrs.is_empty() {
        return (false, "无法解析地址".into());
    }
    let timeout = Duration::from_millis(1500);
    match p {
        ProxyConfig::Tcp { .. } | ProxyConfig::Http { .. } | ProxyConfig::Https { .. } => {
            probe_tcp(&addrs, timeout)
        }
        ProxyConfig::Udp { .. } => probe_udp(&addrs, timeout),
    }
}

/// TCP / HTTP / HTTPS 端口连通性探测：依次尝试解析到的所有地址，任一连接成功即视为可达。
fn probe_tcp(addrs: &[std::net::SocketAddr], timeout: Duration) -> (bool, String) {
    for addr in addrs {
        if std::net::TcpStream::connect_timeout(addr, timeout).is_ok() {
            return (true, "本地端口可达".into());
        }
    }
    (false, "无法连接到本地端口".into())
}

/// UDP 链路可写性探测：能 `send_to` 即视为成功。
///
/// 注意：只代表"链路可写"，**不代表对端应用在监听**。
fn probe_udp(addrs: &[std::net::SocketAddr], timeout: Duration) -> (bool, String) {
    match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(sock) => {
            if sock.set_write_timeout(Some(timeout)).is_err() {
                return (false, "UDP 设置超时失败".into());
            }
            match sock.send_to(&[], addrs[0]) {
                Ok(_) => (true, "UDP 探测已发送".into()),
                Err(e) => (false, format!("UDP 发送失败：{e}")),
            }
        }
        Err(e) => (false, format!("UDP 套接字错误：{e}")),
    }
}

/// 批量检测本地代理端口连通性，所有代理并行探测。
///
/// - `probe_proxy` 内部是阻塞 IO（`connect_timeout` / `UdpSocket`），故必须用
///   `spawn_blocking` 丢到 blocking 线程池；用 `spawn` 会污染异步运行时。
/// - 结果按 `index` 升序排好返回，前端用 `index` 落位即可，不依赖顺序。
#[tauri::command]
pub async fn check_proxies_health(
    proxies: Vec<ProxyConfig>,
) -> Result<Vec<ProxyHealth>, String> {
    let handles: Vec<_> = proxies
        .into_iter()
        .enumerate()
        .map(|(index, p)| {
            tauri::async_runtime::spawn_blocking(move || {
                let (ok, message) = probe_proxy(&p);
                ProxyHealth { index, ok, message }
            })
        })
        .collect();
    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok(r) => results.push(r),
            Err(e) => return Err(format!("检测任务异常退出：{e}")),
        }
    }
    results.sort_by_key(|r| r.index);
    Ok(results)
}
