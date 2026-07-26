//! TOML-based LLM configuration loader
//!
//! Loads LLM configuration from `config/llm.toml` with environment variable interpolation.
//! Priority: CLI args > TOML file (with ${VAR} interpolation) > defaults.

use anyhow::{Context, Result};
use core_domain::{FredFileConfig, LlmFileConfig};
use market_store::StorageConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================
// Config Source Tracking
// ============================================================

/// 配置来源追踪（用于 show-llm-config 命令）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSource {
    pub base_url: String,  // "file" | "cli" | "default"
    pub model: String,
    pub api_key: String,   // "env:VAR_NAME" | "file" | "cli" | "none"
    pub config_file: Option<String>,
}

// ============================================================
// Resolved Config
// ============================================================

/// 最终生效的 LLM 配置（合并所有来源后）
#[derive(Debug, Clone)]
pub struct ResolvedLlmConfig {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub api_key: Option<String>,
    pub temperature: f64,
    pub max_tokens: usize,
    pub seed: Option<u64>,
    pub source: ConfigSource,
    /// ADR-112: 共享博弈假设背景是否默认注入（默认 true）
    pub adversarial_auto_inject: bool,
    /// ADR-112: 按 persona 的注入级别映射；未列出的 persona 默认 Full
    pub adversarial_inject: std::collections::HashMap<String, core_domain::InjectLevel>,
    /// ADR-114 ContentPolicy: standard 级别注入最大字符数（默认 4000）
    pub adversarial_max_chars: usize,
    /// ADR-114 ContentPolicy: full 级别注入硬性上限（默认 12000）
    pub adversarial_full_max_chars: usize,
    /// ADR-114 ContentPolicy: 截断策略（默认 paragraph_boundary）
    pub adversarial_truncate_strategy: core_domain::TruncateStrategy,
}

