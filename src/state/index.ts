/**
 * 状态层 barrel：仅做聚合 re-export，不放任何状态。
 *
 * 实际实现按职责拆分到：
 * - `./config`：用户配置态（config + isConfigured + toArgs）
 * - `./prefs`：应用偏好态（prefs）
 * - `./runtime`：运行态（frpcStatus / frpcError / running / logs）
 *
 * 新代码请按需 `from "./state"` 或 `from "./state/config"` 等具体子模块导入。
 */
export * from "./config";
export * from "./prefs";
export * from "./runtime";
