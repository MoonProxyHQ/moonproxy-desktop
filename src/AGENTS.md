# 前端开发指南（MoonProxy）

> 本文件面向在该目录（`src/`）下做前端开发的工程师。涉及前后端协议的部分
> 也给出对应后端命令 / 事件的引用，便于一眼对齐。

## 1. 技术栈

- **框架**：Vue 3（`<script setup lang="ts">`）
- **构建**：Vite 6
- **类型**：TypeScript 5.6（`strict`、`noUnusedLocals`、`noUnusedParameters`）
- **桥接**：`@tauri-apps/api`（`core` / `event` / `window`）+ 官方插件
  - `@tauri-apps/plugin-dialog`：原生对话框
  - `@tauri-apps/plugin-fs`：文本 / 目录读写
  - `@tauri-apps/plugin-opener`：外部链接
  - `@tauri-apps/plugin-shell`：副作用留作扩展位（侧车执行由后端封装）
- **状态**：原生 `ref` / `reactive`，**不引入** Pinia / Vuex
- **样式**：手写 CSS（无 Tailwind / UnoCSS），遵循 `styles.css` 中的 HSL 令牌

## 2. 目录结构与文件职责

```
src/
├── main.ts                       # 挂载入口；按 URL `?view=logs` 分流到 LogsWindow，否则挂 App
├── App.vue                       # 主窗顶层壳：TitleBar + 视图路由 + 全局键盘/右键监听；事件订阅已抽到 useAppEvents
├── types.ts                      # 前后端共享类型（ProxyConfig / FrpcConfig / Prefs / FrpcStatus / LogEntry）
├── state/                        # 核心响应式状态，按主题拆分（详见 §3.2）
│   ├── index.ts                  # 聚合 barrel：仅做 `export * from "./xxx"`，不放任何状态
│   ├── config.ts                 # config + isConfigured + toArgs
│   ├── prefs.ts                  # prefs（应用偏好）
│   ├── runtime.ts                # frpcStatus / frpcError / running / logs
├── commands/                     # 按职责拆分的 invoke 封装（全部吞异常、返回 string | null | boolean）
│   ├── config.ts                 # loadConfig / saveConfig
│   ├── contextMenu.ts            # showEditMenu（输入框原生右键编辑菜单）
│   ├── frpc.ts                   # startFrpc / stopFrpc
│   └── prefs.ts                  # loadPrefs / savePrefs / setAutoLaunch / refreshAutoLaunch
├── styles.css                    # 设计令牌（HSL）+ 通用组件类（.btn / .input / .card / .badge）
├── vite-env.d.ts                 # *.vue 模块声明
├── composables/
│   ├── useToast.ts               # 轻量 Toast（showToast / dismiss timer）
│   ├── useFrpcUpdate.ts          # frpc 自更新：版本 / updateInfo / 下载 / 横幅相关
│   ├── useAppUpdate.ts           # 应用本体自更新
│   ├── useProxyHealth.ts         # 主页端点健康点：proxyHealth + 指数退避轮询（3→6→12→24s）
│   ├── useAppEvents.ts           # 应用级事件订阅 + 启动初始化（App.vue 已委托）
│   └── useLogsWindow.ts          # 打开/聚焦独立日志窗口（WebviewWindow label="logs"）
├── components/
│   ├── TitleBar.vue              # 跨平台标题栏：mac 交通灯避让、Win 最小化/关闭、拖动区；「服务」「设置」（仅 home）与「返回」（仅非 home）均按 OS 分槽（macOS 右 / Windows 左，远离系统窗口控件）
│   ├── CloseConfirm.vue          # frpc 运行时的关闭确认弹窗（最小化 / 退出）
│   ├── Toast.vue                 # 顶部 Toast 渲染
│   ├── home/                     # HomeView 拆出的子组件（详见 §5.4）
│   │   ├── CircleButton.vue      # 大圆按钮 + Canvas 波纹粒子系统 + 4 态文案
│   │   ├── ProxyList.vue         # 公网访问地址列表 + 健康点 + 复制按钮 + 指数退避健康轮询
│   │   ├── GuideCard.vue         # 未配置引导卡片
│   │   └── SystemStatus.vue      # 底部只读系统状态栏（开机启动 / 定时连接）
│   ├── banners/
│   │   └── UpdateBanners.vue     # 顶部 4 类横幅：frpc 错误条 / 软件本体更新 / 引擎已应用 / 引擎待应用
│   └── settings/                 # 设置面板 Tab 子组件
│       ├── ProviderTab.vue
│       ├── ProxyTab.vue
│       ├── InterfaceTab.vue      # 界面语言切换
│       ├── LaunchTab.vue         # 开机启动 / 静默启动 / 开机自动连接（ScheduleSection 抽出独立子件）
│       ├── ScheduleSection.vue   # 定时连接：主开关 + 星期选择 + 起止时间 + 校验 + 保存
│       ├── LogsTab.vue           # 运行日志
│       └── AboutTab.vue          # 关于（含软件更新 + 核心引擎）
└── views/
    ├── HomeView.vue              # 主面板：纯组装（CircleButton + GuideCard + ProxyList + SystemStatus + 错误条 + 启停逻辑）
    ├── SettingsView.vue          # 设置面板：分段控件 + Tab 切换
    └── LogsWindow.vue            # 独立日志窗口根组件：get_logs 拉历史 + listen 实时；不复用 App.vue 的关闭/快捷键逻辑
```

