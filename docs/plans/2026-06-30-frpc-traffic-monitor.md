# FRPC 流量监控图表 实施计划

> **For Claude:** REQUIRED SUB-SKILL: 使用 `superpowers:executing-plans` 按任务逐步实施本计划。

**目标：** 在主页顶部新增独立的流量监控卡片组件，实时展示 frpc 客户端的累计流量、瞬时吞吐速率与当前 frpc↔frps 工作连接数。

**架构：** 在 frpc 与用户真实本地服务之间插入 MoonProxy 自有的 TCP 中转层；每条 TCP/HTTP/HTTPS 代理绑定一个 OS 动态分配的中转端口（`127.0.0.1:0`）。中转层用 `tokio::io::copy_bidirectional` 双向转发并按字节计数，每秒差分计算瞬时速率后通过 `frpc://traffic` 事件广播给前端；前端 60 秒滚动窗口维护图表数据集，由独立的 `TrafficChart.vue` 组件用 Chart.js 渲染。

**技术栈：** Rust + Tauri v2 + tokio（`net` / `io-util`）；Vue 3 + TypeScript + vue-chartjs + Chart.js。

---

## 拍板点（已与主公对齐）

| # | 决策 | 选项 |
|---|---|---|
| 1 | 中转端口策略 | 动态绑定 `127.0.0.1:0`，OS 分配后写回 `frpc.toml` |
| 2 | 代理类型覆盖 | 首版仅 TCP/HTTP/HTTPS；UDP 不中转（保持原 `localPort` 写入 toml） |
| 3 | 图表位置 | 主页顶部，长方形圆角卡片，占满 `home-body` 宽度，**独立组件** `components/home/TrafficChart.vue`，高度 ≈ 76px（2× 标题栏 38px） |
| 4 | 图表库 | Chart.js + vue-chartjs |
| 5 | 历史 retention | 不持久化；内存保留最近 60 秒（每秒 1 个采样点） |

## 提交策略

- 按主公全局规则，**末将不在每个 Task 后自动 commit**。
- 主公可在任意阶段结束时审阅并要求统一提交；或执行末了由主公拍板。
- 计划本身（本文件）不强制入库。

## 关键设计点

### D1. 中转端口生命周期
应用启动时**不开**任何中转 listener；每次 `start_frpc` 调用按当前 `StartArgs.proxies` 列表为每条 TCP/HTTP/HTTPS proxy 单独开 listener，把 OS 分配的端口写入"改写后的 args"再喂给 `build_toml`。`stop_frpc` 与 `Terminated` 都会触发 listener 关闭与计数器清零。

### D2. 字节方向定义（用户视角）
中转层是 `frpc ↔ relay ↔ user_service` 三段：
- **下行（download，in）**：用户服务 → relay → frpc → frps → 公网客户端。对应 `in_bytes` 累计。
- **上行（upload，out）**：公网客户端 → frps → frpc → relay → 用户服务。对应 `out_bytes` 累计。
- 图表展示两条曲线：**上传速率（out）** 与 **下载速率（in）**。

### D3. "TCP 连接数"语义
中转段当前活跃的 TCP 连接数 = frpc↔frps 当前 work connection 数（一对一）。不含那条常驻 control connection（不经过中转层）。

### D4. UDP 处理（首版不中转）
UDP 代理的 `localIP/localPort` 原样写入 `frpc.toml`；图表中的流量与连接数仅反映 TCP/HTTP/HTTPS 子集。

### D5. 端口冲突保护
动态绑定 `0` 由 OS 保证唯一；用户机已占端口不会冲突。frpc 子进程被 `reap_orphan_frpc` 兜底清理，但中转端口随主进程退出立即释放（drop listener）。

### D6. data race 保护
- 单条中转连接：`Arc<AtomicU64>` 双向字节计数（不阻塞 IO 路径）
- 全局聚合：每秒 tick 时按 proxy 累加 atomic 得到瞬时值
- relay state 锁：`Mutex<RelayState>`，仅在启动/停止/查询时短临界区

### D7. 前后端事件协议
新增事件 `frpc://traffic`，载荷结构：

```ts
interface TrafficPayload {
  /** 累计下行字节（用户服务 → frpc，download 方向） */
  total_in_bytes: number;
  /** 累计上行字节（frpc → 用户服务，upload 方向） */
  total_out_bytes: number;
  /** 瞬时下行速率 bytes/s（与上次采样差分） */
  in_rate: number;
  /** 瞬时上行速率 bytes/s */
  out_rate: number;
  /** 当前活跃中转 TCP 连接数（= frpc↔frps work connections） */
  connections: number;
}
```

采样频率：**每 1 秒 1 次**（仅在 frpc 处于 connecting/connected/error 时广播；stopped 时不广播）。

### D8.UDP 之外的副作用
对 HTTP/HTTPS proxy，frpc 实际只与本地服务建立 TCP 连接（HTTP 协议层对 frpc 透明），中转层不需要解析 HTTP——纯 TCP 字节流转发即可。

