use std::path::PathBuf;

use crate::error::AppError;

pub const LAUNCH_AGENT_LABEL: &str = "local.netmeter.collector";
pub const COLLECTOR_BIN_NAME: &str = "netmeter-collector";

fn home_dir() -> Result<PathBuf, AppError> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(AppError::HomeNotSet)
}

/// `~/Library/Application Support/netmeter`: holds the database, the installed collector binary and its log.
pub fn data_dir() -> Result<PathBuf, AppError> {
    Ok(home_dir()?.join("Library/Application Support/netmeter"))
}

pub fn db_path() -> Result<PathBuf, AppError> {
    Ok(data_dir()?.join("usage.sqlite"))
}

/// Stable location the LaunchAgent runs the collector from, independent of where the GUI app lives.
pub fn installed_collector_path() -> Result<PathBuf, AppError> {
    Ok(data_dir()?.join(COLLECTOR_BIN_NAME))
}

pub fn collector_log_path() -> Result<PathBuf, AppError> {
    Ok(data_dir()?.join("collector.log"))
}

pub fn launch_agent_plist_path() -> Result<PathBuf, AppError> {
    Ok(home_dir()?
        .join("Library/LaunchAgents")
        .join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}