## 3. 状态层

> 设计原则：**单例、扁平、纯响应式**。所有跨视图共享状态按主题拆分到独立模块，
> 视图组件只读 + 通过封装的命令函数修改。
>
> - `types.ts`：前后端共享类型（snake_case，与 Rust 一一对应）
> - `state/` 子目录：核心响应式状态，按主题拆为 `config` / `prefs` / `runtime`
>   三个模块；统一经 `state/index.ts` barrel 暴露；
>   `isConfigured` / `toArgs` 留在 `state/config.ts`（仅服务 config）
> - `commands/config.ts` / `commands/frpc.ts` / `commands/prefs.ts`：按职责拆分的 invoke 封装
> - `composables/useFrpcUpdate.ts`：frpc 引擎自更新相关状态
> - `composables/useAppUpdate.ts`：应用本体自更新相关状态
> - `composables/useProxyHealth.ts`：代理本地端口连通性
> - `composables/useAppEvents.ts`：应用级 Tauri 事件订阅 + 启动初始化（App.vue 已委托）

### 3.1 类型

`types.ts`：

```ts
// ProxyConfig 是按 `type` 拆分的 discriminated union——每种 frp 代理类型
// 有独立的 schema（TCP/UDP 走 remotePort，HTTP/HTTPS 走 customDomains 且
// 不接受 remotePort）。聚合在扁平结构里会让 build_toml / URL 生成路径
// 都需按字符串 type 分叉，且无法在编译期排除非法字段。
type ProxyConfig =
  | { type: "tcp" | "udp"; name: string; local_ip: string;
      local_port: number; remote_port: number }
  | { type: "http" | "https"; name: string; local_ip: string;
      local_port: number; custom_domains: string[] };
interface FrpcConfig {
  custom_name: string;   // 自定义服务商显示名称
  server_addr: string; server_port: number;
  token: string; user: string;
  proxies: ProxyConfig[];
}
interface Prefs {
  auto_launch: boolean;  // 开机启动（OS 实际状态）
  silent_start: boolean; // 静默启动：开机自启时隐藏到托盘
  auto_connect: boolean; // 开机自动连接：OS 自启后自动拉起 frpc（仅 --auto-launched 触发）
  schedule: Schedule;    // 定时连接配置
}
interface Schedule {
  enabled: boolean;       // 主开关；false 时调度器与启动补跑均跳过
  weekdays: [boolean, boolean, boolean, boolean, boolean, boolean, boolean];
  // 下标 0=周一 … 6=周日；与 chrono::Weekday::num_days_from_monday 一一对应
  start_time: string;     // "HH:MM" 24 小时制
  stop_time: string;      // "HH:MM" 24 小时制
}
type FrpcStatus = "stopped" | "connecting" | "connected" | "error";
interface LogEntry {
  stream: "stdout" | "stderr" | "system";
  line: string;
}
```

`UpdateInfo`（frpc 远端版本信息）定义在 `composables/useFrpcUpdate.ts`；
`AppUpdateInfo`（应用本体远端版本信息）定义在 `composables/useAppUpdate.ts`；
`ProxyHealth` 定义在 `composables/useProxyHealth.ts`。