/// ADR-112 内置默认分级映射（文件未配置时使用）
fn default_adversarial_inject_map() -> std::collections::HashMap<String, core_domain::InjectLevel> {
    use core_domain::InjectLevel::*;
    [
        ("market_story", Standard),
        ("portfolio_review", Standard),
        ("risk_view", Standard),
        ("devils_advocate", Standard),
        ("short_term_trader", Standard),
        ("long_term_allocator", Standard),
        ("explain_decision", Compact),
        ("preclose_review", Compact),
        ("market_adversarial_lens", None),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// CLI 参数覆盖
#[derive(Debug, Clone)]
pub struct CliLlmArgs {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

// ============================================================
// File Loading
// ============================================================

/// 从 TOML 文件加载 LLM 配置
pub fn load_llm_config_from_file(path: &Path) -> Result<LlmFileConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let mut config: LlmFileConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    // 处理环境变量插值（所有字符串字段）
    resolve_env_vars(&mut config)?;

    Ok(config)
}

/// 获取配置文件路径
pub fn get_config_path(profile: Option<&str>) -> Option<PathBuf> {
    let root = StorageConfig::project_root().ok()?;
    let config_dir = root.join("config");

    // 尝试 profile 特定文件
    if let Some(p) = profile {
        let profile_path = config_dir.join(format!("llm.{}.toml", p));
        if profile_path.exists() {
            return Some(profile_path);
        }
    }

    // 回退到默认文件
    let default_path = config_dir.join("llm.toml");
    if default_path.exists() {
        return Some(default_path);
    }

    None
}

// ============================================================
// Environment Variable Interpolation
// ============================================================

/// 解析环境变量插值（支持 ${VAR} 和 $${LITERAL} 转义）
fn resolve_env_vars(config: &mut LlmFileConfig) -> Result<()> {
    config.llm.base_url = interpolate(&config.llm.base_url)?;
    config.llm.model = interpolate(&config.llm.model)?;
    if let Some(ref key) = config.llm.auth.api_key {
        config.llm.auth.api_key = Some(interpolate(key)?);
    }
    Ok(())
}

/// 单个字符串的环境变量插值
///
/// **限制**：仅支持完整值替换，不支持嵌入式插值。
/// - ✅ `${VAR_NAME}` → 从环境变量读取
/// - ✅ `$${LITERAL}` → 转义为 `${LITERAL}`（字面值）
/// - ❌ `https://${HOST}/v1` → 不会插值，原样返回
///
/// 如需嵌入式插值，请使用完整环境变量引用：
/// ```toml
/// # 错误（不会插值）
/// base_url = "https://${HOST}/v1"
///
/// # 正确（完整值引用）
/// base_url = "${OPENAI_BASE_URL}"  # 环境变量中存储完整 URL
/// ```
fn interpolate(value: &str) -> Result<String> {
    // 转义: $${LITERAL} → ${LITERAL}
    if value.starts_with("$$") {
        return Ok(value[1..].to_string());
    }

    // 插值: ${VAR_NAME} → env::var(VAR_NAME)
    if value.starts_with("${") && value.ends_with('}') {
        let var_name = &value[2..value.len() - 1];
        std::env::var(var_name).with_context(|| {
            format!(
                "Environment variable '{}' not set (referenced in config)",
                var_name
            )
        })
    } else {
        Ok(value.to_string())
    }
}

// ============================================================
// Priority Resolution
// ============================================================

impl ResolvedLlmConfig {
    /// 按优先级解析配置：CLI > File(含 ${VAR} 插值) > Default
    pub fn resolve(cli_args: Option<CliLlmArgs>) -> Result<Self> {
        // 1. 加载文件配置（已含 ${VAR} 插值）
        let profile = std::env::var("LLM_CONFIG_PROFILE").ok();
        let config_path = get_config_path(profile.as_deref());

        let file_config = match config_path.as_ref().and_then(|p| {
            load_llm_config_from_file(p).ok()
        }) {
            Some(c) => c,
            None => LlmFileConfig::default(),
        };

        // 2. 从文件配置提取值
        let mut base_url = file_config.llm.base_url.clone();
        let mut model = file_config.llm.model.clone();
        let mut api_key = file_config.llm.auth.api_key.clone();
        let mut source = ConfigSource {
            base_url: if config_path.is_some() {
                "file".to_string()
            } else {
                "default".to_string()
            },
            model: if config_path.is_some() {
                "file".to_string()
            } else {
                "default".to_string()
            },
            api_key: if api_key.is_some() {
                "file".to_string()
            } else {
                "none".to_string()
            },
            config_file: config_path.map(|p| p.display().to_string()),
        };

        // 3. CLI 参数覆盖（最高优先级）
        if let Some(args) = cli_args {
            if let Some(url) = args.base_url {
                base_url = url;
                source.base_url = "cli".to_string();
            }
            if let Some(m) = args.model {
                model = m;
                source.model = "cli".to_string();
            }
            if let Some(key) = args.api_key {
                api_key = Some(key);
                source.api_key = "cli".to_string();
            }
        }

        // 4. Windows 安全警告
        warn_if_plaintext_key_windows(&api_key);

        // 5. ADR-112: 共享博弈背景配置（文件缺失时用内置默认映射）
        //    ADR-114: ContentPolicy（max_chars / full_max_chars / truncate_strategy）
        //    与 InjectionLevel 解耦——级别是内容粒度，policy 是体积保护。
        let (
            adversarial_auto_inject,
            adversarial_inject,
            adversarial_max_chars,
            adversarial_full_max_chars,
            adversarial_truncate_strategy,
        ) = match file_config.llm.adversarial {
            Some(ref section) => {
                let mut map = default_adversarial_inject_map();
                // 文件中的显式配置覆盖内置默认
                for (persona, level) in &section.inject {
                    map.insert(persona.clone(), *level);
                }
                (
                    section.auto_inject,
                    map,
                    section.max_chars,
                    section.full_max_chars,
                    section.truncate_strategy,
                )
            }
            None => (
                true,
                default_adversarial_inject_map(),
                core_domain::default_adversarial_max_chars(),
                core_domain::default_adversarial_full_max_chars(),
                core_domain::TruncateStrategy::default(),
            ),
        };

        Ok(Self {
            base_url,
            model,
            timeout_secs: file_config.llm.timeout_secs,
            api_key,
            temperature: file_config.llm.defaults.temperature,
            max_tokens: file_config.llm.defaults.max_tokens,
            seed: file_config.llm.defaults.seed,
            source,
            adversarial_auto_inject,
            adversarial_inject,
            adversarial_max_chars,
            adversarial_full_max_chars,
            adversarial_truncate_strategy,
        })
    }
}

// ============================================================
// Windows Security Warning
// ============================================================

/// Windows 上检测到明文 API Key 时发出警告
fn warn_if_plaintext_key_windows(api_key: &Option<String>) {
    #[cfg(windows)]
    {
        if let Some(ref key) = api_key {
            if !key.starts_with("${") && !key.is_empty() {
                eprintln!();
                eprintln!("┌─────────────────────────────────────────────────────────────────┐");
                eprintln!("│ WARN: API key stored in plaintext in config/llm.toml           │");
                eprintln!("│ This is insecure on Windows (no file permission protection)     │");
                eprintln!("│                                                                │");
                eprintln!("│ Recommended: Use environment variable reference instead:        │");
                eprintln!("│   api_key = \"${{OPENAI_API_KEY}}\"                              │");
                eprintln!("│                                                                │");
                eprintln!("│ Then set the environment variable:                             │");
                eprintln!("│   set OPENAI_API_KEY=sk-xxxx                                   │");
                eprintln!("└─────────────────────────────────────────────────────────────────┘");
                eprintln!();
            }
        }
    }
}

// ============================================================
// Config File Writing (for CLI commands)
// ============================================================

/// 读取现有配置文件（不存在则返回默认）
pub fn read_or_default_config(path: &Path) -> LlmFileConfig {
    load_llm_config_from_file(path).unwrap_or_default()
}

/// 写入配置到 TOML 文件
pub fn write_llm_config_to_file(path: &Path, config: &LlmFileConfig) -> Result<()> {
    let content =
        toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;

    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    std::fs::write(path, &content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    // Unix: 设置文件权限为 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to set file permissions: {}", path.display()))?;
    }

    Ok(())
}

/// 获取默认配置文件路径（用于写入）
pub fn default_config_path() -> Result<PathBuf> {
    let root = StorageConfig::project_root()?;
    Ok(root.join("config").join("llm.toml"))
}

// ============================================================
// Validation
// ============================================================

/// 配置验证结果
#[derive(Debug, Serialize)]
pub struct ConfigValidation {
    pub file_exists: bool,
    pub file_parseable: bool,
    pub env_vars_resolved: bool,
    pub missing_env_vars: Vec<String>,
    pub url_format_valid: bool,
    pub api_key_set: bool,
}

/// 验证配置文件
pub fn validate_config() -> ConfigValidation {
    let mut result = ConfigValidation {
        file_exists: false,
        file_parseable: false,
        env_vars_resolved: true,
        missing_env_vars: Vec::new(),
        url_format_valid: false,
        api_key_set: false,
    };

    let path = match default_config_path() {
        Ok(p) => p,
        Err(_) => return result,
    };

    // 检查文件是否存在
    result.file_exists = path.exists();
    if !result.file_exists {
        return result;
    }

    // 尝试解析文件
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    let config: LlmFileConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return result,
    };

    result.file_parseable = true;

    // 检查 URL 格式
    result.url_format_valid = config.llm.base_url.starts_with("http://")
        || config.llm.base_url.starts_with("https://");

    // 检查环境变量引用
    if config.llm.base_url.starts_with("${") && config.llm.base_url.ends_with('}') {
        let var_name = &config.llm.base_url[2..config.llm.base_url.len() - 1];
        if std::env::var(var_name).is_err() {
            result.env_vars_resolved = false;
            result.missing_env_vars.push(var_name.to_string());
        }
    }

    if config.llm.model.starts_with("${") && config.llm.model.ends_with('}') {
        let var_name = &config.llm.model[2..config.llm.model.len() - 1];
        if std::env::var(var_name).is_err() {
            result.env_vars_resolved = false;
            result.missing_env_vars.push(var_name.to_string());
        }
    }

    if let Some(ref key) = config.llm.auth.api_key {
        if key.starts_with("${") && key.ends_with('}') {
            let var_name = &key[2..key.len() - 1];
            if std::env::var(var_name).is_err() {
                result.env_vars_resolved = false;
                result.missing_env_vars.push(var_name.to_string());
            } else {
                // 环境变量引用解析成功，视为 key 已设置
                result.api_key_set = true;
            }
        } else if !key.is_empty() {
            result.api_key_set = true;
        }
    }

    result
}

// ============================================================
// FRED Configuration (ADR-064)
// ============================================================

/// 从 TOML 文件加载 FRED 配置
pub fn load_fred_config_from_file(path: &Path) -> Result<FredFileConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read FRED config file: {}", path.display()))?;
    let mut config: FredFileConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse FRED config file: {}", path.display()))?;

    // 处理环境变量插值
    resolve_fred_env_vars(&mut config)?;

    Ok(config)
}