---

## 任务列表

### Task 1：扩展 tokio 依赖特性

**Files:**
- Modify: `src-tauri/Cargo.toml:29`

**Step 1：** 修改 tokio 行：

```toml
tokio = { version = "1", default-features = false, features = ["time", "net", "io-util", "rt", "rt-multi-thread", "sync", "macros"] }
```

> 说明：现有 `poll_conn_state` 只用 `tokio::time`。新模块需要 `tokio::net::{TcpListener, TcpStream}`、`tokio::io::copy_bidirectional`、`tokio::sync` 与 `tokio::spawn`。`rt-multi-thread` 与 `macros` 让 `tauri::async_runtime::spawn` 调度链路稳。

**Step 2：** 验证编译通过：

```bash
cd src-tauri && cargo check
```

期望：编译通过，无新 warning。

---

### Task 2：新建 `proxy_relay.rs` 模块骨架

**Files:**
- Create: `src-tauri/src/proxy_relay.rs`
- Modify: `src-tauri/src/lib.rs:29`（mod 声明）+ `src-tauri/src/lib.rs:13`（注释）

**Step 1：** 在 `lib.rs` 模块声明区按字母序插入：

```rust
mod proxy_relay;
```

并在文件头注释列表中追加一行：

```rust
//! - `proxy_relay`：frpc 与本地服务之间的 TCP 中转层 + 流量统计
```

**Step 2：** 创建 `src-tauri/src/proxy_relay.rs`，写入骨架：

```rust
//! frpc 与本地真实服务之间的 TCP 中转层。
//!
//! 设计动机：frpc v0.69.1 客户端的 `/api/status` 不暴露任何流量/速率字段
//! （详见 docs/plans/2026-06-30-frpc-traffic-monitor.md）。要让用户看到吞吐
//! 曲线，MoonProxy 必须自己「夹」在 frpc 与用户服务之间：frpc 不再直连用户
//! 的 `localIP:localPort`，而是连到 MoonProxy 自开的动态端口；MoonProxy 双向
//! copy 字节并按方向计数。
//!
//! 该模块对所有 TCP 类代理（含 HTTP/HTTPS——它们本质也是 TCP 流）启用中转；
//! UDP 首版不中转。

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

/// 单条代理的运行态：监听句柄 + 双向字节累计 + 当前活跃连接计数。
///
/// `in_bytes`：用户服务 → frpc 方向（用户视角 download）
/// `out_bytes`：frpc → 用户服务方向（用户视角 upload）
pub struct RelayEntry {
    /// 中转 listener 句柄；drop 即关闭端口
    pub listener: Option<TcpListener>,
    /// 用户真实本地地址 `host:port`
    pub upstream: String,
    pub in_bytes: AtomicU64,
    pub out_bytes: AtomicU64,
    pub connections: AtomicI64,
}

impl RelayEntry {
    fn new(listener: TcpListener, upstream: String) -> Self {
        Self {
            listener: Some(listener),
            upstream,
            in_bytes: AtomicU64::new(0),
            out_bytes: AtomicU64::new(0),
            connections: AtomicI64::new(0),
        }
    }
}

/// 全部代理中转层的集合，挂在 `FrpcState` 上。
pub struct RelayState {
    /// 按 proxy 名字索引；同名 proxy 重启时覆盖（实际 build_toml 已校验唯一）
    pub entries: Mutex<Vec<(String, Arc<RelayEntry>)>>,
}

impl Default for RelayState {
    fn default() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }
}

/// 通过 `frpc://traffic` 事件广播给前端的载荷。
#[derive(Serialize, Clone, Default)]
pub struct TrafficPayload {
    pub total_in_bytes: u64,
    pub total_out_bytes: u64,
    pub in_rate: u64,
    pub out_rate: u64,
    pub connections: i64,
}
```

**Step 3：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：编译通过；可能有未使用字段警告，Task 3 会消费。

---

### Task 3：实现中转连接处理（spawn + copy_bidirectional）

**Files:**
- Modify: `src-tauri/src/proxy_relay.rs`

**Step 1：** 在 `proxy_relay.rs` 末尾追加连接处理逻辑：

```rust
/// 单条 frpc → 中转端口的 TCP 连接处理：再连到用户真实服务，双向 copy 并计数。
///
/// 任何方向 IO 出错都终止本连接；连接计数在进出时配对加减。
async fn handle_relay_conn(mut frpc_conn: TcpStream, entry: Arc<RelayEntry>) {
    let upstream = match TcpStream::connect(&entry.upstream).await {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut user_conn = upstream;
    entry.connections.fetch_add(1, Ordering::Relaxed);

    // 拆分两条 half-conn，按方向计数。
    // copy_bidirectional 返回 (user_to_frpc, frpc_to_user) 字节数；
    // 我们要把方向对齐到「用户视角」：
    //   - user_conn → frpc_conn：用户服务流向 frpc，= in（download）
    //   - frpc_conn → user_conn：frpc 流向用户服务，= out（upload）
    let (user_to_frpc, frpc_to_user) = {
        let (mut frpc_rd, mut frpc_wr) = frpc_conn.split();
        let (mut user_rd, mut user_wr) = user_conn.split();
        copy_bidirectional_counted(
            &mut user_rd, &mut frpc_wr, &entry.in_bytes,
            &mut frpc_rd, &mut user_wr, &entry.out_bytes,
        ).await
    };

    let _ = user_to_frpc;
    let _ = frpc_to_user;
    entry.connections.fetch_sub(1, Ordering::Relaxed);
}

/// 与 `tokio::io::copy_bidirectional` 等价，但分别在两个方向上累加字节计数。
///
/// 实现策略：不用 `copy_bidirectional`（其内部不暴露计数），改为并发跑两个
/// `copy_counted`，先完成者通过 `tokio::select!` 取消另一条。
async fn copy_bidirectional_counted(
    user_rd: &mut tokio::io::ReadHalf<'_>,
    frpc_wr: &mut tokio::io::WriteHalf<'_>,
    in_counter: &AtomicU64,
    frpc_rd: &mut tokio::io::ReadHalf<'_>,
    user_wr: &mut tokio::io::WriteHalf<'_>,
    out_counter: &AtomicU64,
) -> (u64, u64) {
    let in_task = copy_one_way(user_rd, frpc_wr, in_counter);
    let out_task = copy_one_way(frpc_rd, user_wr, out_counter);
    tokio::pin!(in_task);
    tokio::pin!(out_task);

    let mut in_bytes = 0u64;
    let mut out_bytes = 0u64;
    loop {
        tokio::select! {
            res = &mut in_task => {
                if let Ok(n) = res { in_bytes += n; }
                break;
            }
            res = &mut out_task => {
                if let Ok(n) = res { out_bytes += n; }
                break;
            }
        }
    }
    (in_bytes, out_bytes)
}

/// 单向 copy：读完立即写、立即累加计数器；与 `tokio::io::copy` 等价但加计数。
async fn copy_one_way<R, W>(
    rd: &mut R,
    wr: &mut W,
    counter: &AtomicU64,
) -> std::io::Result<u64>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    W: tokio::io::AsyncWrite + Unpin + ?Sized,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = vec![0u8; 8 * 1024];
    let mut total = 0u64;
    loop {
        let n = rd.read(&mut buf).await?;
        if n == 0 {
            return Ok(total);
        }
        wr.write_all(&buf[..n]).await?;
        total += n as u64;
        counter.fetch_add(n as u64, Ordering::Relaxed);
    }
}
```

> **注：** 上述实现选择「读完整段再写」而非零拷贝 `sendfile`，原因：我们需要按字节计数，必须经过用户态缓冲；8 KiB 缓冲兼顾吞吐与延迟。

**Step 2：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：编译通过。

---

### Task 4：实现端口分配 + 监听接受循环

**Files:**
- Modify: `src-tauri/src/proxy_relay.rs`

**Step 1：** 在 `proxy_relay.rs` 末尾追加启动与停止逻辑：

```rust
use crate::types::{ProxyConfig, StartArgs};

