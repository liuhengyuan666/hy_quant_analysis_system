# LLM 配置方案迁移设计：SQLite+Keyring → TOML+Env

> **Oracle 复核结果**: 条件批准，需修复 3 个关键问题后方可实施。

---

## 1. 背景与动机

### 当前痛点
- **透明性差**：配置"藏"在 SQLite 和 OS Keyring 里，用户无法一眼看到完整配置
- **可编辑性差**：必须通过 CLI 命令查看/修改，不能直接编辑文件
- **可移植性差**：换机器需要重新设置，无法备份/恢复配置
- **多环境不支持**：无法轻松切换 dev/staging/prod 配置

### 设计目标
- ✅ 配置透明：打开文件一目了然
- ✅ 安全不妥协：API Key 不落盘明文
- ✅ 向后兼容：保留 CLI 命令作为快捷入口
- ✅ 符合 Rust 生态惯例

---

## 2. Oracle 复核结果与关键修复

### 关键问题（必须修复）

| # | 问题 | 修复方案 |
|---|------|----------|
| 1 | **架构违规**：将 I/O 逻辑放入 `core-domain` 违反 crate 契约 | `core-domain` 只定义配置结构体，加载逻辑移至 `app-service` |
| 2 | **忽略 V4 Skill 系统**：计划只改 deprecated 的 `analyze-with-llm`，未说明 V4 如何获取配置 | 明确 V4 `OpenAiProvider` 从同一 TOML 读取，`LlmCallConfig` 使用 `[llm.defaults]` |
| 3 | **Windows 无保护**：`chmod 600` 仅 Unix 有效，Windows 上明文 key 可被任意读取 | Windows 上检测到非 `${VAR}` 的 api_key 时发出警告，推荐仅用环境变量 |

### 建议改进（采纳）

- **`${VAR}` 插值扩展到所有字符串字段**（base_url, model, api_key）
- **简化环境变量路径**：文件内 `${VAR}` 插值后，不再单独处理 `OPENAI_*` 环境变量层
- **`migrate-llm-config` 碰撞行为**：已存在 `llm.toml` 时拒绝覆盖，需 `--force` 标志
- **添加 `show-llm-config --validate`**：检查文件存在、`${VAR}` 引用可解析、URL 格式正确

### 边界情况处理

| 场景 | 处理方式 |
|------|----------|
| 字面值以 `${` 开头 | 使用 `$${` 转义（`$${LITERAL}` → `${LITERAL}`） |
| 所有来源都无 api_key | V3 报错，V4 使用 PlaceholderProvider（当前行为） |
| `LLM_CONFIG_PROFILE=dev` 但 `llm.dev.toml` 不存在 | 回退到 `llm.toml`，再回退到默认值 |

---

## 3. 配置文件结构

### 2.1 示例文件（提交到 Git）

```toml
# config/llm.toml.example
# LLM 配置示例文件
# 复制为 config/llm.toml 并填入实际值

[llm]
# LLM API 基础 URL
# 支持 OpenAI、DeepSeek、本地模型等 OpenAI-compatible API
base_url = "https://api.openai.com/v1"

# 模型名称
model = "gpt-4o-mini"

# 请求超时时间（秒）
timeout_secs = 60

[llm.auth]
# API Key 配置（三选一，按优先级）：
#
# 方式1: 环境变量引用（推荐，最安全）
# api_key = "${OPENAI_API_KEY}"
#
# 方式2: 直接写入（不推荐，仅测试用）
# api_key = "sk-xxxxxxxxxxxxxxxx"
#
# 方式3: 不设置此项，通过 CLI 命令设置到 Keyring
# api_key = ""

[llm.defaults]
# 默认 temperature（0.0-2.0）
temperature = 0.7

# 默认 max_tokens
max_tokens = 4096

# 可选：固定 seed 用于可复现分析
# seed = 42
```

### 2.2 实际配置文件（gitignore）

```toml
# config/llm.toml
[llm]
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"
timeout_secs = 120

[llm.auth]
api_key = "${DEEPSEEK_API_KEY}"

[llm.defaults]
temperature = 0.7
max_tokens = 4096
```

### 2.3 多环境配置（可选）

```
config/
├── llm.toml           # 默认配置
├── llm.dev.toml       # 开发环境
├── llm.prod.toml      # 生产环境
└── llm.toml.example   # 示例文件（提交到 Git）
```

通过环境变量 `LLM_CONFIG_PROFILE=dev` 选择配置文件。

---

## 4. 加载优先级

```
优先级从高到低：
1. CLI 参数（--base-url, --model, --api-key）
2. 环境变量（OPENAI_API_KEY, OPENAI_BASE_URL, OPENAI_MODEL）
3. config/llm.toml 文件
4. 内置默认值
```

