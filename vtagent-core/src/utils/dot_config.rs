//! Dot folder configuration and cache management

use crate::config::constants::defaults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// VTAgent configuration stored in ~/.vtagent/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotConfig {
    pub version: String,
    pub last_updated: u64,
    pub preferences: UserPreferences,
    pub providers: ProviderConfigs,
    pub cache: CacheConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub default_model: String,
    pub default_provider: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub auto_save: bool,
    pub theme: String,
    pub keybindings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigs {
    pub openai: Option<ProviderConfig>,
    pub anthropic: Option<ProviderConfig>,
    pub gemini: Option<ProviderConfig>,
    pub openrouter: Option<ProviderConfig>,
    pub lmstudio: Option<ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
    pub priority: i32, // Higher priority = preferred
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size_mb: u64,
    pub ttl_days: u64,
    pub prompt_cache_enabled: bool,
    pub context_cache_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub show_timestamps: bool,
    pub max_output_lines: usize,
    pub syntax_highlighting: bool,
    pub auto_complete: bool,
    pub history_size: usize,
}

impl Default for DotConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            preferences: UserPreferences::default(),
            providers: ProviderConfigs::default(),
            cache: CacheConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_model: defaults::DEFAULT_MODEL.to_string(),
            default_provider: defaults::DEFAULT_PROVIDER.to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            auto_save: true,
            theme: "dark".to_string(),
            keybindings: HashMap::new(),
        }
    }
}

impl Default for ProviderConfigs {
    fn default() -> Self {
        Self {
            openai: None,
            anthropic: None,
            gemini: None,
            openrouter: None,
            lmstudio: None,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: 100,
            ttl_days: 30,
            prompt_cache_enabled: true,
            context_cache_enabled: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_timestamps: true,
            max_output_lines: 1000,
            syntax_highlighting: true,
            auto_complete: true,
            history_size: 1000,
        }
    }
}

/// Dot folder manager for VTAgent configuration and cache
pub struct DotManager {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    config_file: PathBuf,
}

impl DotManager {
    pub fn new() -> Result<Self, DotError> {
        let home_dir = dirs::home_dir().ok_or_else(|| DotError::HomeDirNotFound)?;

        let config_dir = home_dir.join(".vtagent");
        let cache_dir = config_dir.join("cache");
        let config_file = config_dir.join("config.toml");

        Ok(Self {
            config_dir,
            cache_dir,
            config_file,
        })
    }

    /// Initialize the dot folder structure
    pub fn initialize(&self) -> Result<(), DotError> {
        // Create directories
        fs::create_dir_all(&self.config_dir).map_err(|e| DotError::Io(e))?;
        fs::create_dir_all(&self.cache_dir).map_err(|e| DotError::Io(e))?;

        // Create subdirectories
        let subdirs = [
            "cache/prompts",
            "cache/context",
            "cache/models",
            "logs",
            "sessions",
            "backups",
        ];

        for subdir in &subdirs {
            fs::create_dir_all(self.config_dir.join(subdir)).map_err(|e| DotError::Io(e))?;
        }

        // Create default config if it doesn't exist
        if !self.config_file.exists() {
            let default_config = DotConfig::default();
            self.save_config(&default_config)?;
        }

        Ok(())
    }

    /// Load configuration from disk
    pub fn load_config(&self) -> Result<DotConfig, DotError> {
        if !self.config_file.exists() {
            return Ok(DotConfig::default());
        }

        let content = fs::read_to_string(&self.config_file).map_err(|e| DotError::Io(e))?;

        toml::from_str(&content).map_err(|e| DotError::TomlDe(e))
    }

    /// Save configuration to disk
    pub fn save_config(&self, config: &DotConfig) -> Result<(), DotError> {
        let content = toml::to_string_pretty(config).map_err(|e| DotError::Toml(e))?;

        fs::write(&self.config_file, content).map_err(|e| DotError::Io(e))?;

        Ok(())
    }

    /// Update configuration with new values
    pub fn update_config<F>(&self, updater: F) -> Result<(), DotError>
    where
        F: FnOnce(&mut DotConfig),
    {
        let mut config = self.load_config()?;
        updater(&mut config);
        config.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.save_config(&config)
    }

    /// Get cache directory for a specific type
    pub fn cache_dir(&self, cache_type: &str) -> PathBuf {
        self.cache_dir.join(cache_type)
    }

    /// Get logs directory
    pub fn logs_dir(&self) -> PathBuf {
        self.config_dir.join("logs")
    }

