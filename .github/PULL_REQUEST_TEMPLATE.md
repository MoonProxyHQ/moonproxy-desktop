## 描述

简要说明本次改动解决了什么 / 改变了什么。

## 关联 Issue

Fixes #(issue) / Relates to #(issue)

## 改动类型

- [ ] Bug 修复
- [ ] 新功能
- [ ] 重构 / 性能
- [ ] 文档 / SEO / GEO（README、llm.txt、官网等）
- [ ] 构建 / CI / 工具链
- [ ] 其他

## 改动清单

- [ ] 列出本次的主要改动点
- [ ] 如果涉及版本号：`package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` / `src/composables/useAppUpdate.ts::APP_VERSION` 四处是否同步（参考 AGENTS.md「版本号同步规范」）

## 测试

- [ ] 已本地 `pnpm tauri dev` 联调
- [ ] 已 `cargo check` / `cargo test`
- [ ] 已 `pnpm build` 走通
- [ ] 如改 UI：附截图 / 录屏

## 影响面

- [ ] 兼容旧配置
- [ ] 不影响 frpc 二进制（如有变化需说明）
- [ ] 不需要新发布（只改文档 / CI）