### 3.1 环境变量映射

| 环境变量 | 配置项 | 说明 |
|----------|--------|------|
| `OPENAI_API_KEY` | `llm.auth.api_key` | API Key |
| `OPENAI_BASE_URL` | `llm.base_url` | API 基础 URL |
| `OPENAI_MODEL` | `llm.model` | 模型名称 |
| `LLM_TIMEOUT_SECS` | `llm.timeout_secs` | 超时时间 |
| `LLM_CONFIG_PROFILE` | - | 配置文件 profile（dev/prod） |

### 3.2 TOML 内环境变量插值

```toml
[llm.auth]
api_key = "${OPENAI_API_KEY}"  # 运行时替换为环境变量值
```

插值逻辑：
1. 读取 TOML 值
2. 检查是否匹配 `${VAR_NAME}` 格式
3. 如果匹配，从 `std::env::var("VAR_NAME")` 读取
4. 如果环境变量不存在，返回错误

---

## 5. 实现架构

### 5.1 配置结构体定义（core-domain）

> **Oracle 修复 #1**: `core-domain` 只定义数据结构，不包含 I/O 逻辑。

```rust
// crates/core-domain/src/lib.rs

/// LLM 文件配置结构体（从 TOML 加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFileConfig {
    pub llm: LlmSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSection {
    pub base_url: String,
    pub model: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub defaults: DefaultsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthSection {
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSection {
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    pub seed: Option<u64>,
}

fn default_timeout() -> u64 { 60 }
fn default_temperature() -> f64 { 0.7 }
fn default_max_tokens() -> usize { 4096 }

impl Default for DefaultsSection {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            seed: None,
        }
    }
}

impl Default for LlmFileConfig {
    fn default() -> Self {
        Self {
            llm: LlmSection {
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                timeout_secs: 60,
                auth: AuthSection::default(),
                defaults: DefaultsSection::default(),
            },
        }
    }
}
```

### 5.2 配置加载逻辑（app-service）

> **Oracle 修复 #1**: I/O 操作放在 `app-service`，与 `AppContext` 对齐。

```rust
// crates/app-service/src/config_loader.rs

use anyhow::{Context, Result};
use core_domain::LlmFileConfig;
use std::path::Path;

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

/// 从默认路径加载（支持 profile）
pub fn load_llm_config() -> Result<LlmFileConfig> {
    let profile = std::env::var("LLM_CONFIG_PROFILE").ok();
    let root = StorageConfig::project_root()?;
    let config_dir = root.join("config");
    
    // 尝试 profile 特定文件
    if let Some(ref p) = profile {
        let profile_path = config_dir.join(format!("llm.{}.toml", p));
        if profile_path.exists() {
            return load_llm_config_from_file(&profile_path);
        }
    }
    
    // 回退到默认文件
    let default_path = config_dir.join("llm.toml");
    if default_path.exists() {
        return load_llm_config_from_file(&default_path);
    }
    
    // 无文件时返回默认配置
    Ok(LlmFileConfig::default())
}

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
fn interpolate(value: &str) -> Result<String> {
    // 转义: $${LITERAL} → ${LITERAL}
    if value.starts_with("$$") {
        return Ok(value[1..].to_string());
    }
    
    // 插值: ${VAR_NAME} → env::var(VAR_NAME)
    if value.starts_with("${") && value.ends_with('}') {
        let var_name = &value[2..value.len()-1];
        std::env::var(var_name)
            .with_context(|| format!("Environment variable '{}' not set (referenced in config)", var_name))
    } else {
        Ok(value.to_string())
    }
}
```

### 5.3 统一配置解析（优先级合并）

