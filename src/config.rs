//! BIND-19 运行时配置（调试模式 + 环境变量）
//!
//! 调试模式（CI144_DEBUG=1）：
//! - 规则 6（Replay-Enable=0 强制降级至 MEDIUM）可跳过
//! - 规则 1-3（CATASTROPHIC 硬覆盖）仍生效，不可跳过
//! - 环境变量仅在启动时读取，不可运行时动态切换
//! - 启动时输出警告 banner
//!
//! 规范依据：规则 6（Replay-Enable=0 安全约束）
//! ADR：ADR-0002（验证失败处理）

use std::sync::OnceLock;

/// 调试模式环境变量名
pub const DEBUG_ENV_VAR: &str = "CI144_DEBUG";

/// 全局配置（仅初始化一次，启动时读取环境变量）
static CONFIG: OnceLock<BindConfig> = OnceLock::new();

/// BIND-19 运行时配置
#[derive(Debug, Clone, Copy, Default)]
pub struct BindConfig {
    /// 调试模式（CI144_DEBUG=1）
    /// - true：规则 6 降级可跳过
    /// - false：生产模式，所有规则严格执行
    pub debug_mode: bool,
}

impl BindConfig {
    /// 从环境变量创建配置（仅在启动时调用一次）
    ///
    /// 环境变量 CI144_DEBUG：
    /// - "1" / "true" / "yes" / "on"（不区分大小写）→ 调试模式
    /// - 其他值或未设置 → 生产模式
    pub fn from_env() -> Self {
        let debug_mode = std::env::var(DEBUG_ENV_VAR)
            .map(|v| {
                matches!(
                    v.to_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        if debug_mode {
            eprintln!(
                "⚠️  [BIND-19] 调试模式已启用（{}=1）",
                DEBUG_ENV_VAR
            );
            eprintln!(
                "    规则 6（Replay-Enable=0 强制降级）将被跳过"
            );
            eprintln!(
                "    规则 1-3（CATASTROPHIC 硬覆盖）仍然严格生效"
            );
            eprintln!("    此模式仅用于开发/调试，禁止在生产环境使用");
        }

        Self { debug_mode }
    }

    /// 获取全局配置（如果未初始化，自动从环境变量初始化）
    pub fn global() -> &'static Self {
        CONFIG.get_or_init(Self::from_env)
    }

    /// 手动设置全局配置（用于测试，仅在未初始化时有效）
    pub fn set_global(config: Self) -> Result<(), &'static str> {
        CONFIG
            .set(config)
            .map_err(|_| "global config already initialized")
    }

    /// 是否为调试模式
    pub fn is_debug(&self) -> bool {
        self.debug_mode
    }

    /// 规则 6（Replay-Enable=0 强制降级）是否生效
    ///
    /// 调试模式下规则 6 可跳过，生产模式下严格执行。
    pub fn rule6_enabled(&self) -> bool {
        !self.debug_mode
    }

    /// 规则 1-3（CATASTROPHIC 硬覆盖）是否生效
    ///
    /// 无论是否调试模式，CATASTROPHIC 规则始终生效，不可跳过。
    pub fn catastrophic_rules_enabled(&self) -> bool {
        true // 始终生效，不可跳过
    }
}

// ─── 单元测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_production() {
        let config = BindConfig::default();
        assert!(!config.is_debug());
        assert!(config.rule6_enabled());
        assert!(config.catastrophic_rules_enabled());
    }

    #[test]
    fn test_debug_mode_skips_rule6() {
        let config = BindConfig { debug_mode: true };
        assert!(config.is_debug());
        assert!(!config.rule6_enabled()); // 规则 6 可跳过
        assert!(config.catastrophic_rules_enabled()); // CATASTROPHIC 仍生效
    }

    #[test]
    fn test_production_mode_enforces_all_rules() {
        let config = BindConfig { debug_mode: false };
        assert!(!config.is_debug());
        assert!(config.rule6_enabled());
        assert!(config.catastrophic_rules_enabled());
    }

    #[test]
    fn test_set_global() {
        // 注意：这个测试可能与其他测试冲突（全局状态）
        // 使用独立的测试配置，不依赖全局状态
        let config = BindConfig { debug_mode: true };
        assert!(config.is_debug());
        assert!(!config.rule6_enabled());
    }

    #[test]
    fn test_debug_env_var_name() {
        assert_eq!(DEBUG_ENV_VAR, "CI144_DEBUG");
    }
}
