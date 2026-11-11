use std::path::{Path, PathBuf};
use std::process::ExitStatus;

/// Every failure raised by the collector, the database layer and the LaunchAgent manager.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("HOME environment variable is not set")]
    HomeNotSet,

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("cannot enable WAL journal mode on {path} (got {journal_mode:?}); GUI reads would block collector writes")]
    WalUnavailable { path: PathBuf, journal_mode: String },

    #[error("cannot parse nettop line {line:?}: {reason}")]
    NettopParse { line: String, reason: String },

    #[error("nettop stdout pipe is not available")]
    NettopStdoutMissing,

    #[error("nettop exited unexpectedly with {status}")]
    NettopExited { status: ExitStatus },

    #[error("`launchctl {args}` failed with {status}: {stderr}")]
    Launchctl {
        args: String,
        status: ExitStatus,
        stderr: String,
    },

    #[error("collector binary not found at {path}; build it with `cargo build --bins`")]
    CollectorBinaryMissing { path: PathBuf },
}

/// Builds a closure that wraps an `io::Error` with the path it happened at.
pub fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> AppError + '_ {
    move |source| AppError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Tauri commands send errors to the frontend as their display string.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
