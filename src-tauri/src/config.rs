use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String, // "DEBUG", "INFO", "WARN", "ERROR"
    #[serde(default = "default_log_to_console")]
    pub log_to_console: bool,
}

fn default_log_to_console() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "INFO".to_string(),
            log_to_console: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            logging: LoggingConfig::default(),
        }
    }
}

/// Get the config directory path
pub fn get_config_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "jira", "viewer") {
        let config_dir = proj_dirs.config_dir().to_path_buf();
        fs::create_dir_all(&config_dir).ok();
        config_dir
    } else {
        // Fallback to current directory
        PathBuf::from(".")
    }
}

/// Get the config file path
pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

/// Get the logs directory path
pub fn get_logs_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "jira", "viewer") {
        let logs_dir = proj_dirs.data_dir().join("logs");
        fs::create_dir_all(&logs_dir).ok();
        logs_dir
    } else {
        // Fallback to temp directory
        let logs_dir = std::env::temp_dir().join("jira_viewer_logs");
        fs::create_dir_all(&logs_dir).ok();
        logs_dir
    }
}

/// Generate a timestamped log file path for this session
pub fn get_log_file_path() -> PathBuf {
    let logs_dir = get_logs_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    logs_dir.join(format!("jira_viewer_{}.log", timestamp))
}

/// Load configuration from file, or create default if not exists
pub fn load_config() -> AppConfig {
    let config_path = get_config_path();

    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }

    // Create default config file if it doesn't exist
    let default_config = AppConfig::default();
    let toml_content = r#"# jira-viewer configuration

[logging]
# Log level: "DEBUG", "INFO", "WARN", "ERROR"
# DEBUG includes raw HTTP request/response data
# INFO shows API calls and results (recommended for normal use)
level = "INFO"

# Whether to also log to console (useful for development)
log_to_console = true
"#;

    fs::write(&config_path, toml_content).ok();
    default_config
}