```rust
// crates/app-service/src/config_loader.rs

/// 最终生效的 LLM 配置
#[derive(Debug, Clone)]
pub struct ResolvedLlmConfig {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub api_key: Option<String>,
    pub temperature: f64,
    pub max_tokens: usize,
    pub seed: Option<u64>,
    pub config_source: ConfigSource,  // 用于 show-llm-config
}

/// 配置来源追踪
#[derive(Debug, Clone)]
pub struct ConfigSource {
    pub base_url: String,      // "file" | "cli" | "default"
    pub model: String,
    pub api_key: String,       // "env:VAR_NAME" | "file" | "cli" | "none"
    pub config_file: Option<PathBuf>,
}

impl ResolvedLlmConfig {
    /// 按优先级解析配置：CLI > File(含 ${VAR} 插值) > Default
    /// 
    /// Oracle 修复：简化环境变量路径，不再单独处理 OPENAI_* 层
    pub fn resolve(cli_args: Option<CliLlmArgs>) -> Result<Self> {
        // 1. 加载文件配置（已含 ${VAR} 插值）
        let (file_config, config_file) = match load_llm_config() {
            Ok(c) => {
                let profile = std::env::var("LLM_CONFIG_PROFILE").ok();
                let path = get_config_path(profile.as_deref());
                (c, path)
            }
            Err(e) => {
                eprintln!("WARN: Failed to load LLM config: {}. Using defaults.", e);
                (LlmFileConfig::default(), None)
            }
        };
        
        // 2. 从文件配置提取值
        let mut base_url = file_config.llm.base_url.clone();
        let mut model = file_config.llm.model.clone();
        let mut api_key = file_config.llm.auth.api_key.clone();
        let mut source = ConfigSource {
            base_url: "file".to_string(),
            model: "file".to_string(),
            api_key: if api_key.is_some() { "file".to_string() } else { "none".to_string() },
            config_file,
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
        #[cfg(windows)]
        if let Some(ref key) = api_key {
            if !key.starts_with("${") {
                eprintln!("WARN: API key stored in plaintext on Windows. Consider using environment variables: api_key = \"${{OPENAI_API_KEY}}\"");
            }
        }
        
        Ok(Self {
            base_url,
            model,
            timeout_secs: file_config.llm.timeout_secs,
            api_key,
            temperature: file_config.llm.defaults.temperature,
            max_tokens: file_config.llm.defaults.max_tokens,
            seed: file_config.llm.defaults.seed,
            source,
        })
    }
}
```

---

## 6. 向后兼容策略

### 5.1 保留 CLI 命令

```bash
# 仍然支持，但行为改为写入 config/llm.toml
cargo run -p quant-cli -- set-llm-config --base-url ... --model ...
cargo run -p quant-cli -- set-llm-api-key --key sk-xxx
```

实现：CLI 命令改为读取 → 修改 → 写回 TOML 文件。

### 5.2 迁移路径

提供迁移命令：
```bash
cargo run -p quant-cli -- migrate-llm-config
```

功能：
1. 从 SQLite 读取现有 `llm_config`
2. 从 Keyring/SQLite 读取 `llm_api_key`
3. 写入 `config/llm.toml`
4. 打印迁移结果

### 5.3 配置来源指示

```bash
cargo run -p quant-cli -- show-llm-config
```

输出示例：
```json
{
  "base_url": "https://api.deepseek.com/v1",
  "model": "deepseek-chat",
  "timeout_secs": 120,
  "api_key_source": "env:DEEPSEEK_API_KEY",
  "config_file": "config/llm.toml",
  "config_loaded": true
}
```

---

## 7. 安全考虑

### 7.1 文件权限（Unix）

```rust
// 设置配置文件权限为 600（仅 owner 可读写）
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
}
```

### 7.2 Windows 安全策略

> **Oracle 修复 #3**: Windows 无原生文件权限保护，采用警告+推荐环境变量策略。

```rust
// crates/app-service/src/config_loader.rs

#[cfg(windows)]
fn warn_if_plaintext_key(api_key: &str) {
    // 如果 api_key 不是 ${VAR} 形式，说明是明文存储
    if !api_key.starts_with("${") && !api_key.is_empty() {
        eprintln!("┌─────────────────────────────────────────────────────────────┐");
        eprintln!("│ WARN: API key stored in plaintext in config/llm.toml       │");
        eprintln!("│ This is insecure on Windows (no file permission protection) │");
        eprintln!("│                                                            │");
        eprintln!("│ Recommended: Use environment variable reference instead:    │");
        eprintln!("│   api_key = \"${{OPENAI_API_KEY}}\"                          │");
        eprintln!("│                                                            │");
        eprintln!("│ Then set the environment variable:                         │");
        eprintln!("│   set OPENAI_API_KEY=sk-xxxx                               │");
        eprintln!("└─────────────────────────────────────────────────────────────┘");
    }
}
```

### 7.3 Git 保护

更新 `.gitignore`：
```gitignore
# LLM 配置（可能包含敏感信息）
config/llm.toml
config/llm.*.toml
!config/llm.toml.example
```

### 7.4 日志脱敏

API Key 在日志中显示为 `sk-****`：
```rust
fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    } else {
        "****".to_string()
    }
}
```

### 7.5 推荐的安全实践

| 平台 | 推荐方式 | 说明 |
|------|----------|------|
| **Windows** | `${VAR}` 环境变量 | 唯一安全方式 |
| **macOS** | `${VAR}` 或 Keyring | Keyring 通过 `set-llm-api-key` 命令 |
| **Linux** | `${VAR}` 或文件权限 600 | 文件权限在 Linux 上有效 |

### 6.2 Git 保护

更新 `.gitignore`：
```gitignore
# LLM 配置（包含敏感信息）
config/llm.toml
config/llm.*.toml
!config/llm.toml.example
```

