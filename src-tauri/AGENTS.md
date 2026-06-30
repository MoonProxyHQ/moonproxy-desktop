# 后端开发指南（MoonProxy）

> 本文件面向在该目录（`src-tauri/`）下做后端 / 命令 / 进程管理开发的工程师。
> 涉及前后端协议的部分给出对应前端调用点的引用，便于一眼对齐。

## 1. 技术栈

- **框架**：Tauri v2（`tauri = "2"` + `wry` + `tray-icon` + `image-png/ico` + `devtools`）
- **异步**：`tauri::async_runtime`（背后为 tokio）
- **HTTP**：`reqwest 0.12`（`rustls-tls`，禁用 `default-features` 避免 OpenSSL）
- **序列化**：`serde` / `serde_json`；TOML 配置用 `format!` 手写拼接，**不引入 toml crate**
- **压缩 / 哈希**：`flate2`（gzip）+ `tar`（解 tar）+ `sha2` + `hex` + `semver`
- **进程扫描**：`sysinfo 0.32`（仅用于启动前清理孤儿 frpc，详见 §5.2 第 3 步）
- **时间**：`chrono 0.4`（仅启用 `clock` feature；只用来取本地时间 / 周几，供 `scheduler.rs` 使用）
- **插件**：`tauri-plugin-shell` / `-dialog` / `-fs` / `-single-instance` / `-updater` / `-process` / `-autostart`（开机启动）/ `-store`（KV 偏好存储）

## 2. 目录结构与模块职责

```
src-tauri/
├── Cargo.toml                # crate = moonproxy-desktop；lib + bin（cdylib/staticlib/rlib）
├── build.rs                  # tauri_build::build() 入口
├── tauri.conf.json           # 主配置：窗口 400×740、无装饰、sidecar 声明、updater 端点
├── tauri.macos.conf.json     # macOS 平台覆盖：开系统交通灯 + hiddenTitle + Overlay
├── binaries/                 # sidecar 二进制，按目标三元组命名（不入库，开发者本地放置）
│   ├── frpc-aarch64-apple-darwin
│   └── frpc-x86_64-pc-windows-msvc.exe
├── assets/                   # 编译期嵌入的运行时资源（include_bytes!），目前含托盘图标
│   ├── tray_macos_44.png
│   └── tray_windows_64.png
├── capabilities/
│   └── default.json          # 权限：core:window 最小化/隐藏/关闭/拖动 + shell 侧车 + fs 文本 + updater/process/autostart/store
├── src/
│   ├── main.rs               # Windows release 抑制额外控制台（windows_subsystem）
│   ├── lib.rs                # Tauri Builder 配置 + setup hook + invoke_handler + ExitRequested 兜底（业务全部抽到子模块）
│   ├── types.rs              # 共享类型：StartArgs / ProxyConfig
│   ├── config.rs             # frpc.toml 生成（build_toml / escape_toml）+ 配置持久化命令（save/load_config）
│   ├── context_menu.rs       # 输入框原生右键编辑菜单（show_edit_menu：剪切/复制/粘贴/全选）
│   ├── process.rs            # frpc 子进程生命周期（start/stop_frpc / frpc_running/status / get_logs / reap_orphan_frpc）
│   ├── frpc_state.rs         # 连接状态机：FrpcConnState / FrpcState / poll_conn_state + emit_log
│   ├── proxy_health.rs       # 代理本地端口连通性探测（probe_proxy / check_proxies_health）
│   ├── frpc_update.rs        # frpc 自更新：版本、GitHub 查询、下载、原子替换
│   ├── prefs.rs              # 应用偏好：auto_launch / silent_start / auto_connect / schedule（store 持久化、autostart 写入、setup 静默 + 自连判定、schedule 校验）
│   ├── scheduler.rs          # 按星期定时启停 frpc：spawn_scheduler（分钟对齐 tick）+ maybe_catch_up_start（启动补跑）
│   └── tray.rs               # 系统托盘：init_tray（菜单 + 左键唤起窗口）
└── .gitignore                # /target/ 与 /gen/schemas
```

## 3. 命令清单（`invoke_handler`）

| 命令                          | 入参                              | 出参                              | 来源              |
| ----------------------------- | --------------------------------- | --------------------------------- | ----------------- |
| `start_frpc`                  | `StartArgs`                       | `Result<(), String>`              | `process.rs`      |
| `stop_frpc`                   | —                                 | `Result<(), String>`              | `process.rs`      |
| `frpc_running`                | —                                 | `Result<bool, String>`            | `process.rs`      |
| `frpc_status`                 | —                                 | `Result<StatusPayload, String>`   | `process.rs`      |
| `get_logs`                    | —                                 | `Result<Vec<LogEntry>, String>`   | `process.rs`      |
| `save_config`                 | `StartArgs`                       | `Result<(), String>`              | `config.rs`       |
| `load_config`                 | —                                 | `Result<Option<StartArgs>, String>` | `config.rs`     |
| `check_proxies_health`        | `proxies: Vec<ProxyConfig>`       | `Result<Vec<ProxyHealth>, String>` | `proxy_health.rs`|
| `get_frpc_version`            | —                                 | `Result<String, String>`          | `frpc_update.rs`  |
| `check_frpc_update`           | —                                 | `Result<Option<UpdateInfo>, String>` | `frpc_update.rs` |
| `download_frpc_update`        | `version: String`                 | `Result<(), String>`              | `frpc_update.rs`  |
| `apply_pending_frpc_update`   | —                                 | `Result<Option<String>, String>`  | `frpc_update.rs`  |
| `get_prefs`                   | —                                 | `Result<Prefs, String>`           | `prefs.rs`        |
| `save_prefs`                  | `prefs: Prefs`                    | `Result<(), String>`              | `prefs.rs`        |
| `set_auto_launch`             | `enabled: bool`                   | `Result<bool, String>`            | `prefs.rs`        |
| `get_auto_launch`             | —                                 | `Result<bool, String>`            | `prefs.rs`        |
| `show_edit_menu`              | `window: Window`（Tauri 注入）    | `Result<(), String>`              | `context_menu.rs` |

