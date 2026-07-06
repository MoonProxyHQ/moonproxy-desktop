# 社交预览图（Social Preview）

GitHub 在分享仓库链接到社交媒体 / 聊天工具时，会展示一张 **1200×630** 的预览图。
若仓库根目录没有这张图，会回退为「无图」或默认头像，**点击率显著下降**。

## 命名与位置

文件名固定为 `social-preview.png`，放在 `.github/`：

```
moonproxy-desktop/
└── .github/social-preview.png   ← 2560×1280，≤ 1MB，PNG
```

> GitHub 官方文档：<https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview>

## 设计要求

- 尺寸：**1280×640**（GitHub 推荐）/ 1200×630（社交媒体通用）
- 内容：
  - 品牌名：**月神代理（MoonProxy）**
  - 一句话定位：跨平台 FRP 内网穿透桌面客户端
  - 平台标识：macOS · Windows
  - 技术栈角标（可选）：Tauri v2 · Rust · Vue
- 色调：与官网 `moonproxy.app` 保持一致（主色 `#2563eb`）
- 不要放：截图全屏（社交媒体会裁切）、过多文字、敏感信息

## 设计资源

- 设计源文件位于 `docs/app-icons/`（APP Icon + 托盘图标）
- 官网已使用的预览图：`../moonbox-website/public/screenshots/main-pic.jpg`（可作参考）
- 品牌字体 / 配色参考：`../moonbox-website/src/styles/global.css`

## 当前状态

- [x] 已落地：`.github/social-preview.png`（2560×1280）
- 临时方案：GitHub 仓库设置中可手动上传一张图作为社交预览
  （Settings → Social preview → Edit → Upload），无需提交到仓库

## 后续

设计完成后：
1. 将 `social-preview.png` 放到 `.github/`
2. 在 GitHub 仓库 Settings → Social preview 上传同一张图
3. 更新本文档状态勾选项