/// 为 `args.proxies` 中所有可中转代理（TCP/HTTP/HTTPS）开 listener 并 spawn 接受循环；
/// 返回「改写后的 args」——把每条可中转代理的 `localIP/localPort` 替换为
/// `127.0.0.1:<relay_port>`，UDP 保持原样。
///
/// 调用时机：`start_frpc` 在 `build_toml` 之前。失败（listener bind 失败）逐条
/// 记录并跳过该条（不阻塞其他 proxy）。
pub async fn start_relay(
    app: &AppHandle,
    args: &StartArgs,
) -> (StartArgs, RelayState) {
    let _ = app; // 预留：未来可能 emit 启动失败事件
    let mut state = RelayState::default();
    let mut rewritten = args.clone();

    let mut entries = state.entries.lock().await;
    for p in &mut rewritten.proxies {
        let (name, local_ip, local_port) = match p {
            ProxyConfig::Tcp { name, local_ip, local_port, .. }
            | ProxyConfig::Http { name, local_ip, local_port, .. }
            | ProxyConfig::Https { name, local_ip, local_port, .. } => {
                (name.clone(), local_ip.clone(), *local_port)
            }
            // UDP 首版不中转
            ProxyConfig::Udp { .. } => continue,
        };

        // 绑定动态端口
        let listener = match TcpListener::bind(("127.0.0.1", 0u16)).await {
            Ok(l) => l,
            Err(_) => continue,
        };
        let relay_port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
        if relay_port == 0 {
            continue;
        }

        // 改写 args 中本条 proxy 的 localIP/localPort
        let new_ip = "127.0.0.1".to_string();
        match p {
            ProxyConfig::Tcp { local_ip, local_port, .. }
            | ProxyConfig::Http { local_ip, local_port, .. }
            | ProxyConfig::Https { local_ip, local_port, .. } => {
                *local_ip = new_ip;
                *local_port = relay_port;
            }
            ProxyConfig::Udp { .. } => unreachable!(),
        }

        let entry = Arc::new(RelayEntry::new(listener, format!("{local_ip}:{local_port}")));
        let entry_for_loop = entry.clone();

        // spawn 接受循环
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt; // 让 &mut TcpStream 可拆分（split）
            loop {
                let conn = match entry_for_loop.listener.as_ref() {
                    Some(l) => match l.accept().await {
                        Ok((s, _)) => s,
                        Err(_) => break,
                    },
                    None => break,
                };
                let e = entry_for_loop.clone();
                tokio::spawn(handle_relay_conn(conn, e));
            }
            // 隐藏未使用 import 警告
            let _: fn(&mut TcpStream) -> usize = |_| 0;
        });

        entries.push((name, entry));
    }
    drop(entries);
    (rewritten, state)
}