> **前端类型映射注脚**：`load_config` 后端返回 `Result<Option<StartArgs>, String>`，
> 前端 `src/commands/config.ts::loadConfig` 映射为 `Promise<boolean>`——`Some`
> → `true`（已写回响应式 `config`），`None` 或解析失败 → `false`（视同首次启动）。
> `commands/*.ts` 中其它命令统一返回 `Promise<string | null>`（错误消息）。

### 3.1 `StartArgs` / `ProxyConfig`（`types.rs`）

- `StartArgs` 字段：`custom_name`（自定义服务商语义）+ `server_addr` / `server_port` / `token` / `user` / `proxies`
- 字段命名严格 snake_case
- `custom_name` / `token` / `user` 为 `Option<String>`：前端在
  `toArgs()` 中空字符串 → `null`；后端据此决定是否写入 `auth.token` / `user` 字段
- `ProxyConfig` 按 frp 官方各类型 schema 拆分为 `#[serde(tag = "type")]` 的内部标签 enum：
  - `tcp` / `udp` variant：`name` / `local_ip` / `local_port` / `remote_port`
    （frp 接受 `remotePort`，**不接受** `customDomains`）
  - `http` / `https` variant：`name` / `local_ip` / `local_port` / `custom_domains: Vec<String>`
    （frp 接受 `customDomains`，**不接受** `remotePort`——否则报 `unknown field "remotePort"`）
  - 序列化形态：`{ "type": "tcp", "name": "...", ... }`，与前端 `ProxyConfig`
    discriminated union 一一对应；**非法字段在编译期就不可能出现在错的 variant 上**，
    避免 `build_toml` / `probe_proxy` / URL 生成路径再按 `proxy_type` 字符串运行期分叉

### 3.2 `Prefs`（`prefs.rs`）

```rust
struct Prefs {
    auto_launch: bool,   // 开机启动（OS 实际状态）
    silent_start: bool,  // 静默启动：开机自启时隐藏到托盘
    auto_connect: bool,  // 开机自动连接：仅 OS 自启时（--auto-launched）自动拉起 frpc
    schedule: Schedule,  // 定时连接（按星期多选 + 起止时间）
    language: String,    // 界面语言 code（"zh-CN" / "en"）
}

struct Schedule {
    enabled: bool,
    weekdays: [bool; 7],   // 下标 0=周一 … 6=周日；与 chrono::Weekday::num_days_from_monday 一一对应
    start_time: String,    // "HH:MM" 24 小时制
    stop_time: String,     // "HH:MM" 24 小时制
}
```

- **存储**：`tauri-plugin-store`，文件名 `prefs.json`，键 `auto_launch` / `silent_start` / `auto_connect` / `schedule` / `language`（旧 prefs.json 缺新字段时反序列化为 default，向后兼容）
- **与 frpc 配置区分**：`Prefs` 走 `prefs.json`，`StartArgs`（服务商 / Token / 代理规则）走另一个 store 文件 `config.store.json`（键 `start_args`）；二者均由 `tauri-plugin-store` 托管，路径与序列化由插件统一管理
- **`save_prefs`** 入口处调 `validate_schedule`，启用态下检查「至少一天 + 时间合法 + 起早于止（暂不支持跨夜）」；失败返 `Err`，前端映射为 Toast
- **`set_auto_launch`**：先调用 `tauri-plugin-autostart` 的 `enable()/disable()` 写 OS 启动项，再用 `is_enabled()` 反查实际状态回填 `Prefs.auto_launch`（避免插件在某些平台 silently 失败导致状态不一致）
- **autostart 启动参数**：`init` 时传入 `Some(vec!["--auto-launched"])`，OS 启动应用时会带上该 flag，**用户主动双击启动则不带**——这是静默启动 + 开机自动连接判定的共同依据
- **`maybe_auto_connect`**（setup 调用）：检测到 `--auto-launched` 且 `Prefs.auto_connect=true` 时，spawn 异步任务执行 `fire_auto_connect`——先 `child.is_some()` 预检查幂等跳过（避免与 `scheduler::maybe_catch_up_start` 竞争时被 `start_frpc` 内部互斥返 `"已在运行"` 而产生误导性错误日志），再加载 `StartArgs` 并 `start_frpc`；未配置服务端时 `emit_log` 提示跳过
- **`Prefs` 不再 `Copy`**：`Schedule` 含 `String`；调用方用引用或 `.clone()`

## 4. 事件协议（`app.emit`）

| 事件名                       | 载荷结构                                  | 触发点                                                                |
| ---------------------------- | ----------------------------------------- | --------------------------------------------------------------------- |
| `frpc://log`                 | `{ line, stream: 'stdout'\|'stderr'\|'system' }` | `emit_log()` 在生成配置、启动、Stdout/Stderr/Terminated/Error 时广播；同时写入 `FrpcState.logs` 环形缓冲（500 条）供 `get_logs` 拉取 |
| `frpc://status`              | `{ status: 'stopped'\|'connecting'\|'connected'\|'error', error: string\|null }` | `start_frpc` 设 `connecting`；`poll_conn_state` 推 `connected` / `error`；`stop_frpc` 与 `Terminated` 设 `stopped` |
| `frpc://update-downloaded`   | `{ version: string }`                     | `download_frpc_update` 成功原子写入后                                 |

> 协议命名采用 `frpc://` URI 风格，与 Tauri 自带事件命名空间区分。修改载荷时
> 必须同步 `src/composables/useAppEvents.ts` 中的 `listen<T>` 注册点与
> `src/components/settings/LogsTab.vue` 的 `listen<T>` 历史拉取点。

## 5. 进程管理（`process.rs` + `frpc_state.rs`）

