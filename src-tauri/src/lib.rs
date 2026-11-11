pub mod app_name;
pub mod collector;
pub mod db;
pub mod error;
pub mod launch_agent;
pub mod nettop;
pub mod paths;

use db::{AppUsage, DayUsage};
use error::AppError;
use launch_agent::CollectorStatus;

fn open_db() -> Result<rusqlite::Connection, AppError> {
    db::open(&paths::db_path()?)
}

#[tauri::command]
fn list_months() -> Result<Vec<String>, AppError> {
    db::months(&open_db()?)
}

#[tauri::command]
fn month_apps(month: String) -> Result<Vec<AppUsage>, AppError> {
    db::month_apps(&open_db()?, &month)
}

#[tauri::command]
fn month_days(month: String) -> Result<Vec<DayUsage>, AppError> {
    db::month_days(&open_db()?, &month)
}

#[tauri::command]
fn app_days(month: String, app: String) -> Result<Vec<DayUsage>, AppError> {
    db::app_days(&open_db()?, &month, &app)
}

// The collector commands run `launchctl`, so they are `async` to keep it off the UI thread.
#[tauri::command(async)]
fn collector_status() -> Result<CollectorStatus, AppError> {
    launch_agent::status()
}

#[tauri::command(async)]
fn install_collector() -> Result<CollectorStatus, AppError> {
    launch_agent::install()?;
    launch_agent::status()
}

#[tauri::command(async)]
fn uninstall_collector() -> Result<CollectorStatus, AppError> {
    launch_agent::uninstall()?;
    launch_agent::status()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_months,
            month_apps,
            month_days,
            app_days,
            collector_status,
            install_collector,
            uninstall_collector
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