    /// Get sessions directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.config_dir.join("sessions")
    }

    /// Get backups directory
    pub fn backups_dir(&self) -> PathBuf {
        self.config_dir.join("backups")
    }

    /// Clean up old cache files
    pub fn cleanup_cache(&self) -> Result<CacheCleanupStats, DotError> {
        let config = self.load_config()?;
        let max_age = std::time::Duration::from_secs(config.cache.ttl_days * 24 * 60 * 60);
        let now = std::time::SystemTime::now();

        let mut stats = CacheCleanupStats::default();

        // Clean prompt cache
        if config.cache.prompt_cache_enabled {
            stats.prompts_cleaned =
                self.cleanup_directory(&self.cache_dir("prompts"), max_age, now)?;
        }

        // Clean context cache
        if config.cache.context_cache_enabled {
            stats.context_cleaned =
                self.cleanup_directory(&self.cache_dir("context"), max_age, now)?;
        }

        // Clean model cache
        stats.models_cleaned = self.cleanup_directory(&self.cache_dir("models"), max_age, now)?;

        Ok(stats)
    }

    /// Clean up files in a directory older than max_age
    fn cleanup_directory(
        &self,
        dir: &Path,
        max_age: std::time::Duration,
        now: std::time::SystemTime,
    ) -> Result<u64, DotError> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut cleaned = 0u64;

        for entry in fs::read_dir(dir).map_err(|e| DotError::Io(e))? {
            let entry = entry.map_err(|e| DotError::Io(e))?;
            let path = entry.path();

            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            if path.is_file() {
                                fs::remove_file(&path).map_err(|e| DotError::Io(e))?;
                                cleaned += 1;
                            } else if path.is_dir() {
                                fs::remove_dir_all(&path).map_err(|e| DotError::Io(e))?;
                                cleaned += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(cleaned)
    }

    /// Get disk usage statistics
    pub fn disk_usage(&self) -> Result<DiskUsageStats, DotError> {
        let mut stats = DiskUsageStats::default();

        stats.config_size = self.calculate_dir_size(&self.config_dir)?;
        stats.cache_size = self.calculate_dir_size(&self.cache_dir)?;
        stats.logs_size = self.calculate_dir_size(&self.logs_dir())?;
        stats.sessions_size = self.calculate_dir_size(&self.sessions_dir())?;
        stats.backups_size = self.calculate_dir_size(&self.backups_dir())?;

        stats.total_size = stats.config_size
            + stats.cache_size
            + stats.logs_size
            + stats.sessions_size
            + stats.backups_size;

        Ok(stats)
    }

    /// Calculate directory size recursively
    fn calculate_dir_size(&self, dir: &Path) -> Result<u64, DotError> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut size = 0u64;

        fn calculate_recursive(path: &Path, current_size: &mut u64) -> Result<(), DotError> {
            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    *current_size += metadata.len();
                }
            } else if path.is_dir() {
                for entry in fs::read_dir(path).map_err(|e| DotError::Io(e))? {
                    let entry = entry.map_err(|e| DotError::Io(e))?;
                    calculate_recursive(&entry.path(), current_size)?;
                }
            }
            Ok(())
        }

        calculate_recursive(dir, &mut size)?;
        Ok(size)
    }

    /// Backup current configuration
    pub fn backup_config(&self) -> Result<PathBuf, DotError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let backup_name = format!("config_backup_{}.toml", timestamp);
        let backup_path = self.backups_dir().join(backup_name);

        if self.config_file.exists() {
            fs::copy(&self.config_file, &backup_path).map_err(|e| DotError::Io(e))?;
        }

        Ok(backup_path)
    }

    /// List available backups
    pub fn list_backups(&self) -> Result<Vec<PathBuf>, DotError> {
        let backups_dir = self.backups_dir();
        if !backups_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups = vec![];

        for entry in fs::read_dir(backups_dir).map_err(|e| DotError::Io(e))? {
            let entry = entry.map_err(|e| DotError::Io(e))?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("toml") {
                backups.push(entry.path());
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        Ok(backups)
    }

    /// Restore configuration from backup
    pub fn restore_backup(&self, backup_path: &Path) -> Result<(), DotError> {
        if !backup_path.exists() {
            return Err(DotError::BackupNotFound(backup_path.to_path_buf()));
        }

        fs::copy(backup_path, &self.config_file).map_err(|e| DotError::Io(e))?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct CacheCleanupStats {
    pub prompts_cleaned: u64,
    pub context_cleaned: u64,
    pub models_cleaned: u64,
}

#[derive(Debug, Default)]
pub struct DiskUsageStats {
    pub config_size: u64,
    pub cache_size: u64,
    pub logs_size: u64,
    pub sessions_size: u64,
    pub backups_size: u64,
    pub total_size: u64,
}

/// Dot folder management errors
#[derive(Debug, thiserror::Error)]
pub enum DotError {
    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("Backup not found: {0}")]
    BackupNotFound(PathBuf),
}

use std::sync::{LazyLock, Mutex};

/// Global dot manager instance
static DOT_MANAGER: LazyLock<Mutex<DotManager>> =
    LazyLock::new(|| Mutex::new(DotManager::new().unwrap()));

/// Get global dot manager instance
pub fn get_dot_manager() -> &'static Mutex<DotManager> {
    &DOT_MANAGER
}

/// Initialize dot folder (should be called at startup)
pub fn initialize_dot_folder() -> Result<(), DotError> {
    let manager = get_dot_manager().lock().unwrap();
    manager.initialize()
}

/// Load user configuration
pub fn load_user_config() -> Result<DotConfig, DotError> {
    let manager = get_dot_manager().lock().unwrap();
    manager.load_config()
}

/// Save user configuration
pub fn save_user_config(config: &DotConfig) -> Result<(), DotError> {
    let manager = get_dot_manager().lock().unwrap();
    manager.save_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dot_manager_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".vtagent");

        // Test directory creation
        assert!(!config_dir.exists());

        let manager = DotManager {
            config_dir: config_dir.clone(),
            cache_dir: config_dir.join("cache"),
            config_file: config_dir.join("config.toml"),
        };

        manager.initialize().unwrap();
        assert!(config_dir.exists());
        assert!(config_dir.join("cache").exists());
        assert!(config_dir.join("logs").exists());
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".vtagent");

        let manager = DotManager {
            config_dir: config_dir.clone(),
            cache_dir: config_dir.join("cache"),
            config_file: config_dir.join("config.toml"),
        };

        manager.initialize().unwrap();

        let mut config = DotConfig::default();
        config.preferences.default_model = "test-model".to_string();

        manager.save_config(&config).unwrap();
        let loaded_config = manager.load_config().unwrap();

        assert_eq!(loaded_config.preferences.default_model, "test-model");
    }
}