### 3.2 状态（按模块分组）

**`state/config.ts`**：

| 标识            | 类型                              | 含义                                                  |
| --------------- | --------------------------------- | ----------------------------------------------------- |
| `config`        | `reactive<FrpcConfig>`            | 已保存的服务端 / 代理配置；由 `commands/config.ts` 从 `config.store.json` 加载 / 写回 |
| `isConfigured()` | `() => boolean`                  | 是否已完成初始配置（有服务端地址且至少一条代理）；用于主页大圆按钮 disabled 态 |
| `toArgs()`      | `() => StartArgs`                | 序列化为 Rust 端 `StartArgs`：trim / Number 化，空字符串 → `null`；`startFrpc` / `saveConfig` 调用入口 |

**`state/prefs.ts`**：

| 标识      | 类型                              | 含义                                                  |
| --------- | --------------------------------- | ----------------------------------------------------- |
| `prefs`   | `reactive<Prefs>`                 | 应用偏好（开机启动 / 静默启动 / 开机自动连接 / 定时连接 / 界面语言）；通过 `tauri-plugin-store` 持久化到 `prefs.json`（与 frpc 配置 `config.store.json` 是两个 store 文件）；`auto_launch` 字段以 OS 实际状态为准 |

**`state/runtime.ts`**：

| 标识          | 类型                              | 含义                                                  |
| ------------- | --------------------------------- | ----------------------------------------------------- |
| `frpcStatus`  | `Ref<FrpcStatus>`                 | frpc 连接状态；`connected` 仅由后端通过 frpc `/api/status` 推导派发 |
| `frpcError`   | `Ref<string \| null>`             | 进入 `error` 时的提示文案；状态变更即清空             |
| `running`     | `ComputedRef<boolean>`            | **派生**：`frpcStatus.value !== 'stopped'`；保留以兼容旧调用点 |
| `logs`        | `ShallowReactive<LogEntry[]>`     | 实时日志缓冲（前端自带 500 条上限）                   |

**`composables/useFrpcUpdate.ts`**：

| 标识                | 类型                              | 含义                                                  |
| ------------------- | --------------------------------- | ----------------------------------------------------- |
| `frpcVersion`       | `Ref<string>`                     | 当前生效的 frpc 版本                                  |
| `updateInfo`        | `Ref<UpdateInfo \| null>`         | 远端可下载的新版本信息                                |
| `downloadedPending` | `Ref<string \| null>`             | 已下载、待下次启动应用生效的版本                      |
| `recentlyApplied`   | `Ref<string \| null>`             | 本次启动相对上次启动升级到的新版本（用于横幅提示）    |
| `downloading`       | `Ref<boolean>`                    | 是否正在下载新版本                                    |

**`composables/useAppUpdate.ts`**：

| 标识                    | 类型                              | 含义                                                  |
| ----------------------- | --------------------------------- | ----------------------------------------------------- |
| `APP_VERSION`           | `string`                          | 应用本体版本号（三处同步：`tauri.conf.json` / `package.json` / `Cargo.toml`）|
| `appUpdateAvailable`    | `Ref<AppUpdateInfo \| null>`      | 检测到的应用本体新版本                                |
| `appUpdatePending`      | `Ref<Update \| null>`             | 已下载、待用户点「重启并安装」的 updater Update 句柄  |
| `appUpdateChecking`     | `Ref<boolean>`                    | 是否正在检查                                          |
| `appUpdateDownloading`  | `Ref<boolean>`                    | 是否正在下载                                          |
| `appUpdateProgress`     | `Ref<number>`                     | 下载进度（粗粒度 0–100）                              |

**`composables/useProxyHealth.ts`**：

| 标识          | 类型                                       | 含义                                                  |
| ------------- | ------------------------------------------ | ----------------------------------------------------- |
| `proxyHealth` | `Ref<(ProxyHealth \| undefined)[]>`        | 每条代理本地端口连通性，下标与 `config.proxies` 对齐；未检测项为 `undefined` |

### 3.3 命令封装

所有命令都返回 `string | null`（错误消息）或 `void`（`loadConfig` 例外，返回 `boolean`），**前端不抛异常**：

**`commands/config.ts`**：