> 子进程生命周期（启动 / 停止 / 状态 / 日志 / 孤儿清理）在 `process.rs`；
> 连接状态机与日志环形缓冲在 `frpc_state.rs`；`lib.rs::run()` 仅做
> `FrpcState` 注入 + `ExitRequested` 兜底 kill。

### 5.1 状态

```rust
struct FrpcState {
    child: Mutex<Option<CommandChild>>,
    conn: Mutex<FrpcConnState>,        // 详见 §5.5
    error_msg: Mutex<Option<String>>,
    started_at: Mutex<Option<Instant>>,
    poll_gen: AtomicU64,               // 每次 start_frpc 自增；旧 polling 据此退出
    logs: Mutex<VecDeque<LogEntry>>,   // 最近 500 条 frpc 日志，独立日志窗口打开时一次性拉取
}
```

通过 `.manage(FrpcState::default())` 注入运行时。`State<'_, FrpcState>`
在命令签名中访问。

> `logs` 缓冲与 `frpc://log` 事件**同源同写**：`emit_log` 先 `push_log`
> 入 `VecDeque` 并 trim 到 500 条（`LOG_BUFFER_LIMIT`），再 `app.emit`
> 广播。独立日志窗口（label=`logs`）通过 `get_logs` 命令拉历史，
> 再 `listen("frpc://log")` 实时追加；主窗仍按原协议 listen 用于顶部错误条
> 等场景。

### 5.2 启动序列

`start_frpc` 是 `async fn`，第 2 / 3 步的阻塞工作（fs 写盘 + sysinfo 全表扫描 +
命中后 sleep）通过 `tauri::async_runtime::spawn_blocking` 丢到 blocking 线程池
**并行**执行，不在 tokio worker 上直接跑——避免阻塞 Tauri 主线程与 UI 响应。

1. **互斥检查 + sidecar 拉起 + 写入状态**（**合并到同一 `state.child`
   锁临界区**）：`lock()` → 检查 `child.is_some()`（是则 `Err("核心引擎已在运行中")`）
   → `app.shell().sidecar("frpc").args(["-c", path]).env("FRPC_LOG_LEVEL", "warn")`
   → `*guard = Some(child)` → drop guard。spawn sidecar 是同步 syscall（fork + exec），
   毫秒级，在锁内执行可接受。原子化消除两个并发 `start_frpc` 各自 spawn frpc
   进程导致端口冲突 / state 引用错乱 / 孤儿进程的 TOCTOU 竞争。
2. **生成配置**（spawn_blocking）：`app_config_dir()/frpc.toml`，内容为手写
   TOML（含 `webServer.*`、`transport.*`、可选 `auth.token` / `user`、`[[proxies]]`）；
   代理类型必须是 `SUPPORTED_PROXY_TYPES`（tcp/udp/http/https）之一，否则拒绝
3. **清理孤儿 frpc**（spawn_blocking，与第 2 步并行）：`reap_orphan_frpc`
   用 `sysinfo` 扫描所有进程，匹配「映像名为 `frpc` 或 `frpc.exe`（Windows
   大小写不敏感）**且** `cmd()` 参数中存在等于本次 `cfg_path` 字符串的项」
   的进程，命中则 SIGKILL。匹配规则保证只杀本应用派生的 frpc（用同一
   `frpc.toml` 路径），不会误伤用户机器上其他用途的 frpc。命中数 > 0 时
   sleep `REAP_SETTLE_MS = 200`ms 等 OS 回收端口。
   - **为什么需要**：`tauri dev` 改 Rust 代码触发 cargo 重编译时旧主二进制
     被 SIGKILL → `ExitRequested` 钩子来不及跑 → frpc sidecar 被 reparent 到
     init/launchd 持续占着 `127.0.0.1:7400` → 下次启动 bind 必失败
4. **状态广播**：`mark_connecting` bump generation 并设 `started_at`；
   `emit_status` 广播 `connecting` + `emit_log(system, "核心引擎已启动")`
5. **异步消费**：`tauri::async_runtime::spawn` 收 `CommandEvent`：
   - `Stdout/Stderr` → 按行 `emit_log(stream, line)`
   - `Terminated` → 清空 `child`、`reset_to_stopped`、`emit_status` 广播 `stopped`、`break`
   - `Error` → 记 `stderr` 错误
6. **状态轮询**：再 spawn 一个 `frpc_state::poll_conn_state` 任务（3s 间隔探测
   frpc webServer `/api/status`，详见 §5.5）

#### 5.2.1 setup hook 状态共享

`maybe_silent_start` / `maybe_auto_connect` / `scheduler::maybe_catch_up_start`
三个 `maybe_*` 都「同步判定 + 异步 spawn」，且都消费 `Prefs` 与
`--auto-launched` 启动参数。历史上每个 `maybe_*` 各自 `prefs::load` /
`args().any`，会重复读 3 次 store + 扫 2 次 argv。当前约定由 setup hook
顶层算一次后传入，避免重复开销。

**约定**：

- `maybe_*` 入口函数**接收** `&Prefs` 与 `auto_launched: bool` 形参；由 setup
  hook 顶层 `let prefs = prefs::load(app); let auto_launched = std::env::args().any(...)`
  算一次后传入。**不要**在 `maybe_*` 内部重新 `load` / 重新扫 argv
- `scheduler::spawn_scheduler` 的**分钟 tick** 仍每次重读 `prefs`——这是
  刻意的热加载语义，与 setup 共享的「首跑 prefs」**不**同源，不要为
  一致性而把 tick 改为传入 `Prefs`
- 新增第 4 个 `maybe_*` 入口（如快捷键触发）时，沿用同一约定

### 5.3 停止

`stop_frpc` 直接 `child.kill()`，**不等 Terminated**——但 `Terminated` 事件
仍会异步到达并清空 `child`（双保险）。`child` 为 `None` 时幂等返回 `Ok(())`，
既兼容前端"未运行也点停止"的路径，也让退出钩子可以无条件调用而不污染日志。
错误统一 `format!("停止核心引擎失败：{e}")`。

