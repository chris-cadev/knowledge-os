use chrono::Local;
use log::LevelFilter;
use tauri::{AppHandle, Emitter};

const LOG_EVENT: &str = "log-entry";

pub fn init(data_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let log_file = data_dir.join("app.log");

    let console_config = fern::Dispatch::new()
        .format(|out, message, record| {
            let color = match record.level() {
                log::Level::Error => "\x1b[31m",
                log::Level::Warn => "\x1b[33m",
                log::Level::Info => "\x1b[32m",
                log::Level::Debug => "\x1b[34m",
                log::Level::Trace => "\x1b[90m",
            };
            out.finish(format_args!(
                "{}[{} {} {}]\x1b[0m {}",
                color,
                Local::now().format("%H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(std::io::stdout());

    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(fern::log_file(log_file)?);

    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    fern::Dispatch::new()
        .level(level)
        .level_for("hyper", LevelFilter::Warn)
        .level_for("reqwest", LevelFilter::Warn)
        .level_for("rusqlite", LevelFilter::Warn)
        .chain(console_config)
        .chain(file_config)
        .apply()?;

    log::info!(
        "logger.initialized: level={:?}, log_file={:?}",
        level,
        data_dir.join("app.log")
    );

    Ok(())
}

pub struct LogBridge {
    app: AppHandle,
}

impl LogBridge {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn emit(&self, level: log::Level, target: &str, message: &str) {
        let entry = serde_json::json!({
            "timestamp": Local::now().to_rfc3339(),
            "level": level.to_string(),
            "target": target,
            "message": message,
        });

        if let Err(e) = self.app.emit(LOG_EVENT, entry) {
            eprintln!("failed to emit log event: {}", e);
        }
    }
}

pub fn install_tauri_bridge(app: AppHandle) {
    let bridge = std::sync::Arc::new(LogBridge::new(app));

    log::set_boxed_logger(Box::new(BridgeLogger { inner: bridge }))
        .map(|_| {
            let level = if cfg!(debug_assertions) {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            };
            log::set_max_level(level);
        })
        .ok();
}

struct BridgeLogger {
    inner: std::sync::Arc<LogBridge>,
}

impl log::Log for BridgeLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        self.inner
            .emit(record.level(), record.target(), &record.args().to_string());
    }

    fn flush(&self) {}
}