| 函数                       | 对应后端 `invoke`          | 用途                                       |
| -------------------------- | -------------------------- | ------------------------------------------ |
| `loadConfig()`             | `load_config`              | 启动时拉取已保存配置；返回 `true` 表示命中 |
| `saveConfig()`             | `save_config`              | 持久化；错误透传                           |

**`commands/frpc.ts`**：

| 函数                       | 对应后端 `invoke`          | 用途                                       |
| -------------------------- | -------------------------- | ------------------------------------------ |
| `startFrpc()`              | `start_frpc`               | 启动 frpc 子进程                           |
| `stopFrpc()`               | `stop_frpc`                | 停止 frpc 子进程                           |

**`commands/prefs.ts`**：

| 函数                       | 对应后端 `invoke`          | 用途                                       |
| -------------------------- | -------------------------- | ------------------------------------------ |
| `loadPrefs()`              | `get_prefs`                | 启动时拉取应用偏好（开机启动 / 静默启动）   |
| `savePrefs()`              | `save_prefs`               | 持久化偏好（不触发 OS 动作，仅写 store）   |
| `setAutoLaunch(b)`         | `set_auto_launch`          | 写 OS 启动项并以 OS 实际状态回填 prefs     |
| `refreshAutoLaunch()`      | `get_auto_launch`          | 启动时校正 OS 实际开机启动状态到 prefs     |

**`commands/contextMenu.ts`**：

| 函数                       | 对应后端 `invoke`          | 用途                                       |
| -------------------------- | -------------------------- | ------------------------------------------ |
| `showEditMenu()`           | `show_edit_menu`           | 弹出原生编辑菜单（剪切/复制/粘贴/全选），仅 input/textarea/select 右键时调 |

**`composables/useFrpcUpdate.ts`**：

| 函数                       | 对应后端 `invoke`          | 用途                                       |
| -------------------------- | -------------------------- | ------------------------------------------ |
| `initFrpcVersion()`        | `get_frpc_version`         | 拉取当前版本并比对 localStorage 设置横幅   |
| `checkFrpcUpdate()`        | `check_frpc_update`        | 后台静默查 GitHub Release                  |
| `downloadFrpcUpdate(v)`    | `download_frpc_update`     | 下载到 pending 目录                        |

**`composables/useProxyHealth.ts`**：

| 函数                       | 对应后端 `invoke`          | 用途                                       |
| -------------------------- | -------------------------- | ------------------------------------------ |
| `checkProxiesHealth()`     | `check_proxies_health`     | 批量探测代理本地端口连通性，刷新 `proxyHealth` |

**`composables/useAppUpdate.ts`**（基于 `@tauri-apps/plugin-updater`，非自研 invoke）：

| 函数                       | 用途                                       |
| -------------------------- | ------------------------------------------ |
| `checkAppUpdate()`         | 后台静默查 GitHub Release（应用本体）       |
| `downloadAppUpdate()`      | 下载到本地缓存，成功后置 `appUpdatePending` |
| `installAppUpdate()`       | 安装并 `relaunch()` 重启                    |

调用模板：

```ts
const err = await startFrpc();
if (err) showToast(err, "error");
```

### 3.4 序列化到后端 `StartArgs`

`toArgs()`（`state/config.ts`）在写入 / 启动时统一 trim / Number 化，**空字符串
→ `null`**（后端据此决定是否写入 `auth.token` / `user` 字段）。

## 4. 与后端的事件协议

事件订阅已抽到 `composables/useAppEvents.ts`，由 `App.vue` 在 setup 顶层调用。
`useAppEvents` 在 `onMounted` 中注册四类事件，在 `onUnmounted` 中统一 unlisten。

| 事件名                    | 载荷                                | 来源               | 前端处理                              |
| ------------------------- | ----------------------------------- | ------------------ | ------------------------------------- |
| `frpc://log`              | `{ stream: 'stdout' \| 'stderr' \| 'system'; line: string }` | 后端 stdout/stderr 解析 + 系统消息 | 推入 `logs.value`，超过 500 条 shift 掉最旧 |
| `frpc://status`           | `{ status: 'stopped' \| 'connecting' \| 'connected' \| 'error'; error: string \| null }` | start_frpc 设 `connecting`；轮询任务推 `connected` / `error`；stop_frpc 与 `Terminated` 设 `stopped` | 同步 `frpcStatus` / `frpcError`；`error` 时显示顶部红色错误条（由 `components/banners/UpdateBanners.vue` 渲染） |
| `frpc://update-downloaded` | `{ version: string }`               | 后端下载完成       | 置 `downloadedPending`，触发顶部横幅  |
| `onCloseRequested`         | —                                  | 窗口关闭请求       | frpc 运行时弹 `CloseConfirm` 让用户选「最小化 / 退出」；停止时直接 `exit(0)`；详见后端 `src-tauri/AGENTS.md` §10.2 |