> **退出兜底**：`run()` 的 `.run(...)` 回调监听 `RunEvent::ExitRequested`——无论
> 哪条退出路径（托盘"退出" / Cmd+Q / 关闭确认"退出" / 最后窗口关闭）都先
> 同步 `child.kill()` frpc 子进程，避免主进程退出后留下孤儿 frpc。
>
> **SIGKILL 失效场景**：当主进程被强制杀死（`tauri dev` 热重载触发 cargo
> SIGKILL 旧二进制 / `kill -9` / 崩溃）时，`ExitRequested` 钩子**来不及跑**，
> frpc sidecar 被 OS reparent 到 init/launchd 持续占用 7400 端口。该场景靠
> §5.2 第 3 步的 `reap_orphan_frpc` 在**下次启动时**兜底清理，正常退出路径
> 仍依赖本钩子。

### 5.4 代理健康检测（`check_proxies_health`）

> **目的**：在主页代理行前置一个健康状态点，提前告诉用户"本地端口是否能
> 联通"，避免启动 frpc 后才发现穿透失败。

#### 数据结构

```rust
#[derive(Serialize, Clone)]
struct ProxyHealth {
    index: usize,   // 与传入代理列表下标一一对应
    ok: bool,       // 本地端口是否可达
    message: String,// 给用户看的状态文案（直接显示在状态点 title）
}
```

#### 检测策略（`probe_proxy`）

| 代理类型                  | 探测方式                                              | 超时    |
| ------------------------- | ----------------------------------------------------- | ------- |
| `tcp` / `http` / `https`  | `TcpStream::connect_timeout` 解析到的所有 IP 依次尝试 | 1.5 s   |
| `udp`                     | `UdpSocket::bind("0.0.0.0:0")` + `send_to(&[])` 探测  | 1.5 s   |
| 其他（理论上 UI 已限制）  | 不探测，直接返回 `ok=true` 标记为"未检测"             | —       |

- 仅在多 IP 主机上，`ToSocketAddrs` 可能返回多个地址；**任意一个 TCP 连接成功
  即视为可达**——这与 frp 客户端自身解析顺序的语义对齐
- UDP 用空 payload 探测；能 `send_to` 成功只代表"链路可写"，**不代表对方有
  应用在监听**。在 `message` 中明确文案"UDP 探测已发送"以避免误读

#### 并发模型

`check_proxies_health` 是 `async` 命令，但 `probe_proxy` 内部是阻塞 IO
（`connect_timeout` / `UdpSocket`）。**所以必须用 `tauri::async_runtime::spawn_blocking`**
把每条代理探测丢到 blocking 线程池，再 `await` 收结果——

```rust
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
```

- 不要换成 `std::thread::spawn`——会绕过 tokio 线程池，阻塞 IO 跑满系统线程
- 也不要用 `tauri::async_runtime::spawn`——后者假设任务是非阻塞的
- 任务 panic 时 `JoinHandle` 自身不会 panic；`h.await` 返回 `Err`，统一
  透传为 `"检测任务异常退出：{e}"`

#### 协议约定

- 字段命名严格 snake_case / 标量 `bool` / `usize` / `String` 直出，前端按
  `ProxyHealth` 接口接收
- 返回顺序按 `index` 升序排好（即使将来并发策略改变），前端用 `index` 落位
  不依赖顺序

#### capabilities

`check_proxies_health` 是纯计算命令，**不需要** `capabilities/default.json`
新增权限。命令直接定义在 `proxy_health.rs` 内（`#[tauri::command]` 属性），
`lib.rs` 通过 `use proxy_health::check_proxies_health` 引入并注册到
`invoke_handler!`。

### 5.5 连接状态机（`FrpcConnState` + `poll_conn_state`）

> **目的**：让"已连接"由 frpc 自身证据支撑，而不是 Tauri 自维护。
> frpc webServer（`webServer.*` 段，固定 `127.0.0.1:7400`，凭据 `admin/admin`）
> 暴露 `/api/status`：frpc 成功 login 到 frps 并注册代理后，对应代理的
> `status` 字段会变成 `"running"`。任何 `running` 即视为 `Connected`。

#### 状态枚举

```rust
enum FrpcConnState { Stopped, Connecting, Connected, Error }
```

#### 转换规则（`poll_conn_state`，3s 间隔）

| 当前态      | 探测结果                       | 新态         |
| ----------- | ------------------------------ | ------------ |
| `Stopped`   | —                              | 不轮询       |
| `Connecting`| 至少一条代理 `status="running"`| `Connected`  |
| `Connecting`| 30s 内始终没有代理 running     | `Error`      |
| `Connecting`| 还在 30s 内                    | `Connecting` |
| `Connected` | 仍有 running                   | `Connected`  |
| `Connected` | running 全消失                 | 回到 `Connecting`（重新计时） |
| `Error`     | 任意                           | 保持 `Error` 直到外部 stop/启动 |

> **退出条件**（任一触发 polling task `break`）：
> - 子进程终止（`child.is_none()`）
> - 新一轮 polling 已接管（`poll_gen` 与起始时记录不一致——`mark_connecting` 每次自增）
>
> 旧 polling task 不会泄漏；前端在 `Connecting` 中点取消时也会触发 stop → reset → 下次循环 break。

#### 兜底说明

- frpc `/api/status` 在「刚启动」「网络错」「token 错」三种场景都返回 `{}`，
  无法精确区分；故 `Connecting` 状态最长维持 30s，30s 内仍 `{}` 即升级为 `Error`
- 30s 是经验值：正常环境下 frpc 登录 + 注册代理通常 ≤3s；30s 给慢网 / DNS 留
  充分余量
- **Connected 掉线**：当已 `Connected` 但某次探测发现 running 全消失（frps 短暂
  抖动 / 网络中断），自动回到 `Connecting` 并刷新 `started_at`，给 frpc 一个
  30s 自愈窗口；超过 30s 仍未恢复则升级为 `Error`。这样避免短暂网络抖动
  立即把用户导向"连接错误"
