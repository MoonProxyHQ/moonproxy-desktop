# Moonbox Desktop

> A cross-platform **FRP desktop client** for non-technical users. Built with [Tauri v2](https://tauri.app), runs on macOS and Windows — turns [frp](https://github.com/fatedier/frp) reverse-proxy / NAT-traversal into a one-click experience.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![CI](https://github.com/lingjistudio/moonbox-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/lingjistudio/moonbox-desktop/actions/workflows/ci.yml)
[![Release](https://github.com/lingjistudio/moonbox-desktop/actions/workflows/release.yml/badge.svg)](https://github.com/lingjistudio/moonbox-desktop/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/lingjistudio/moonbox-desktop?include_prereleases)](https://github.com/lingjistudio/moonbox-desktop/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-blue)](#install)
[![frpc](https://img.shields.io/badge/frpc-v0.69.1-orange)](https://github.com/fatedier/frp)
[![Tauri](https://img.shields.io/badge/Tauri-v2-blue)](https://tauri.app)

**[简体中文](./README.md)** · English

---

<video src="./screenshots/main-mv.mp4" controls muted preload="metadata" width="100%"></video>

![Moonbox Desktop main UI: 4-state circular status button + public access URL + endpoint health indicators](./screenshots/main-pic.webp)

A friendly **desktop GUI for [frp](https://github.com/fatedier/frp)** — the reverse-proxy / NAT-traversal tool.
You bring your own frps server (self-hosted or any community frps you trust) — Moonbox Desktop
takes care of the rest: configuration, lifecycle, connection health, auto-update,
and a polished tray-resident experience.

No CLI, no hand-edited `frpc.toml`, no manual process management — purpose-built for
**individual developers, self-hosters, and remote workers** who want frp without the terminal.

## Highlights

- **Visual proxy management** — TCP / UDP / HTTP / HTTPS rules in a single tab.
- **One-click start/stop** — a circular button reflects 4 states
  (stopped / connecting / connected / error); the connected state is derived
  from real frpc evidence, not optimistic flags.
- **Live endpoint health** — local port reachability is polled every 3s so you
  catch broken tunnels before they bite you.
- **System tray resident** — closing the window hides to tray while frpc keeps
  running in the background.
- **Launch at login + silent start** — auto-launch can hide straight to tray.
- **Scheduled connect** — pick weekdays and start/stop times; hot-reloaded
  each minute by the scheduler.
- **Engine self-update** — frpc is fetched from upstream GitHub releases,
  SHA256-verified, then atomically swapped without reinstalling the app.
- **App self-update** — built on `tauri-plugin-updater`.
- **Bundled frpc sidecar** — users never need to install frp separately.

## Use Cases

- **Remote work** — SSH / RDP into office machines behind NAT, without the VPN hassle.
- **Self-hosted services** — temporarily expose NAS, Home Assistant, home media, or a personal blog.
- **Dev & webhook debugging** — expose local ports for Webhook / OAuth / third-party callbacks.
- **Team tooling** — share a local service with colleagues without a public IP or cloud VM.
- **Game servers** — open Minecraft (and similar) to friends for a session.

## Relationship to frp

[Moonbox Desktop](https://github.com/lingjistudio/moonbox-desktop) is an **unofficial** desktop GUI client
for [fatedier/frp](https://github.com/fatedier/frp) and is independent from the frp project.

- **frp** is the open-source reverse-proxy / NAT-traversal project maintained by fatedier.
- **Moonbox Desktop** does not modify frpc behavior — it handles **configuration generation,
  subprocess lifecycle, and connection-state visualization** only.
- The frpc binary (v0.69.1) is bundled via Tauri's sidecar mechanism; users never install frp separately.
- The frpc engine can auto-update from upstream frp GitHub releases, with atomic swap.

> In short: **frp provides the capability, Moonbox Desktop provides the usability.**

## Install

Pre-built binaries are published on the
[GitHub Releases page](https://github.com/lingjistudio/moonbox-desktop/releases).

| Platform | Download |
| --- | --- |
| macOS (Apple Silicon) | `Moonbox-Desktop_<version>_aarch64.dmg` |
| macOS (Intel) | `Moonbox-Desktop_<version>_x64.dmg` |
| Windows (x64) | `Moonbox-Desktop_<version>_x64-setup.exe` |

> **macOS first-launch note:** the app is ad-hoc signed but **not** notarized
> (no Apple Developer certificate). On first launch, right-click the app →
> **Open** → confirm in the dialog. Alternatively, after dragging to
> `/Applications`, run `xattr -cr "/Applications/Moonbox Desktop.app"` to drop
> the quarantine attribute.

## Build from source

```bash
pnpm install
pnpm sync:frpc        # download frpc sidecar binaries
pnpm tauri dev        # local dev
pnpm tauri build      # production build for current platform
```

> Requires Node.js, pnpm, Rust toolchain, and platform-specific build tools.
> See [CONTRIBUTING.md](./CONTRIBUTING.md) for details.

## License

[MIT](./LICENSE).

---

> This project is independent from [fatedier/frp](https://github.com/fatedier/frp).
> frp's releases and licensing remain with the upstream project;
> Moonbox Desktop is a desktop client only.