> `useAppEvents` 同时负责启动初始化序列（`loadConfig → loadPrefs → setLocale →
> refreshAutoLaunch → frpc_status → initFrpcVersion → checkFrpcUpdate →
> checkAppUpdate`），保证各 store 在子视图首次渲染前已被填好。

## 5. 视图层

### 5.1 路由方式

`App.vue` 用 `ref<View>('home' | 'settings' | 'services')` + `v-if/v-else` 切视图。
**不引入** vue-router。Esc 在非 home 视图（settings / services）下返回 home。

> 独立窗口（如日志窗）不走这套机制——`main.ts` 直接读 `location.search`
> 决定挂载哪个根组件：`?view=logs` → `LogsWindow.vue`，否则 `App.vue`。
> 这样让独立窗口避开 `App.vue` 的 `onCloseRequested` / 全局快捷键 / `TitleBar`
> 等主窗专属逻辑，并允许复用同一份前端 bundle。

### 5.2 编辑副本模式

`SettingsView` 下各 Tab（`ProviderTab` / `ProxyTab` / `InterfaceTab` / `LaunchTab` / `LogsTab` / `AboutTab`）
统一约定：表单绑定本地 `form` 副本，**保存时才写回 `config` / `prefs`**。
这样用户中途取消不会污染 store，也避免每次输入触发后端写入。

### 5.3 Toast

各 Tab 通过 `composables/useToast` 自带 `showToast(msg, type, duration)`（默认 2.5s），
`Toast.vue` 渲染顶部气泡。常见用法：

- 保存成功 → 1.2s 后短提示；保存失败 → 4s 错误提示
- 「重启并安装」等破坏性操作失败 → 4s 错误提示

### 5.4 HomeView 实时性

HomeView 本身是纯组装壳层，所有"实时"职责拆到 `components/home/` 子件：

| 子件 | 职责 |
| --- | --- |
| `CircleButton.vue` | 大圆按钮 + Canvas 波纹粒子系统（`useParticles(frpcStatus)`）+ 4 态文案；只 emit `click`，启停逻辑由 `HomeView.vue` 处理 |
| `ProxyList.vue` | 公网访问地址列表 + 健康点 + 复制按钮 + 指数退避健康轮询（自管理 onMounted/onUnmounted，3→6→12→24s，整体稳定升档 / 配置变化或状态翻转立即回 3s）；按代理类型分支生成地址：`http`/`https` → `${type}://${custom_domains[0]}`（未配域名时回退到 name 占位），`tcp`/`udp` → `${server_addr}:${remote_port}`；点击地址复制到剪贴板（`navigator.clipboard?.writeText`，失败静默） |
| `GuideCard.vue` | 未配置引导卡片；emit `services` |
| `SystemStatus.vue` | 底部只读系统状态栏：开机启动状态 + 定时连接摘要 |

- 独立日志窗口（`LogsWindow.vue`）打开时调用 `get_logs` 拉历史，再
  `listen<LogEntry>("frpc://log")` 实时追加；新条目入栈后 `nextTick` 滚到底
  （`logBox.scrollTop = scrollHeight`），最多保留 500 条
- 日志内容用 ANSI HTML 渲染（颜色保留）；整窗 `user-select: text` 允许选择/复制

### 5.5 TitleBar 跨平台要点

- `navigator.userAgent` 区分 macOS / Windows
- macOS：`--left-slot: 78px` 给系统交通灯让位
- Windows：右槽额外渲染最小化 / 关闭按钮，`-webkit-app-region: no-drag`
- 拖动区使用 `data-tauri-drag-region` + `.slot-center > * { pointer-events: none }`
  防止标题文字吞掉拖动手势
- 「服务」（Server 图标）+「设置」（Settings 图标）两个图标按钮：仅 `home`
  视图渲染，点击 emit `services` / `settings`；位置按 OS 分流——macOS 在右槽
  （与左侧交通灯分立两侧），Windows 在左槽（远离右侧窗口控制按钮）