### 6.3 日志脱敏

API Key 在日志中显示为 `sk-****`：
```rust
fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    } else {
        "****".to_string()
    }
}
```

---

## 8. 实现步骤

### Phase 1: 核心配置加载（1-2天）

1. 添加依赖到 `Cargo.toml`：
   ```toml
   [dependencies]
   toml = "0.8"
   ```

2. 在 `core-domain/src/lib.rs` 添加 `LlmFileConfig` 结构体（仅数据结构）

3. 在 `app-service/src/config_loader.rs` 实现加载逻辑：
   - `load_llm_config_from_file()` - 文件读取 + TOML 解析
   - `load_llm_config()` - 默认路径 + profile 支持
   - `interpolate()` - 环境变量插值（支持 `$${LITERAL}` 转义）

4. 创建 `config/llm.toml.example` 文件

### Phase 2: 优先级合并 + V4 集成（1天）

1. 实现 `ResolvedLlmConfig::resolve()` 方法（CLI > File > Default）

2. 添加 `ConfigSource` 追踪配置来源

3. **V4 Skill 集成**：
   - `OpenAiProvider::from_config()` 使用 `ResolvedLlmConfig`
   - `LlmCallConfig` 从 `[llm.defaults]` 读取 temperature/max_tokens/seed

4. 更新 `AppContext::get_llm_config()` 使用新逻辑

### Phase 3: CLI 命令改造（1天）

1. 改造 `set-llm-config` 命令：
   - 读取现有 `config/llm.toml`（不存在则创建）
   - 更新对应字段
   - 写回文件

2. 改造 `set-llm-api-key` 命令：
   - 写入 `api_key = "${OPENAI_API_KEY}"` 到 TOML
   - 提示用户设置环境变量

3. 新增 `show-llm-config` 命令：
   - 显示解析后的配置
   - 显示每个字段的来源（file/cli/env/default）
   - `--validate` 标志检查 `${VAR}` 引用可解析

4. 新增 `migrate-llm-config` 命令：
   - 从 SQLite 读取现有 `llm_config`
   - 从 Keyring/SQLite 读取 `llm_api_key`
   - 写入 `config/llm.toml`
   - 已存在文件时拒绝覆盖（需 `--force`）

### Phase 4: 清理与文档（0.5天）

1. 更新 `.gitignore`

2. 更新 `README.md` 配置说明

3. 更新 `docs/日常操作手册.md`

4. 添加 Windows 安全警告

5. 可选：标记 Keyring 依赖为 optional（保留向后兼容）

---

## 9. 影响范围

### 需要修改的文件

| 文件 | 修改内容 |
|------|----------|
| `Cargo.toml` | 添加 `toml = "0.8"` 依赖 |
| `crates/core-domain/src/lib.rs` | 添加 `LlmFileConfig` 结构体定义 |
| `crates/app-service/src/lib.rs` | 修改 `get_llm_config()` 使用新逻辑 |
| `crates/app-service/src/config_loader.rs` | **新建** - 配置加载、插值、优先级合并 |
| `crates/research-skills/src/openai_provider.rs` | 使用 `ResolvedLlmConfig` 构造 provider |
| `apps/cli/src/main.rs` | 修改 CLI 命令，新增 `show-llm-config` |
| `.gitignore` | 添加 `config/llm.toml` 排除规则 |
| `config/llm.toml.example` | **新建** - 示例配置文件 |

### 不需要修改的部分

- `crates/research-skills/src/provider.rs` — `LlmProvider` trait 不变
- `crates/research-skills/src/executor.rs` — SkillExecutor 不变
- `crates/market-store/` — 存储层不变（保留 credential_store 向后兼容）
- 桌面端 — 通过 `AppContext` 间接使用，无需改动

---

## 10. 测试策略

1. **单元测试**：
   - 环境变量插值解析
   - 优先级合并逻辑
   - 默认值回退

2. **集成测试**：
   - 从文件加载配置
   - CLI 命令写入/读取
   - 迁移命令

3. **手动测试**：
   - 无配置文件场景
   - 环境变量覆盖场景
   - CLI 参数覆盖场景

---

## 11. 决策记录

### 为什么选择 TOML 而不是 JSON？

- TOML 支持注释，配置文件更易读
- TOML 是 Rust 生态标准（Cargo.toml）
- TOML 支持嵌套结构，表达力更强

### 为什么不用 `config` crate？

- `config` crate 功能过重，支持多种格式
- 我们只需要 TOML，直接用 `toml` crate 更简单
- 减少依赖，降低编译时间

### 为什么保留 CLI 命令？

- 向后兼容，不破坏现有工作流
- 为脚本/自动化提供接口
- 作为快速设置的便捷入口