/// 解析 FRED 配置中的环境变量插值
fn resolve_fred_env_vars(config: &mut FredFileConfig) -> Result<()> {
    config.fred.base_url = interpolate(&config.fred.base_url)?;
    if let Some(ref key) = config.fred.auth.api_key {
        config.fred.auth.api_key = Some(interpolate(key)?);
    }
    Ok(())
}

/// 获取 FRED 配置文件路径
pub fn get_fred_config_path() -> Option<PathBuf> {
    let root = StorageConfig::project_root().ok()?;
    let config_dir = root.join("config");
    let path = config_dir.join("fred.toml");
    if path.exists() {
        return Some(path);
    }
    None
}

/// 最终生效的 FRED 配置（合并所有来源后）
#[derive(Debug, Clone)]
pub struct ResolvedFredConfig {
    pub enabled: bool,
    pub base_url: String,
    pub api_key: Option<String>,
    pub request_delay_ms: u64,
    pub timeout_secs: u64,
    pub source: String, // "file" | "default"
    pub config_file: Option<String>,
}

impl ResolvedFredConfig {
    /// 解析 FRED 配置：File(含 ${VAR} 插值) > Default
    pub fn resolve() -> Result<Self> {
        let config_path = get_fred_config_path();

        let file_config = match config_path.as_ref().and_then(|p| {
            load_fred_config_from_file(p).ok()
        }) {
            Some(c) => c,
            None => FredFileConfig::default(),
        };

        let source = if config_path.is_some() {
            "file".to_string()
        } else {
            "default".to_string()
        };

        let api_key = file_config.fred.auth.api_key;

        Ok(Self {
            enabled: file_config.fred.enabled,
            base_url: file_config.fred.base_url,
            api_key,
            request_delay_ms: file_config.fred.request_delay_ms,
            timeout_secs: file_config.fred.timeout_secs,
            source,
            config_file: config_path.map(|p| p.display().to_string()),
        })
    }