- 返回按钮（ArrowLeft 图标）：仅 `services` / `settings` 子视图渲染，点击 emit
  `back`；位置同样按 OS 分流——macOS 在右槽，Windows 在左槽（远离系统窗口控件）

### 5.6 代理健康检测（HomeView）

> **目的**：在主页代理行前置一个状态点，告诉用户本地端口是否可达。

#### 状态映射

`composables/useProxyHealth.ts` 中 `proxyHealth` 是
`(ProxyHealth | undefined)[]`，下标与 `config.proxies` 一致。后端返回时已经按
`index` 排好，前端用 `index` 落位而非依赖顺序：

```ts
const map: (ProxyHealth | undefined)[] = new Array(config.proxies.length);
for (const r of results) {
  if (r.index >= 0 && r.index < map.length) map[r.index] = r;
}
proxyHealth.value = map;
```

- **`undefined`**（下标位置没数据）：表示该代理尚未检测过，UI 显示
  "正在检测本地端口…"
- **`ok: true`**：绿色（`.dot-ok`，有微光晕）
- **`ok: false`**：红色（`.dot-fail`，闪烁动画 `dot-blink 1.2s`）

#### 轮询策略（`ProxyList.vue`）

健康轮询由 `components/home/ProxyList.vue` 自管理，采用**指数退避**调度（不是固定间隔）：

- 档位：**3s → 6s → 12s → 24s**，4 档后封顶（3 次倍增）
- 节奏：每轮探测完成后比对 `proxyHealth` 的「所有代理 ok 值串」签名
  - 与上一次签名相同 → streak + 1，下次间隔升一档
  - 与上一次签名不同（任意代理翻转）→ streak = 0，下次间隔回到 3s
- `onMounted` 立即跑一次并启动自调度循环
- `watch(props.proxies, deep)` 触发**立即重检 + 重置退避**——新代理的初始
  signature 必然与上一轮不同，自然落到 3s 起步档
- `onUnmounted` 清理挂起的 `setTimeout`

```ts
async function tickHealth() {
  await checkProxiesHealth();
  const sig = healthSignature();
  healthStreak = (healthPrevSig !== null && sig === healthPrevSig) ? healthStreak + 1 : 0;
  healthPrevSig = sig;
  healthTimer = setTimeout(tickHealth, nextHealthInterval(healthStreak));
}
function nextHealthInterval(streak: number) {
  return Math.min(HEALTH_INTERVAL_MIN * 2 ** streak, HEALTH_INTERVAL_MAX);
}
```

> **为什么退避**：本地端口状态相对稳定时，定时高频探测既耗 `spawn_blocking`
> 又反复打同一个端口。稳定后拉长到 24s，最坏延迟到下一次探测 24s；任一代理
> 翻转立即回 3s 档。改动退避档位 / 封顶值时同步评估「最坏感知延迟」。
>
> **首轮特殊**：首次进入时 `healthPrevSig === null`，签名比对必判为"翻转"
> （streak=0），落到 3s 起步档——与改动前的固定 3s 体验一致。
>
> **后端失败静默**：`checkProxiesHealth` 内部 `try/catch` 只 `console.warn`，
> 不弹错误条——避免频繁轮询失败刷屏；连续失败会显示"正在检测…"，已足够提示。
> **注意**：后端失败时 `proxyHealth.value` **不会更新**（赋值在 try 块内），
> 故 signature 与上次相同，**仍会被判为稳定继续退避**——本地端口状态本身未变，
> 这个行为语义合理；只有真正检测出新结果（ok 值变化）时才会重置退避。

#### 三类辅助函数

```ts
function healthFor(i)    // 取第 i 项，可能是 undefined
function healthClass(i)  // 返回 'dot-ok' | 'dot-fail' | 'dot-pending'
function healthTitle(i)  // 拼给 title / aria-label 的中文文案
function isFailed(i)     // 仅 ok=false 时为 true（用于行高亮等）
```

> 修改文案时三处都要看：状态点 `title`、`aria-label`、UI 文本；保持一致。

## 6. 设计令牌（`styles.css`）

> 全部使用 **HSL 分量**（如 `240 5.9% 10%`），消费时拼成 `hsl(var(--primary))`。
> 修改某个色相只改 `:root` 即可。