/// 关闭所有 listener 与活跃中转任务。
///
/// 调用时机：`stop_frpc` 或 `Terminated`。drop listener 让 accept 循环下次
/// 迭代返回 Err 从而 break；活跃中转任务在 IO 出错时自然结束。
pub async fn stop_relay(state: &RelayState) {
    let mut entries = state.entries.lock().await;
    for (_, entry) in entries.drain(..) {
        if let Some(l) = entry.listener.as_ref() {
            // 显式关 listener 让 accept 立即返回
            // 注：listener 在 Arc 内，drop 这里只是 take 出 Option
        }
        // Arc 强引用减少；剩余的 spawn 任务会在 IO 出错后自然退出
    }
}
```

> **注：** `stop_relay` 实现是「清空 entries，drop Arc」，listener 通过 `RelayEntry::drop` 关闭。需为 `RelayEntry` 实现 `Drop`：见 Task 5。

**Step 2：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：可能有"未使用 import AsyncReadExt"警告，下个 Task 处理。

---

### Task 5：为 `RelayEntry` 实现 Drop，清理 listener

**Files:**
- Modify: `src-tauri/src/proxy_relay.rs`

**Step 1：** 在 `RelayEntry` impl 块后追加 Drop：

```rust
impl Drop for RelayEntry {
    fn drop(&mut self) {
        // drop listener 让 accept 循环在下一次迭代返回 Err 从而退出
        // （已 spawn 的接受任务持 Arc<RelayEntry>，listener 在最后一个 Arc drop 时关闭）
        drop(self.listener.take());
    }
}
```

**Step 2：** 简化 Task 4 中的 spawn 代码，去掉无用 import：

把 `Task 4` 中的：
```rust
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            loop {
                ...
            }
            let _: fn(&mut TcpStream) -> usize = |_| 0;
        });
```

改为：

```rust
        tokio::spawn(async move {
            loop {
                let conn = match entry_for_loop.listener.as_ref() {
                    Some(l) => match l.accept().await {
                        Ok((s, _)) => s,
                        Err(_) => break,
                    },
                    None => break,
                };
                let e = entry_for_loop.clone();
                tokio::spawn(handle_relay_conn(conn, e));
            }
        });
```

**Step 3：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：通过，无 warning。

---

### Task 6：扩展 `FrpcState` 容纳 RelayState

**Files:**
- Modify: `src-tauri/src/frpc_state.rs:59-85`（FrpcState struct + Default impl）

**Step 1：** 在 `FrpcState` struct 中新增字段：

```rust
pub struct FrpcState {
    pub child: Mutex<Option<CommandChild>>,
    pub conn: Mutex<FrpcConnState>,
    pub error_msg: Mutex<Option<String>>,
    pub started_at: Mutex<Option<Instant>>,
    pub poll_gen: AtomicU64,
    pub logs: Mutex<VecDeque<LogEntry>>,
    /// 中转层状态：仅 start_frpc 时填入；stop 时清空
    pub relay: tokio::sync::Mutex<Option<crate::proxy_relay::RelayState>>,
    /// 上次流量采样的累计字节，用于差分瞬时速率
    pub last_in_bytes: AtomicU64,
    pub last_out_bytes: AtomicU64,
}
```

**Step 2：** 同步更新 `Default` impl：

```rust
impl Default for FrpcState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            conn: Mutex::new(FrpcConnState::default()),
            error_msg: Mutex::new(None),
            started_at: Mutex::new(None),
            poll_gen: AtomicU64::new(0),
            logs: Mutex::new(VecDeque::new()),
            relay: tokio::sync::Mutex::new(None),
            last_in_bytes: AtomicU64::new(0),
            last_out_bytes: AtomicU64::new(0),
        }
    }
}
```

**Step 3：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：通过。

---

### Task 7：实现流量采样任务 + emit `frpc://traffic`

