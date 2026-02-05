use crate::config::{get_config_path, get_log_file_path, get_logs_dir, load_config};
use log::LevelFilter;
use simplelog::*;
use std::fs::File;

/// Initialize the logging system based on config file settings
/// Creates a new timestamped log file for each app session
pub fn init_logging() {
    let config = load_config();
    let log_path = get_log_file_path();

    // Parse log level from config
    let level = match config.logging.level.to_uppercase().as_str() {
        "DEBUG" => LevelFilter::Debug,
        "INFO" => LevelFilter::Info,
        "WARN" | "WARNING" => LevelFilter::Warn,
        "ERROR" => LevelFilter::Error,
        "TRACE" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    // Build logger configuration with timestamps
    let log_config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_target_level(LevelFilter::Error)
        .set_location_level(LevelFilter::Debug)
        .set_thread_level(LevelFilter::Off)
        .build();

    let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();

    // Console logger (for development)
    if config.logging.log_to_console {
        loggers.push(TermLogger::new(
            level,
            log_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ));
    }

    // File logger - always create a new timestamped file
    if let Ok(file) = File::create(&log_path) {
        loggers.push(WriteLogger::new(level, log_config.clone(), file));
    }

    // Initialize combined logger
    if !loggers.is_empty() {
        CombinedLogger::init(loggers).ok();
    }

    // Log startup information
    log::info!("========================================");
    log::info!("Jira Viewer - Session Started");
    log::info!("========================================");
    log::info!("Log level: {:?}", level);
    log::info!("Log file: {:?}", log_path);
    log::info!("Config file: {:?}", get_config_path());
    log::info!("Logs directory: {:?}", get_logs_dir());
    log::debug!("Debug logging is ENABLED - raw API data will be logged");
}