| 类别             | 变量                                                   |
| ---------------- | ------------------------------------------------------ |
| 中性背景         | `--background / --foreground / --card`                 |
| 文本             | `--muted-foreground`                                   |
| 状态             | `--primary / --success / --warning / --destructive`    |
| 边框 / 控件      | `--border / --input / --ring`                          |
| 几何             | `--radius: 0.5rem`                                     |
| 字体栈           | `-apple-system, BlinkMacSystemFont, "PingFang SC",` 等 |

通用类（按需扩展）：

- `.btn` + 修饰符（`.btn-primary / .btn-outline / .btn-ghost / .btn-destructive`
  + `.btn-sm / .btn-icon`）
- `.input`（focus 显示 ring）
- `.card`（白底圆角 + 边框）
- `.badge` + `.badge-success / .badge-muted / .badge-destructive`

## 7. 桌面化体验约定

- `body { user-select: none; overscroll-behavior: none; -webkit-user-drag: none; }`
- `input/textarea/select` 显式允许 `user-select: text`
- 右键菜单：可编辑元素（input/textarea/select）弹原生编辑菜单（剪切/复制/粘贴/全选，后端 `show_edit_menu`），其余区域阻止默认菜单（`App.vue::onContextMenu`）
- 全局快捷键（`App.vue::onKeydown`）：
  - `Cmd/Ctrl + W`：关闭窗口（统一走 `App.vue::onCloseRequested` 钩子；frpc 运行时弹 `CloseConfirm.vue` 确认，详见后端 `src-tauri/AGENTS.md` §10.2）
  - `Cmd/Ctrl + M`：原生最小化到任务栏（与 `CloseConfirm` 中的 `hide()` 到托盘语义不同，互不冲突）
  - `Esc`：settings 视图返回 home；`CloseConfirm` 打开时由弹窗独占（关弹窗而非返回 home）

## 8. 常用开发流程

```bash
# 根目录
pnpm install
pnpm tauri dev          # 启动前后端联调
pnpm tauri build        # 当前平台打包
pnpm dev                # 仅前端（http://localhost:1420）
pnpm build              # 仅前端构建
```

## 9. 写新功能时的检查清单

- [ ] 跨视图共享类型放在 `types.ts`；与某主题强绑定的类型（`UpdateInfo` /
      `AppUpdateInfo` / `ProxyHealth`）放在对应 composable
- [ ] 新增 `invoke` 命令时，在 `commands/` 对应子模块或对应 composable 封装成
      `xxxXxx(): Promise<string | null | void>`
- [ ] 新增 `listen` 事件时，**确保 `onUnmounted` 中调用** `unlisten`；
      若属应用级事件（影响所有视图），优先扩 `composables/useAppEvents.ts`
- [ ] 颜色 / 间距统一走 `styles.css` 令牌，不要硬编码 `#fff` / `12px`
- [ ] 文案保持简体中文（除非术语），按钮动词在前（"启动服务" / "停止服务"）；
      新术语加入前先查根目录 `AGENTS.md` 术语表
- [ ] 不要把进程对象、Child 句柄等放到响应式状态里（无意义且会拖慢）
- [ ] 日志展示最大长度仍受 `logs.length > 500` 限制，避免长会话内存膨胀
- [ ] 新增独立窗口（`WebviewWindow`）时，**必须**在 `src-tauri/capabilities/`
      下加对应 `<label>.json`，否则该窗口连 `listen` / `invoke` 都没有权限；
      `?view=...` 入口分流统一在 `main.ts` 完成
- [ ] 涉及修改后端命令名 / 事件名 / 载荷结构时，**先同步后端文档**
      （`src-tauri/AGENTS.md`）再改前端
- [ ] 单文件超过 ~300 行时优先评估是否按职责拆子件（参见 `components/home/` /
      `components/settings/ScheduleSection.vue` 等示例）
- [ ] 新增 toggle 控件时，参考 §11 同构代码模板；同构 toggle **不足 3 个**
      时保留两份复制（Rule of Three），抽 `usePrefToggle(key)` composable
      反而增加抽象成本