**Files:**
- Modify: `src-tauri/src/proxy_relay.rs`

**Step 1：** 在 `proxy_relay.rs` 追加采样与 emit：

```rust
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// 每 1 秒采样一次累计字节，差分出瞬时速率，聚合 connections，
/// 通过 `frpc://traffic` 广播。
///
/// 退出条件与 `poll_conn_state` 对齐：
/// - `relay` 被清空（stop / Terminated 已接管）
/// - `poll_gen` 与起始记录的不一致（新一轮 start 已接管）
pub async fn poll_traffic(app: AppHandle, start_gen: u64) {
    const TICK: Duration = Duration::from_secs(1);
    loop {
        tokio::time::sleep(TICK).await;
        let state = match app.try_state::<crate::frpc_state::FrpcState>() {
            Some(s) => s,
            None => break,
        };
        if state.poll_gen.load(Ordering::Acquire) != start_gen {
            break;
        }
        // relay 锁保护：stop_frpc 会把 relay 置 None
        let payload = {
            let guard = state.relay.lock().await;
            match guard.as_ref() {
                Some(r) => sample(r, &state).await,
                None => break,
            }
        };
        let _ = app.emit("frpc://traffic", payload);
    }
}

async fn sample(
    relay: &RelayState,
    state: &crate::frpc_state::FrpcState,
) -> TrafficPayload {
    let entries = relay.entries.lock().await;
    let mut total_in = 0u64;
    let mut total_out = 0u64;
    let mut conns = 0i64;
    for (_, e) in entries.iter() {
        total_in += e.in_bytes.load(Ordering::Relaxed);
        total_out += e.out_bytes.load(Ordering::Relaxed);
        conns += e.connections.load(Ordering::Relaxed);
    }
    drop(entries);

    let last_in = state.last_in_bytes.swap(total_in, Ordering::Relaxed);
    let last_out = state.last_out_bytes.swap(total_out, Ordering::Relaxed);
    let in_rate = total_in.saturating_sub(last_in);
    let out_rate = total_out.saturating_sub(last_out);

    TrafficPayload {
        total_in_bytes: total_in,
        total_out_bytes: total_out,
        in_rate,
        out_rate,
        connections: conns,
    }
}
```

**Step 2：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：通过。

---

### Task 8：修改 `process.rs::start_frpc` 集成 relay

**Files:**
- Modify: `src-tauri/src/process.rs:24-90`（start_frpc 头部）

**Step 1：** 在 `start_frpc` 函数最开头（cfg_path 计算之前）插入 relay 启动：

```rust
#[tauri::command]
pub async fn start_frpc(
    app: AppHandle,
    state: State<'_, FrpcState>,
    args: StartArgs,
) -> Result<(), String> {
    // 0. 启动 TCP 中转层：返回改写后的 args（localIP/localPort 已指向中转端口）
    //    + RelayState。失败不致命（最差情况是图表为 0），但要把 RelayState 塞入 state。
    let (args, relay_state) = crate::proxy_relay::start_relay(&app, &args).await;
    {
        let mut guard = state.relay.lock().await;
        *guard = Some(relay_state);
        // 重置上次采样基线，避免首帧瞬时速率爆表
        state.last_in_bytes.store(0, std::sync::atomic::Ordering::Relaxed);
        state.last_out_bytes.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    // 1. 解析 frpc.toml 路径...
    let cfg_path = frpc_config_path(&app)?;
    // ...（原逻辑保持不变）
```

**Step 2：** 在 `start_frpc` 末尾（poll_conn_state spawn 之后）追加 traffic 采样任务 spawn：

```rust
    // 5. 启动连接状态轮询任务（已有）
    let app_for_poll = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::frpc_state::poll_conn_state(app_for_poll).await;
    });

    // 6. 启动流量采样任务：1s 间隔，poll_gen 守护退出
    let app_for_traffic = app.clone();
    let start_gen = state.poll_gen.load(std::sync::atomic::Ordering::Acquire);
    tauri::async_runtime::spawn(async move {
        crate::proxy_relay::poll_traffic(app_for_traffic, start_gen).await;
    });

    Ok(())