- 用户从 `Error` 点击重试：前端会先 `stopFrpc` 再 `startFrpc`，
  重新走一次 `Connecting` 流程（不依赖自动恢复）
- **用户在 `Connecting` 中点取消**：前端直接 `stopFrpc`，状态回到 `Stopped`，
  polling task 通过 `poll_gen` 变化或 `child.is_none()` 自动退出，不卡死

#### `webServer` 段

TOML 模板中固定写入：

```
webServer.addr = "127.0.0.1"
webServer.port = 7400
webServer.user = "admin"
webServer.password = "admin"
```

**顺序约束**：`webServer.*` 必须在 `[[proxies]]` 之前（TOML 数组表语法），
否则报 `unknown field "webServer"`。

## 6. TOML 配置生成与转义

不引入 toml crate；`config::build_toml` 用 `format!` 直接拼。**任何写入 `frpc.toml`
的字符串值都必须经过 `escape_toml` 转义**：

```rust
fn escape_toml(s: &str) -> String {
    s.replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
```

转义策略：先把 `\r` / `\n` / `\t` 替换为空格（避免用户在名称字段粘入换行 / Tab
破坏 frpc.toml 解析），再处理 TOML 字符串值必需的 `\` 与 `"`。`transport.*` 等
数值 / 标识字段不需要转义。

## 7. 自更新（`frpc_update.rs`）

> **核心目的**：让用户在不重新下载安装包的情况下升级 frpc 引擎。
> 流程：GitHub → SHA256 校验 → 提取 tar.gz 中的 frpc → 写 pending → 重启生效。

### 7.1 关键常量

- `BUNDLED_FRPC_VERSION = "0.69.1"` —— 与 `binaries/` 中内置二进制保持同步
- `GITHUB_API_LATEST` / `FRP_DOWNLOAD_HOST` —— GitHub Release 接口
- 资产命名遵循 frp 官方：`frp_{ver}_{os}_{arch}.tar.gz`
  - os: `darwin` / `windows` / `linux`
  - arch: `arm64` / `amd64`

### 7.2 平台映射

| 平台                     | asset suffix        | sidecar 目标三元组                |
| ------------------------ | ------------------- | --------------------------------- |
| macOS Apple Silicon      | `darwin_arm64`      | `frpc-aarch64-apple-darwin`       |
| macOS Intel              | `darwin_amd64`      | `frpc-x86_64-apple-darwin`        |
| Windows x64              | `windows_amd64`     | `frpc-x86_64-pc-windows-msvc.exe` |
| Windows ARM64            | `windows_arm64`     | `frpc-aarch64-pc-windows-msvc.exe`|
| Linux x86 / ARM64        | `linux_amd64/arm64` | `frpc-*-unknown-linux-gnu`        |

> 新增平台支持时：① 把 frp 官方 release 二进制重命名为上表名放入
> `binaries/`；② 同步更新 `tauri.conf.json` 的 `externalBin` 与 capabilities。

### 7.3 状态文件

`app_config_dir()/frpc_update.json`：

```json
{
  "current": "0.69.1",
  "pending": { "version": "0.70.0", "path": "..." }
}
```

- `current`：当前生效版本，启动时由 `apply_pending_update` 接管
- `pending`：已下载未应用，重启时由 `apply_pending_update` 接管

### 7.4 流程

1. **检查** `check_frpc_update`：
   - GET `GITHUB_API_LATEST`
   - 用 `semver::Version` 比较 latest vs current；≤ 则返回 `None`
   - 在 `assets` 中按 `asset_name` 查找；返回 `UpdateInfo`
2. **下载** `download_frpc_update`：
   - GET `frp_{v}_checksums.txt` 解析对应 hash
   - GET 资产 → SHA256 校验
   - 用 `flate2` + `tar` 解压，提取名为 `frpc` / `frpc.exe` 的 entry
   - 写 `frpc_pending_{v}[.exe]` 到 app_config_dir
   - `set_executable(0o755)`（Unix）
   - 保存 state、广播 `frpc://update-downloaded`
3. **应用** `apply_pending_update`（启动时由 `lib.rs::run()` 的 `setup` 调用）：
   - 读 pending → `atomic_write` 替换到 `resource_dir/binaries/frpc-*`
   - `set_executable` + macOS 平台 `ad_hoc_codesign -s - -f`（必要：否则会被 Gatekeeper 拦）
   - 跑一次 `frpc -v` 验证可执行
   - 验证通过后删 pending 文件、更新 state.current

### 7.5 原子写

`atomic_write`（`frpc_update.rs`）写 `*.frpc_tmp` 再 `rename`，
**避免半写状态导致下次启动跑坏二进制**。所有对 sidecar / pending 的写都走它。

## 8. 窗口与平台配置

### 8.1 `tauri.conf.json`

- 窗口 `400×740`、不可调整大小、不可最大化
- `decorations: false`（自定义标题栏）+ `theme: "Light"`
- `dragDropEnabled: false`（避免误触）
- `bundle.targets: ["app", "dmg", "nsis"]`（macOS / Windows 平台各自取子集；不上 `.msi`），`externalBin: "binaries/frpc"` 启用 sidecar

### 8.2 `tauri.macos.conf.json`（macOS 平台覆盖）

- `decorations: true` + `titleBarStyle: "Overlay"` + `hiddenTitle: true`
  → 用系统原生交通灯区，但标题文字不显示
- 真正生效的是 `tauri build --target ...` 走 `.macos.conf.json` 覆盖
- 历史上曾因 `tauri.macos.conf.json` 没覆盖窗口尺寸而退回默认 `800×600`；
  **修改 macOS 配置时同步检查尺寸**别被默认覆盖

## 9. 权限

权限按窗口 label 分文件管理：

### 9.1 `capabilities/default.json`（窗口 `main`）