- [ ] 新增依赖型偏好（依赖 `auto_launch` 等上游开关的子开关）时，
      `enabled` computed 必须有**独立语义**；若多个 toggle 的 enabled 条件
      完全相同，合并为单个 computed 或直接内联，**禁止**保留两个等价 computed
- [ ] 新增 i18n key 前先 `grep locales/`：跨多个控件复用的「不可用前置提示」
      「成功 / 失败通用文案」应抽公共 key（参考 §12 i18n 去重规则）

## 10. 反模式（不要做）

- ❌ 引入 Pinia / Vuex / 任何状态库——本项目刻意保持轻量
- ❌ 在视图组件里直接 `await invoke(...)`——所有命令必须经 `commands/` /
    对应 composable 封装
- ❌ 任意把后端错误吞掉——按规范透传到 Toast / 错误条
- ❌ 使用 Tailwind / UnoCSS / 任何原子化框架——本项目维持手写 CSS 令牌
- ❌ 给 Windows / macOS 各写一份组件——`TitleBar.vue` 已经做了
  `userAgent` 分支，新功能若涉及平台差异应延续该模式
- ❌ 修改全局 `body { user-select: none }`——会破坏输入控件体验
- ❌ 把多个响应式主题（用户配置 / 应用偏好 / 运行态 / 外部数据）混在
  `state.ts` 单文件——按主题拆到 `state/` 子目录下对应文件
- ❌ 在同一文件内保留两个**完全等价**的 computed（求值函数一字不差）——
    要么合并、要么内联；刻意保留必须在注释里说明 WHY（如「为后续独立演化
    预留」），无注释的重复一律视为冗余
- ❌ 文档注释 / JSDoc 重复 identifier 已自明的信息（「`/** 按钮是否禁用 */`」
    配 `disabled: boolean`）——注释只写 WHY：约束、不变式、坑、设计理由

## 11. 同构代码模板：依赖型偏好 toggle

`LaunchTab.vue` 内「静默启动」「开机自动连接」是典型的**依赖型 toggle**：
启用前提是 `prefs.auto_launch=true`，否则 toggle 禁用并显示「需先开启开机启动」
提示。每个 toggle 由四件套构成：

```ts
// 1. saving ref：避免并发保存
const savingX = ref(false);
// 2. enabled computed：是否可编辑（依赖上游开关）。**多个 toggle 完全相同
//    时合并为单个**（如 `launchDependent`），不要各写一份。
const xEnabled = computed(() => prefs.auto_launch);
// 3. onToggle：乐观更新 + 失败回滚 + Toast
async function onToggleX() {
  if (savingX.value || !xEnabled.value) return;
  const next = !prefs.x;
  savingX.value = true;
  prefs.x = next;
  const err = await savePrefs();
  savingX.value = false;
  if (err) {
    prefs.x = !next;
    showToast(t("msg_save_failed", { err }), "error", 3500);
    return;
  }
  showToast(next ? t("msg_x_on") : t("msg_x_off"), "success", 1500);
}
// 4. desc：按 enabled / value 三态拼文案
function xDesc(): string {
  if (!prefs.auto_launch) return t("launch_blocked_dependency");
  return prefs.x ? t("launch_x_on_desc") : t("launch_x_off_desc");
}
```

模板对应模板段（`<div class="pref-row" :class="{ disabled: !xEnabled }">`）
参见 `LaunchTab.vue` 现有两份实现，复制后改 4 个标识符即可。

> **何时抽 composable**：同构 toggle **达到 3 个**时，再抽
> `usePrefToggle({ key, enabled, messages })` 收敛。当前仅 2 个，保留两份复制
> 是刻意选择（符合 §10 反过度抽象原则）。

## 12. i18n 去重规则

`src/locales/zh-CN.ts` / `en.ts` 必须保持 key 一一对应（缺 key 会被
`vue-i18n` fallback，但易引入隐蔽 bug）。新增 key 时遵循：

- **跨控件复用的通用文案**抽公共 key：
  - 「需先开启开机启动后此项才生效」 → `launch_blocked_dependency`（**不要**
    为每个子开关各起 `launch_X_blocked`）
  - 「保存成功」 / 「保存失败：{err}」 → `msg_save_success` / `msg_save_failed`
- **控件特有文案**就近命名：`launch_<控件名>_on_desc` / `_off_desc`
- **新增 key 前必做**：`grep -r "新key名" src/locales/`，确认没有等价既有 key