```

**Step 3：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：通过。

---

### Task 9：修改 `process.rs::stop_frpc` 关闭 relay

**Files:**
- Modify: `src-tauri/src/process.rs:167-184`（stop_frpc）

**Step 1：** 在 `stop_frpc` 的 `child.kill()` 之后插入 relay 关闭：

```rust
#[tauri::command]
pub fn stop_frpc(app: AppHandle, state: State<'_, FrpcState>) -> Result<(), String> {
    let child = {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(child) = child {
        child
            .kill()
            .map_err(|e| format!("停止核心引擎失败：{e}"))?;
        reset_to_stopped(state.inner());
        emit_log(&app, "system", "已停止核心引擎".into());
        emit_status(&app, state.inner());
    }

    // 异步清空 relay：drop Arc<RelayEntry> 触发 listener 关闭
    let relay_state = state.inner();
    let app_for_relay = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app_for_relay; // 预留
        let mut guard = relay_state.relay.lock().await;
        if let Some(r) = guard.take() {
            crate::proxy_relay::stop_relay(&r).await;
        }
    });

    Ok(())
}
```

**Step 2：** 同步处理 `Terminated` 事件分支（process.rs IO task 内）：在 `reset_to_stopped` 之后同样 spawn 一个清理任务。

定位 `CommandEvent::Terminated` 分支（约 process.rs:119-149），在 `emit_status(&app_for_thread, state.inner());` 之后插入：

```rust
                    // 清理 relay：与 stop_frpc 对称
                    let relay_state = state.inner();
                    tauri::async_runtime::spawn(async move {
                        let mut guard = relay_state.relay.lock().await;
                        if let Some(r) = guard.take() {
                            crate::proxy_relay::stop_relay(&r).await;
                        }
                    });
```

> **注：** 这里复用了上面已经锁定的 `state` 句柄；实际代码需要 clone AppHandle 与 state 引用。具体实现按上下文调整。

**Step 3：** 验证编译：

```bash
cd src-tauri && cargo check
```

期望：通过。

---

### Task 10：前端依赖与类型

**Files:**
- Modify: `package.json`
- Modify: `src/types.ts`

**Step 1：** 安装依赖：

```bash
pnpm add chart.js vue-chartjs
```

期望：`package.json` 的 `dependencies` 出现 `chart.js` 与 `vue-chartjs`。

**Step 2：** 在 `src/types.ts` 末尾追加：

```ts
/**
 * 后端 `frpc://traffic` 事件载荷。
 *
 * 字节方向（用户视角）：
 * - `total_in_bytes` / `in_rate`：用户服务 → frpc（download，下行）
 * - `total_out_bytes` / `out_rate`：frpc → 用户服务（upload，上行）
 * - `connections`：当前 frpc↔frps work connection 数（中转段连接数等价）
 *
 * 仅在 frpc 非停止状态下每秒广播一次。
 */
export interface TrafficPayload {
  total_in_bytes: number;
  total_out_bytes: number;
  in_rate: number;
  out_rate: number;
  connections: number;
}
```

**Step 3：** 验证 typecheck：

```bash
pnpm vue-tsc --noEmit
```

期望：通过。

---

### Task 11：新增 `composables/useTraffic.ts`

**Files:**
- Create: `src/composables/useTraffic.ts`

**Step 1：** 写入文件：

```ts
import { ref } from "vue";

import type { TrafficPayload } from "../types";

/** 滚动窗口长度（秒）；与后端 1s 采样频率对齐 */
const WINDOW_SIZE = 60;

/** 后端 payload 增量值；in_rate/out_rate 已是瞬时值，无需差分 */
export interface TrafficSnapshot {
  timestamp: number;
  in_rate: number;
  out_rate: number;
  connections: number;
}

/** 累计值 */
export const totalInBytes = ref(0);
export const totalOutBytes = ref(0);
/** 滚动窗口数据 */
export const trafficHistory = ref<TrafficSnapshot[]>([]);
/** 最新一次 payload，供组件单值展示 */
export const latestTraffic = ref<TrafficSnapshot>({
  timestamp: 0,
  in_rate: 0,
  out_rate: 0,
  connections: 0,
});

let initialized = false;

/**
 * 处理一次后端 payload：更新累计值与滚动窗口。
 *
 * 由 `useAppEvents` 在 traffic 事件触发时调用；本函数不负责 listen 注册。
 */
export function handleTrafficPayload(p: TrafficPayload) {
  totalInBytes.value = p.total_in_bytes;
  totalOutBytes.value = p.total_out_bytes;

  const now = Date.now();
  const snapshot: TrafficSnapshot = {
    timestamp: now,
    in_rate: p.in_rate,
    out_rate: p.out_rate,
    connections: p.connections,
  };
  latestTraffic.value = snapshot;

  const next = trafficHistory.value.concat(snapshot);
  if (next.length > WINDOW_SIZE) {
    next.splice(0, next.length - WINDOW_SIZE);
  }
  trafficHistory.value = next;
}

/** frpc 停止时调用：清零累计与窗口（图表重置） */
export function resetTraffic() {
  totalInBytes.value = 0;
  totalOutBytes.value = 0;
  trafficHistory.value = [];
  latestTraffic.value = { timestamp: 0, in_rate: 0, out_rate: 0, connections: 0 };
}

/** 标记已初始化（预留：未来可能在多窗口场景判别是否需要重置） */
export function markTrafficInitialized() {
  initialized = true;
}
export function isTrafficInitialized() {
  return initialized;
}

