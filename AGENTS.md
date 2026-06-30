# MoonProxy

跨平台、面向非技术用户的 [frp](https://github.com/fatedier/frp) 桌面客户端，
基于 Tauri v2 构建，支持 Windows 与 macOS。

> 用户视角的介绍请看 [README.md](./README.md)；本文档面向在仓库里改代码的工程师。

## 功能

- 用户填入自己的 frps 服务端（地址、端口、Token、用户名），或预置自定义服务商快速切换
- 可视化管理代理规则（TCP / UDP / HTTP / HTTPS），主页实时显示本地端口连通性
- 一键启动 / 停止 frpc：圆形大按钮分 4 态（已停止 / 连接中 / 已连接 / 连接错误），
  连接状态由 frpc 自身证据支撑（`/api/status` 探测 + 30s 超时回退）
- 系统托盘常驻：关闭窗口默认隐藏到托盘，frpc 继续后台运行
- 开机启动 + 静默启动（自启时隐藏到托盘）
- 开机自动连接（仅 OS 自启路径触发，手动启动不会误连）
- 定时连接（按星期多选 + 起止时间，分钟对齐 + 启动补跑 + 热加载）
- frpc 引擎自更新：GitHub Release → SHA256 校验 → 原子替换，无需重装应用
- 应用本体自更新：基于 `tauri-plugin-updater` 的「重启并安装」
- 通过 Tauri sidecar 机制内置 frpc 二进制，用户无需单独安装

## 技术栈

- 前端：Vue 3 + TypeScript + Vite 6
- 后端：Rust + Tauri v2
- frpc 版本：v0.69.1（已内置；可通过 `pnpm sync:frpc` 同步其他平台）

## 目录结构

```
moonproxy-desktop/
├── src/                          # Vue 前端
│   ├── App.vue                   # 顶层壳：TitleBar + 视图路由 + 全局键盘 / 右键 + 事件订阅
│   ├── main.ts                   # 挂载入口（按 ?view= 区分主窗 / 日志窗）
│   ├── state/                    # 跨组件响应式状态（拆分到各文件）
│   │   ├── index.ts              # 统一导出
│   │   ├── runtime.ts            # 运行时：frpc 状态 / 日志 / 启动时间
│   │   ├── config.ts             # 服务商 / 代理配置
│   │   ├── prefs.ts              # 偏好：语言 / 启动项 / 定时
│   ├── styles.css                # HSL 设计令牌 + 通用组件类
│   ├── composables/              # 按主题拆分的可复用状态 / 命令
│   │   ├── useToast.ts
│   │   ├── useFrpcUpdate.ts      # frpc 引擎自更新
│   │   ├── useAppUpdate.ts       # 应用本体自更新
│   │   └── useProxyHealth.ts     # 主页端点健康点 + 指数退避轮询（3→6→12→24s）
│   ├── components/
│   │   ├── TitleBar.vue          # 跨平台标题栏
│   │   ├── CloseConfirm.vue      # frpc 运行时的关闭确认弹窗
│   │   ├── Toast.vue             # 顶部 Toast 渲染
│   │   └── settings/             # 设置面板 Tab 子组件
│   │       ├── ProviderTab.vue   # 服务商
│   │       ├── ProxyTab.vue      # 代理规则
│   │       ├── InterfaceTab.vue  # 界面语言切换
│   │       ├── LaunchTab.vue     # 开机启动 / 静默启动 / 开机自动连接
│   │       ├── ScheduleSection.vue  # 定时连接（LaunchTab 内嵌）
│   │       ├── LogsTab.vue       # 运行日志
│   │       └── AboutTab.vue      # 关于（含软件更新 + 核心引擎）
│   └── views/
│       ├── HomeView.vue          # 主面板：4 态大圆按钮 / 端点列表 / 引导卡片
│       └── SettingsView.vue      # 设置面板：分段控件 + Tab 切换
├── src-tauri/
│   ├── src/
│   │   ├── main.rs               # Windows release 抑制控制台
│   │   ├── lib.rs                # Tauri Builder 配置 + setup hook + invoke_handler + ExitRequested 兜底
│   │   ├── types.rs              # 共享类型：StartArgs / ProxyConfig
│   │   ├── config.rs             # frpc.toml 生成 + 客户端配置持久化
│   │   ├── process.rs            # frpc 子进程生命周期
│   │   ├── frpc_state.rs         # 连接状态机 + 日志环形缓冲
│   │   ├── proxy_health.rs       # 代理本地端口连通性探测
│   │   ├── frpc_update.rs        # frpc 自更新
│   │   ├── prefs.rs              # 应用偏好
│   │   ├── scheduler.rs          # 按星期定时启停 frpc
│   │   ├── tray.rs               # 系统托盘
│   │   └── assets/               # 编译期嵌入资源（托盘图标）
│   ├── binaries/                 # frpc sidecar 二进制（按平台目标命名；不入库，本地放置）
│   ├── capabilities/
│   │   ├── default.json          # 主窗权限
│   │   └── logs.json             # 独立日志窗权限
│   ├── tauri.conf.json           # 窗口 400×740、无装饰、sidecar 声明、updater 端点
│   └── tauri.macos.conf.json     # macOS 平台覆盖：系统交通灯 + Overlay + hiddenTitle
├── docs/app-icons/               # 图标设计源（APP Icon + 托盘图标），含 README 规范
└── scripts/                      # 仓库辅助脚本
    ├── sync-frpc.sh              # 按 .env 版本同步 frpc 二进制到 src-tauri/binaries/
    └── check-icons.py            # 图标规范校验（更新图标后必跑）
```

> 前端 / 后端开发约定详见各自目录下的 `AGENTS.md`（`src/AGENTS.md`、`src-tauri/AGENTS.md`）。

## 配置与数据存储

按语义拆成三套独立存储，全部落到 Tauri 标准 `app_config_dir()`
（macOS：`~/Library/Application Support/<bundle-identifier>/`；
Windows：`%APPDATA%\<bundle-identifier>\`），互不污染。

| 数据           | 存储后端                  | 文件                    | 键 / 格式       | 管理模块                                  | 写入时机                                                                 |
| -------------- | ------------------------- | ----------------------- | --------------- | ----------------------------------------- | ------------------------------------------------------------------------ |
| 应用偏好       | `tauri-plugin-store`      | `prefs.json`            | `auto_launch` / `silent_start` / `auto_connect`（bool）+ `schedule`（`Schedule` 对象：`enabled` / `weekdays[7]` / `start_time` / `stop_time`，按星期定时启停 frpc）+ `language`（字符串） | `src-tauri/src/prefs.rs`                  | `LaunchTab` / `InterfaceTab` 保存；`set_auto_launch` 先写 OS 启动项再以实际状态回填；`auto_connect` 仅在 `--auto-launched`（OS 自启）时触发 `start_frpc`；`schedule` 由 `scheduler.rs` 每分钟重读热加载 |
| 客户端配置     | `tauri-plugin-store`      | `config.store.json`     | 单键 `start_args`（整体序列化 `StartArgs`） | `src-tauri/src/config.rs` `save_config`/`load_config` | 设置面板「服务商」「代理」Tab 保存                                        |
| frpc 运行配置  | 直接 `std::fs::write`     | `app_config_dir()/frpc.toml` | TOML（手写 `format!`，**未引入 toml crate**） | `src-tauri/src/process.rs::start_frpc`（实际拼装在 `config::build_toml`） | **每次点击启动**按当前 `StartArgs` 重新拼装并覆盖                        |

要点：

- `prefs.json` 与 `config.store.json` 是**同一个 `tauri-plugin-store` 插件下的两个独立 store 文件**，前者存应用偏好，后者存用户填的服务端 / 代理配置。
- `frpc.toml` 不是用户配置，而是从 `StartArgs` **派生**的运行时产物——不进 store，每次启动覆盖写入，供 frpc sidecar 进程读取。
- `auto_launch` 字段以 OS 注册表 / LaunchAgent 的实际状态为准（避免插件在某些平台 silently 失败导致 UI 与系统状态不一致）。
- 字段定义、TOML 转义规则、命令清单等细节见 `src-tauri/AGENTS.md` §3 / §5.2 / §6。

> 另：`app_config_dir()/frpc_update.json` 是 frpc 自更新状态机（`current` / `pending` 版本与待安装二进制路径），由 `frpc_update.rs` 自管，不属于用户可编辑配置。

### 版本号同步规范

发布新版本时，**三处**版本号必须手动保持一致：

| 位置 | 字段 |
| --- | --- |
| `src-tauri/tauri.conf.json` | `version` |
| `package.json` | `version` |
| `src-tauri/Cargo.toml` | `version` |

前端运行时镜像 `src/composables/useAppUpdate.ts::APP_VERSION` 是同一版本号的**第四处**（应用本体自更新流程用），与前三处同步。

> **不要**把 `Cargo.lock` 算进同步点：它由 `cargo` 自动维护，`Cargo.toml` 改了之后 `cargo build` 会自动刷新。
> **不要**把 `useAppUpdate.ts` 自己说成「同步点」——它就是 `APP_VERSION` 常量的**定义位置**，不是被同步对象。

修改版本号的标准流程：改上述三 / 四处 → `cargo check` 刷新 `Cargo.lock` → commit。

CI 通过 `git tag v<x.y.z>` 触发 Release 构建（详见 `.github/workflows/release.yml`），不直接读 package.json 的 version 字段做发布决策。

### 术语表（UI 文案 ↔ 代码标识 ↔ 后端字段）

为避免用户可见文案与代码注释 / 文档术语漂移，新增功能时按下表对齐：

| UI 文案（用户可见，i18n 文案基准） | 前端标识 | 后端字段 | 备注 |
| --- | --- | --- | --- |
| 开机启动 | `auto_launch` | `Prefs::auto_launch` | OS 注册表 / LaunchAgent 实际状态为准 |
| 静默启动 | `silent_start` | `Prefs::silent_start` | 自启时隐藏到托盘 |
| 开机自动连接 | `auto_connect` | `Prefs::auto_connect` | 自启后自动拉起 frpc；**不要**简写为「开机自连」 |
| 定时连接 | `schedule` | `Prefs::schedule` | 按星期多选 + 起止时间 |
| 核心引擎 | — | `frpc` sidecar | frpc 子进程；UI 统一叫「核心引擎」不直呼「frpc」 |

## 开发

```bash
pnpm install
pnpm tauri dev
```

## 打包

```bash
# 当前平台
pnpm tauri build

# macOS：仓库附带 tauri.macos.conf.json 平台覆盖
# 产物位于 src-tauri/target/release/bundle/
#   macOS  → .dmg / .app
#   Windows→ .exe（NSIS 安装器；需在 Windows 机器上或 CI 上构建）
```

正式发布由 GitHub Actions 在 tag 推送时触发，详见 [CONTRIBUTING.md](./CONTRIBUTING.md) §Release 流程。

## 跨平台 frpc 二进制管理

Tauri 通过文件名后缀识别 sidecar 目标平台：

| 平台               | 文件名                            |
| ------------------ | --------------------------------- |
| macOS (Apple Silicon) | `frpc-aarch64-apple-darwin`    |
| macOS (Intel)      | `frpc-x86_64-apple-darwin`        |
| Windows (x64)      | `frpc-x86_64-pc-windows-msvc.exe` |
| Windows (ARM64)    | `frpc-aarch64-pc-windows-msvc.exe`|

补齐平台推荐用 `pnpm sync:frpc`（即 `scripts/sync-frpc.sh`）一键下载并按规范命名到
`src-tauri/binaries/`，或手动把对应 `frp_*` 发布包里的 `frpc` 重命名为
上表名称放入。

下载地址：<https://github.com/fatedier/frp/releases>

## 图标与托盘资源更新工作流

> **触发时机**：当设计师 / 开发更新了 `docs/app-icons/` 下的任何图标
> （APP Icon、macOS / Windows 托盘图标），**必须**先跑校验脚本再提交。

### 一键校验

```bash
./scripts/check-icons.py            # 普通模式（默认）
./scripts/check-icons.py --strict   # 严格模式（CI 用，警告也算失败）
```

退出码：`0` 通过 / `1` 有违规 / `2` 环境错误（如缺 Pillow）。

### 校验内容

- **APP Icon**：尺寸匹配命名约定（如 `icon_64.png` 必须 64×64）
- **macOS 托盘** `tray_macos_44.png`：44×44 + 纯黑 `#000000` + 4 角透明
- **Windows 托盘** `tray_windows_64.png`：64×64 + 纯白 `#FFFFFF` + 4 角透明

### 设计规范

详细规范、常见错误与修复指引见 [`docs/app-icons/README.md`](docs/app-icons/README.md)。

### CI 接入

PR 中改动了 `docs/app-icons/**` 的资产时，CI 应运行：

```bash
./scripts/check-icons.py --strict
```

## 使用说明

1. 首次启动主页显示「尚未配置」引导卡片，点击「去配置」进入设置面板
2. 在「服务商」Tab 选「自定义」，填写名称、服务器地址、端口、Token、用户名
3. 在「代理」Tab 添加需要穿透的本地服务：名称、类型、本机地址端口、公网映射端口
4. 保存后回到主页，点击大圆形按钮启动 frpc；按钮颜色和图标会随状态变化
   （已停止 / 连接中 / 已连接 / 连接错误），主页代理行前置的圆点实时显示
   本地端口是否可达
5. 「日志」Tab 实时查看 frpc 输出；运行日志、软件更新、引擎更新都在设置面板里