    /// 检查配置是否完整（enabled=true 时必须有 api_key）
    pub fn is_valid(&self) -> bool {
        if !self.enabled {
            return true;
        }
        self.api_key.is_some() && !self.api_key.as_ref().unwrap().is_empty()
    }
}

/// 读取现有 FRED 配置文件（不存在则返回默认）
pub fn read_or_default_fred_config(path: &Path) -> FredFileConfig {
    load_fred_config_from_file(path).unwrap_or_default()
}

/// 写入 FRED 配置到 TOML 文件
pub fn write_fred_config_to_file(path: &Path, config: &FredFileConfig) -> Result<()> {
    let content =
        toml::to_string_pretty(config).context("Failed to serialize FRED config to TOML")?;

    // 确保目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    std::fs::write(path, &content)
        .with_context(|| format!("Failed to write FRED config file: {}", path.display()))?;

    // Unix: 设置文件权限为 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to set file permissions: {}", path.display()))?;
    }

    Ok(())
}

/// 获取默认 FRED 配置文件路径（用于写入）
pub fn default_fred_config_path() -> Result<PathBuf> {
    let root = StorageConfig::project_root()?;
    Ok(root.join("config").join("fred.toml"))
}

/// FRED 配置验证结果
#[derive(Debug, Serialize)]
pub struct FredConfigValidation {
    pub file_exists: bool,
    pub file_parseable: bool,
    pub env_vars_resolved: bool,
    pub missing_env_vars: Vec<String>,
    pub enabled: bool,
    pub api_key_set: bool,
}

/// 验证 FRED 配置文件
pub fn validate_fred_config() -> FredConfigValidation {
    let mut result = FredConfigValidation {
        file_exists: false,
        file_parseable: false,
        env_vars_resolved: true,
        missing_env_vars: Vec::new(),
        enabled: FredFileConfig::default().fred.enabled,
        api_key_set: false,
    };

    let path = match default_fred_config_path() {
        Ok(p) => p,
        Err(_) => return result,
    };

    // 检查文件是否存在
    result.file_exists = path.exists();
    if !result.file_exists {
        return result;
    }

    // 尝试解析文件
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    let config: FredFileConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return result,
    };

    result.file_parseable = true;
    result.enabled = config.fred.enabled;

    // 检查环境变量引用
    if let Some(ref key) = config.fred.auth.api_key {
        if key.starts_with("${") && key.ends_with('}') {
            let var_name = &key[2..key.len() - 1];
            if std::env::var(var_name).is_err() {
                result.env_vars_resolved = false;
                result.missing_env_vars.push(var_name.to_string());
            } else {
                result.api_key_set = true;
            }
        } else if !key.is_empty() {
            result.api_key_set = true;
        }
    }

    result
}