/** 把字节数格式化为人类可读速率字符串，如 "12.3 KB/s" */
export function formatRate(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${bytesPerSec} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  if (bytesPerSec < 1024 * 1024 * 1024)
    return `${(bytesPerSec / 1024 / 1024).toFixed(2)} MB/s`;
  return `${(bytesPerSec / 1024 / 1024 / 1024).toFixed(2)} GB/s`;
}

/** 把累计字节数格式化为人类可读容量字符串，如 "1.23 GB" */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
```

**Step 2：** 验证 typecheck：

```bash
pnpm vue-tsc --noEmit
```

期望：通过。

---

### Task 12：在 `useAppEvents.ts` 注册 traffic 监听

**Files:**
- Modify: `src/composables/useAppEvents.ts`

**Step 1：** 在 import 区追加：

```ts
import type { FrpcStatus, LogEntry, TrafficPayload } from "../types";
import {
  handleTrafficPayload,
  resetTraffic,
} from "./useTraffic";
```

**Step 2：** 在 `useAppEvents` 函数体顶部声明 unlisten 句柄：

```ts
let unlistenTraffic: UnlistenFn | null = null;
```

**Step 3：** 在 `onMounted` 内 `unlistenUpdate` 注册之后插入 traffic 监听 + 状态联动：

```ts
    // 监听流量更新
    unlistenTraffic = await listen<TrafficPayload>(
      "frpc://traffic",
      (event) => {
        handleTrafficPayload(event.payload);
      }
    );
```

**Step 4：** 修改 `unlistenStatus` 回调，在状态转 `stopped` 时重置图表数据：

```ts
    unlistenStatus = await listen<{ status: string; error: string | null }>(
      "frpc://status",
      (event) => {
        frpcStatus.value = event.payload.status as FrpcStatus;
        frpcError.value = event.payload.error ?? null;
        if (frpcStatus.value === "stopped") {
          resetTraffic();
        }
      }
    );
```

**Step 5：** 在 `onUnmounted` 中补 `unlistenTraffic?.()`：

```ts
  onUnmounted(() => {
    unlistenLog?.();
    unlistenStatus?.();
    unlistenUpdate?.();
    unlistenTraffic?.();
    unlistenClose?.();
  });