- `core:default` —— Tauri v2 默认窗口 / webview 权限集
- `core:webview:allow-create-webview-window` —— 主窗通过 `WebviewWindow` 派生独立日志窗口（label=`logs`），需要显式授权
- `core:window:allow-minimize / allow-hide / allow-close / allow-start-dragging` —— 与前端 `Cmd+M` 原生最小化、关闭确认弹窗 `hide()` 到托盘、关闭按钮、拖动对应（`core:default` 默认集**不**包含这些，必须显式声明）
- `dialog:default` —— `@tauri-apps/plugin-dialog` 兜底（原生对话框）
- `shell:allow-execute` 显式声明 `binaries/frpc` 为 sidecar（`args: true` 允许传参）
- `shell:allow-kill` —— `stop_frpc` / `ExitRequested` 兜底 kill 子进程用
- `fs:default + allow-read-text-file / allow-write-text-file / allow-exists / allow-mkdir / allow-read-dir`
  —— 配置文件读写
- `updater:default` / `process:default` —— 应用本体自更新 + 进程退出
- `autostart:default` —— 开机启动写入 OS 启动项
- `store:default` —— `tauri-plugin-store` 读写 `config.store.json` / `prefs.json`

### 9.2 `capabilities/logs.json`（窗口 `logs`）

独立日志窗口只需最小权限：`core:default`（含 `core:event:default` 允许 `listen`）
+ `core:window:allow-close` + `core:window:allow-minimize`。日志窗口使用系统
原生装饰（`decorations: true`），无需 `start-dragging` / `hide` 等主窗专用权限。

> 新增 fs / shell 操作时必须先在 capabilities 加权限；Tauri v2 默认拒绝一切未声明的特权。
> 新增独立窗口（`WebviewWindow`）时，**必须**额外加 `capabilities/<label>.json`，
> 否则该窗口连 `listen` / `invoke` 都没有权限——`default.json` 的 `windows: ["main"]`
> 不自动覆盖新 label。

## 10. 启动钩子（`lib.rs` setup）

```rust
.setup(|app| {
    if let Err(e) = frpc_update::apply_pending_update(app.handle()) {
        eprintln!("[frpc-update] 应用待安装更新失败: {e}");
    }
    prefs::maybe_silent_start(app.handle());
    prefs::maybe_auto_connect(app.handle());
    scheduler::maybe_catch_up_start(app.handle());
    scheduler::spawn_scheduler(app.handle().clone());
    init_tray(app.handle())?;
    Ok(())
})
```

- **单实例插件**（`tauri-plugin-single-instance`）：第二次启动时唤起主窗
  并 `show / unminimize / set_focus`
- **自更新接管**：每次启动都跑一次 `apply_pending_update`，确保下载完的
  新版本在重启后被原子替换
- **静默启动**（`prefs::maybe_silent_start`）：检测启动参数含 `--auto-launched`
  且 `prefs.silent_start` 为 true → `window.hide()`。用户主动双击启动不带该
  参数，故永远正常显示
- **开机自动连接**（`prefs::maybe_auto_connect`）：检测启动参数含 `--auto-launched`
  且 `prefs.auto_connect` 为 true → spawn `fire_auto_connect`：先预检查
  `child.is_some()` 幂等跳过，再加载 `StartArgs` 并 `start_frpc`。与 schedule
  正交：本函数仅影响"开机那一瞬间是否拉起 frpc"，之后 frpc 的死活由
  schedule / 用户手动操作接管
- **调度补跑**（`scheduler::maybe_catch_up_start`）：若当前正处在用户配置的
  "应运行"窗口内（星期匹配 + 时间区间）且 frpc 未在跑，spawn 异步任务启动 frpc。
  覆盖应用刚启动 / 系统自启 / 应用崩溃后被重新打开等场景
- **调度循环**（`scheduler::spawn_scheduler`）：派生一个长驻 tokio 任务，睡到
  下一分钟整点 tick；命中 `start_time` 启动、命中 `stop_time` 停止（详见 §10.3）
- **托盘初始化**（`init_tray`）：构建菜单（显示主窗口 / 退出）+ 左键点击唤起。
  仅当静默启动开启后，托盘才是用户唤回隐藏窗口的唯一入口

### 10.1 托盘菜单事件

| 菜单项 ID     | 行为                                   |
| ------------- | -------------------------------------- |
| `tray_show`   | `window.show() + unminimize() + set_focus()` |
| `tray_quit`   | `app.exit(0)`                          |

左键点击托盘图标 = `tray_show` 同行为（`on_tray_icon_event`）。

### 10.2 关闭路径与关闭确认弹窗

> **核心约定**：关闭按钮 / Cmd+W / Alt+F4 **始终**先 `preventDefault()`，
> 由前端按 frpc 当前状态决定真正要做什么。拦截实现位置：前端 `src/App.vue`
> 的 `getCurrentWindow().onCloseRequested` 钩子（单点拦截所有关闭路径）；
> Rust 端**不**写 `on_window_event`。

| 触发 | 行为 |
| ---- | ---- |
| frpc `stopped` 状态下任意关闭路径 | `preventDefault()` + 调用 `@tauri-apps/plugin-process` 的 `exit(0)` 直接退出进程（macOS 托盘应用窗口关闭默认不退出 NSApp，故必须显式 exit）|
| frpc `connecting` / `connected` / `error` 下任意关闭路径 | `preventDefault()` + 显示 `CloseConfirm.vue` |
| 用户点「最小化」 | `getCurrentWindow().hide()`（隐藏到托盘，frpc 后台继续运行）|
| 用户点「退出」 | `@tauri-apps/plugin-process` 的 `exit(0)`（与 `tray_quit` 对齐）|
| 用户按 Esc / 点遮罩 / 点 × | 仅关弹窗，窗口状态不变 |
| 托盘菜单 `tray_quit` | **绕过** `onCloseRequested`（走 `app.exit(0)`），等价于强制退出 |
| 弹窗打开期间再触发关闭路径 | `preventDefault()` 防止窗口关闭，弹窗状态保持 |

