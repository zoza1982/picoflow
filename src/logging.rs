//! Structured logging configuration using tracing

use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::FmtSubscriber;

/// Log level configuration
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info, // Default
        }
    }
}

/// Log format configuration
#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Json,
    Pretty,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: LogLevel,
    pub format: LogFormat,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
        }
    }
}

/// Initialize logging with the given configuration
pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    let level: Level = config.level.into();

    match config.format {
        LogFormat::Json => {
            let subscriber = FmtSubscriber::builder()
                .json()
                .with_max_level(level)
                .with_span_events(FmtSpan::CLOSE) // Only log on span close
                .with_writer(std::io::stderr) // Write to stderr, no buffering
                .finish();

            tracing::subscriber::set_global_default(subscriber)?;
        }
        LogFormat::Pretty => {
            let subscriber = FmtSubscriber::builder()
                .with_max_level(level)
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(std::io::stderr)
                .finish();

            tracing::subscriber::set_global_default(subscriber)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        let level: LogLevel = "error".into();
        assert!(matches!(level, LogLevel::Error));

        let level: LogLevel = "INFO".into();
        assert!(matches!(level, LogLevel::Info));

        let level: LogLevel = "debug".into();
        assert!(matches!(level, LogLevel::Debug));

        // Unknown defaults to Info
        let level: LogLevel = "unknown".into();
        assert!(matches!(level, LogLevel::Info));
    }

    #[test]
    fn test_log_level_to_tracing_level() {
        let level: Level = LogLevel::Error.into();
        assert_eq!(level, Level::ERROR);

        let level: Level = LogLevel::Info.into();
        assert_eq!(level, Level::INFO);

        let level: Level = LogLevel::Debug.into();
        assert_eq!(level, Level::DEBUG);
    }

    #[test]
    fn test_default_log_config() {
        let config = LogConfig::default();
        assert!(matches!(config.level, LogLevel::Info));
        assert!(matches!(config.format, LogFormat::Json));
    }
}