```

**Step 6：** 验证 typecheck：

```bash
pnpm vue-tsc --noEmit
```

期望：通过。

---

### Task 13：新增独立组件 `components/home/TrafficChart.vue`

**Files:**
- Create: `src/components/home/TrafficChart.vue`

**Step 1：** 写入文件：

```vue
<script setup lang="ts">
import { computed } from "vue";
import { Line } from "vue-chartjs";
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
        label: "下行",
        data: inData,
        borderColor: "hsl(217 91% 60%)",
        backgroundColor: "hsla(217, 91%, 60%, 0.12)",
        borderWidth: 1.5,
        tension: 0.35,
        fill: true,
        pointRadius: 0,
      },
      {
        label: "上行",
        data: outData,
        borderColor: "hsl(142 71% 45%)",
        backgroundColor: "hsla(142, 71%, 45%, 0.12)",
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
const upText = computed(() => formatRate(latestTraffic.value.out_rate));
const downText = computed(() => formatRate(latestTraffic.value.in_rate));
const totalText = computed(
  () => `${formatBytes(totalInBytes.value)} / ${formatBytes(totalOutBytes.value)}`
);
</script>

<template>
  <div class="traffic-card">
    <div class="metrics">
      <div class="metric">
        <div class="metric-label">连接</div>
        <div class="metric-value">{{ connText }}</div>
      </div>
      <div class="metric">
        <div class="metric-label">上行</div>
        <div class="metric-value up">{{ upText }}</div>
      </div>
      <div class="metric">
        <div class="metric-label">下行</div>
        <div class="metric-value down">{{ downText }}</div>
      </div>
      <div class="metric metric-total">
        <div class="metric-label">累计 ↓ / ↑</div>
        <div class="metric-value small">{{ totalText }}</div>
      </div>
    </div>
    <div class="chart-wrap">
      <Line :data="chartData" :options="chartOptions" />
    </div>
  </div>
</template>

<style scoped>
.traffic-card {
  display: flex;
  flex-direction: row;
  align-items: stretch;
  gap: 10px;
  padding: 8px 10px;
  height: 76px; /* ≈ 2× 标题栏 38px */
  background-color: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
  user-select: none;
}

.metrics {
  display: grid;
  grid-template-columns: auto auto;
  grid-template-rows: 1fr 1fr;
  gap: 2px 12px;
  align-content: center;
  flex-shrink: 0;
}

.metric {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  line-height: 1.2;
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
  min-width: 0;
  position: relative;
}
</style>
```

> **设计说明：**
> - 卡片高度 76px = 2 × TitleBar 38px。
> - 左侧 metrics 列用 2×2 grid 排「连接 / 上行」+「累计 / 下行」四块。
> - 右侧 chart-wrap 占据剩余空间，Chart.js responsive 自动适配。
> - 配色用 `--success`（上行 绿）与蓝色（下行）。

**Step 2：** 验证 typecheck：

```bash
pnpm vue-tsc --noEmit
```

期望：通过。

---

### Task 14：在 `HomeView.vue` 顶部插入 TrafficChart

**Files:**
- Modify: `src/views/HomeView.vue`

**Step 1：** 在 script setup 顶部 import：

```ts
import TrafficChart from "../components/home/TrafficChart.vue";
```

**Step 2：** 在 template 的 `home-body` 第一个子元素位置插入（在 CircleButton 之上）：

```vue
    <div class="home-body">
      <TrafficChart />
      <CircleButton :disabled="!isConfigured()" @click="onToggle" />
      <GuideCard v-if="!isConfigured()" @services="emit('services')" />
      <ProxyList
        :proxies="config.proxies"
        :server-addr="config.server_addr"
      />
      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>
```

**Step 3：** 验证 typecheck：

```bash
pnpm vue-tsc --noEmit
```

期望：通过。

---

### Task 15：i18n key（可选）

**Files:**
- Modify: `src/locales/zh-CN.ts` 与 `src/locales/en.ts`

> 主公拍板未要求文案；当前组件用纯数字+图例（"上行"/"下行"/"连接"/"累计"）。若 i18n 文件已按规范用 key 而非裸字符串，需补 key；否则裸字符串暂时合规（首版不强制）。

**Step 1：** grep 检查既有 locale 文件结构：

```bash
grep -n "上行\|下行\|连接\|累计" src/locales/
```

若有 key 化惯例，按 §12 i18n 去重规则补 `traffic_upload` / `traffic_download` / `traffic_connections` / `traffic_total` 等公共 key。

**Step 2：** 否则跳过本任务。

---

### Task 16：手动验证清单（dev 联调）

**Step 1：** 启动 dev：

```bash
pnpm tauri dev
```

**Step 2：** 验证以下场景：

1. **应用首次启动（未配置）**：主页顶部出现 TrafficChart 卡片，所有数字为 0，曲线为空。
2. **配置 TCP 代理并启动**：
   - 主页大圆按钮变 `connecting` → `connected`
   - 卡片左侧"连接"数从 0 升至 ≥1（外部访问公网端口时）
   - 起一个外部访问（如 `curl http://frps:remote_port`），观察"上行/下行"速率变化，曲线开始上升
   - 累计字节随时间累加
3. **停止 frpc**：卡片重置为 0，曲线清空。
4. **再次启动**：曲线重新开始累计，无残留。
5. **HTTP/HTTPS 代理**：同 TCP 行为（同样被中转）。
6. **UDP 代理**：可正常穿透（不被中转），但卡片连接/速率不含 UDP（首版预期）。

**Step 3：** 验证 `cargo check` 与 `pnpm vue-tsc --noEmit` 均通过、无新 warning。

**Step 4：** 验证 `pnpm tauri build` 在当前平台（macOS）打包通过。

---

## 风险与回滚

| 风险 | 应对 |
|---|---|
| tokio 特性扩展后编译时间变长 | 接受；本地 dev 不显著 |
| `copy_bidirectional_counted` 在大流量下吞吐下降 | 8 KiB 缓冲；后续可换 `bytes::BytesMut` 优化 |
| Chart.js 包体让前端 bundle 增大 ~60 KB | 主公已确认接受 |
| 用户机器上 127.0.0.1 防火墙拦截中转端口 | 极少见；中转只本机回环，不暴露外网 |
| relay 与 frpc 端口冲突 | 动态端口（OS 保证）零冲突 |

回滚路径：

1. 把 HomeView 的 `<TrafficChart />` 注释即可禁用前端展示
2. 在 `start_frpc` 把 `start_relay` 调用注释即可禁用中转（流量图表将恒为 0）
3. 完全回滚：`git revert` 单个 commit（如果分阶段提交了的话）

---

## 文档同步清单

实现完成后需同步以下文档（不在本计划内执行；提示主公）：

- [ ] `src-tauri/CLAUDE.md` §3 命令清单不变（无新命令）；§5 新增 §5.6「流量中转层」小节
- [ ] `src-tauri/CLAUDE.md` §4 事件协议表新增 `frpc://traffic` 行
- [ ] `src/CLAUDE.md` §3.1 类型新增 `TrafficPayload`；§4 事件协议表新增 `frpc://traffic` 行；§2 目录结构补 `proxy_relay.rs` / `useTraffic.ts` / `TrafficChart.vue`
- [ ] 根目录 `CLAUDE.md` 术语表新增「中转层」「流量图表」等术语

---

## 执行选项

请主公选择执行方式：

**1. 子代理驱动（当前 session）** — 末将逐任务派出子代理执行，任务间代码审查；快速迭代

**2. 平行 session（独立 session）** — 主公开新 session，用 `superpowers:executing-plans` 批量执行带 checkpoint

末将默认建议 **方式 1**（子代理驱动），便于主公实时介入与方向调整。