**为什么 `hide()` 而不是 `minimize()`**：托盘已是既有的"唤回隐藏窗口"入口
（`tray_show` / 左键点击托盘），`hide()` 让"最小化保持后台运行"语义闭环；
`minimize()` 会停留在任务栏，与"后台运行"心智不符。`Cmd+M` 仍走原生
`minimize()`，与本弹窗的 `hide()` 是两套独立路径，不冲突。

**配套权限**：`capabilities/default.json` 必须显式声明 `core:window:allow-hide`
（`core:default` 默认集**不**包含此权限，参见 §9）。

### 10.3 调度器（`scheduler.rs`）

> **目的**：按用户配置的星期 + 起止时间，自动启停 frpc，覆盖「公司 ERP
> 周一到周五 8:00 启动 / 18:00 断开」一类典型场景。

#### 数据结构

复用 `Prefs::schedule`（`prefs.rs` §3.2），不另开 store 文件：

```rust
struct Schedule {
    enabled: bool,
    weekdays: [bool; 7],   // 下标 0=周一 … 6=周日
    start_time: String,    // "HH:MM" 24 小时制
    stop_time: String,     // "HH:MM" 24 小时制
}
```

`weekdays` 下标与 `chrono::Weekday::num_days_from_monday()` 一一对应，
调度器直接索引，无须手动 switch。

#### 校验

`prefs::validate_schedule` 在 `save_prefs` 入口处检查：
- `enabled=true` 时至少一天 `true`
- `start_time` / `stop_time` 合法（`is_hhmm`）
- `start_time != stop_time` 且 `start_time < stop_time`（**暂不支持跨夜**）
- `start_time == stop_time` / `start_time > stop_time` 返 `Err`，前端映射为 Toast

#### 调度循环（`spawn_scheduler`）

`setup` 阶段派生一个长驻 tokio 任务：

```text
loop:
  sleep(secs_to_next_minute())   // 睡到下一个整分（1..=60s）
  tick():
    prefs = prefs::load(app)      // 每次重读，热加载
    if !prefs.schedule.enabled: return
    if !weekdays[now.weekday_idx()]: return
    cur = format!("{:02}:{:02}", now.hour(), now.minute())
    if cur >= start_time && cur < stop_time: try_fire_start()
    elif cur >= stop_time: try_fire_stop()
```

- **分钟对齐**而非每秒扫描：避免无用 CPU
- **区间判定**而非整分等值：产品意图是「区间内大概触发」，不要求整分精度。
  即便 OS 冻结（macOS App Nap / Windows Modern Standby）或长 sleep 抖动跨过
  整分，只要下一分钟仍在窗口内就会补跑。重复触发由 `try_fire_start` /
  `try_fire_stop` 内部 `child.is_some()` 幂等检查拦截，无需额外的日期 flag
- **热加载**：用户改完配置点保存，下一个分钟就生效，无需重启应用
- **系统时钟跳变**（NTP 倒退 / DST 切换）仍可能导致重复或错过触发，v1 接受此限制

#### 启动补跑（`maybe_catch_up_start`）

`setup` 阶段同步调用：检查 `in_window(&schedule, now)`，是则 spawn 异步
任务执行 `start_frpc`。`in_window` 语义：

```text
in_window = enabled
         && weekdays[weekday_idx(now)] == true
         && start_time <= cur_time < stop_time
```

- 窗口判定用 `>= start` 且 `< stop`（左闭右开），与 `in_window` 函数内
  字符串比较一致；与 `tick` 的区间判定同源
- **不阻塞** `setup` hook：内部走 `tauri::async_runtime::spawn`

#### 幂等性

调度器触发 `try_fire_start` / `try_fire_stop` 时**先检查 frpc 当前状态**，
已在跑时跳过启动、未在跑时跳过停止——避免重复启停产生误导日志 / 二次绑定。

| 调度触发 → 当前 frpc 状态 | 行为 |
| --- | --- |
| 启动触发 → 未运行 | 调用 `start_frpc`（复用现有 §5.2 流程） |
| 启动触发 → 已在跑 | 跳过（幂等） |
| 停止触发 → 在跑 | 调用 `stop_frpc`（复用现有 §5.3 流程） |
| 停止触发 → 未在跑 | 跳过（幂等） |

#### 错误处理

调度触发的启停失败时**不弹错误条**（无人接收）——`emit_log("system", ...)`
写入日志环形缓冲与 `frpc://log` 事件流，用户在「日志」Tab 即可看到。

`start_frpc` 在调度路径下可能因 `StartArgs` 未保存而失败（用户只填了调度
还没填服务商），处理：调用 `config::load_config`，未拿到 `StartArgs` 时
`emit_log` 记录「调度触发启动，但尚未配置服务端，跳过」并返回。

#### 用户手动操作的优先级

调度器**不会**打断用户的实时操作：
- 调度窗口内用户在主页点「停止」 → frpc 立即停；下一次启动触发（次日 8:00）
  才会再起来，本窗口内不再尝试重连
- 调度窗口内用户手动「启动」 → frpc 起来；本窗口结束时 `stop_time` 触发
  仍会执行（幂等，不会因「已在跑」而出错）

#### 时区

`Local::now()`（chrono `clock` feature）——始终用用户本机时间，无时区配置。
跨时区出差的场景需用户自行调整设备时间或临时关闭主开关。

#### 新增依赖

`Cargo.toml` 加：

```toml
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

只启用 `clock` 特性，关掉默认的 `std` / `clock` / `wasmbind` / `winapi` 等
冗余 feature，编译产物最小化。

## 11. 常用开发流程

```bash
# 根目录
pnpm tauri dev                # 联调（前端 + Rust）
pnpm tauri build              # 当前平台打包

# 仅 Rust
cd src-tauri
cargo check                   # 类型检查
cargo build                   # 编译
cargo build --release         # release 编译

# 产物（release）
# macOS   : src-tauri/target/release/bundle/{dmg,macos}
# Windows : src-tauri/target/release/bundle/nsis
```

## 12. 写新功能时的检查清单

- [ ] 新 `#[tauri::command]` 在 `invoke_handler!` 注册
- [ ] 入参 / 出参类型实现 `Serialize` / `Deserialize`，字段名与前端一致
- [ ] 与前端约定的协议（事件 / 命令名）变更时，**先同步 `src/AGENTS.md`**
- [ ] 写文件 / 删文件都用 `atomic_write`，不要直接 `std::fs::write` 重要文件
- [ ] 涉及 sidecar 之外的命令执行时，更新 `capabilities/default.json` 的 `shell:allow-execute`
- [ ] 自更新涉及下载时，**SHA256 校验**必不可少
- [ ] 错误统一 `Result<_, String>`，错误消息**面向用户可读**（含中文）
- [ ] 异步函数显式 `async`，**不阻塞** `tauri::async_runtime` 之外的线程
- [ ] `BUNDLED_FRPC_VERSION` 升级时记得同步 `binaries/` 内的二进制
- [ ] 触发 `start_frpc` 的 setup 入口：参考 §14「触发 start_frpc 的公共模式」；
      同一 setup hook 内的多个 `maybe_*` 共享顶层 `prefs` 与 `auto_launched`，
      **禁止**内部重新 `prefs::load` / 重新扫 `std::env::args`（参见 §5.2.1）
- [ ] 文档注释（`///`）只写 WHY：约束、不变式、坑、设计理由；**禁止**
      解释函数签名 / 类型 / 字段名（identifier 自明）
- [ ] Mutex poison 处理风格统一：poison 时透传 `Err` 或 `emit_log`，
      **不要** `lock().map(...).unwrap_or(false)` 静默绕过——
      poison 几乎不会发生，但绕过 poison 即绕过幂等性
- [ ] 新增 `Prefs` 字段前查根目录 `AGENTS.md` 术语表：UI 文案 / 前端标识
      / 后端字段三处必须对齐；后端 `#[serde(default)]` 字段命名与前端 TS
      类型完全一致

## 13. 反模式（不要做）

- ❌ 在命令里 `std::thread::spawn` —— 阻塞 IO 用 `tauri::async_runtime::spawn_blocking`；纯异步逻辑用 `tauri::async_runtime::spawn`
- ❌ 把 `CommandChild` 暴露到前端 —— 仅保留后端 `Mutex<Option<CommandChild>>`
- ❌ 直接 `std::fs::remove_file(sidecar_path)` —— 必须经过 `apply_pending_update`
- ❌ 引入 toml / async-tar 等重型 crate —— 本项目刻意保持精简
- ❌ 跳过 SHA256 校验直接写 sidecar —— 自更新链路必须可信
- ❌ 在 setup hook 内做重活阻塞启动 —— frpc 自更新接管是允许的（快、原子）
- ❌ 修改窗口尺寸 / decorations 时只改 `tauri.conf.json` —— 还要查
  `tauri.macos.conf.json` 是否覆盖
- ❌ 在 capabilities 里加未使用的权限（如 `shell:allow-stdin-write`，本项目
  从未向 frpc stdin 写）—— 遵循最小权限原则
- ❌ 同一 setup hook 内多次 `prefs::load(app)` 读取同一份 `prefs.json` ——
    顶层缓存后传引用（参见 §5.2.1）
- ❌ 同一 setup hook 内多次 `std::env::args().any(...)` —— 顶层算一次后
    传 `bool`（参见 §5.2.1）
- ❌ 跨多文件保留两份「判定 + spawn + load + start」25 行级别的同构实现 ——
    一旦出现第 3 个触发源（快捷键 / RPC / 其它事件），立即抽公共 helper
    （参见 §14「触发 start_frpc 的公共模式」）

## 14. 触发 start_frpc 的公共模式

「**外部触发 + 幂等跳过 + 加载 StartArgs + start_frpc + 失败日志**」
是 OS 自启 / 调度 / 未来快捷键 / RPC 等所有非用户主动点击路径的同构骨架：

```text
fn maybe_X(app: &AppHandle, prefs: &Prefs, auto_launched: bool) {
    if !precondition(prefs, auto_launched) { return; }
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        fire_X(&app_clone).await;
    });
}

async fn fire_X(app: &AppHandle) {
    let state = match app.try_state::<FrpcState>() { Some(s) => s, None => return };
    // 幂等：已跑则跳过（start_frpc 内部会再次互斥，这是双保险）
    if state.child.lock().map(|g| g.is_some()).unwrap_or(false) { return; }
    let args = match config::load_config(app.clone()) {
        Ok(Some(c)) => c,
        _ => {
            frpc_state::emit_log(app, "system", "<X>触发启动，但尚未配置服务端，跳过".into());
            return;
        }
    };
    if let Err(e) = process::start_frpc(app.clone(), state, args).await {
        frpc_state::emit_log(app, "system", format!("<X>启动失败：{e}"));
    }
}
```

> 当前 `prefs.rs::maybe_auto_connect` + `fire_auto_connect` 与
> `scheduler.rs::maybe_catch_up_start` + `try_fire_start` 是该模式的两份实例，
> 仅在「判定条件」「日志文案」上分叉。

**约定**：

- 该模式**出现 3 个及以上实例时**，抽 `fire_start_if_idle(app, label)` 公共
  helper 到 `process.rs`（或新建 `frpc_runner.rs`），各 `maybe_*` 入口只传
  `label`（用于拼日志文案）。**当前 2 个实例不抽**——与前端 Rule of Three
  同理，避免抽象成本高于重复成本
- 幂等预检查**不能**完全消除竞态：`start_frpc` 内部会再次取锁作为兜底。
  预检查的真正价值是「减少 `Err("核心引擎已在运行中")` 误导性日志」，
  不是互斥保证
- 日志文案「<X>触发启动」中的 `<X>` 应是中文用户可读短语（「开机自动连接」
  /「调度」/ 未来可能的「快捷键」），不直接用标识符
