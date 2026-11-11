use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::{AppError, io_error};

/// One row per local day and app; the collector keeps adding deltas to it.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS daily_usage (
    day       TEXT    NOT NULL, -- local date, YYYY-MM-DD
    app       TEXT    NOT NULL,
    bytes_in  INTEGER NOT NULL,
    bytes_out INTEGER NOT NULL,
    PRIMARY KEY (day, app)
) WITHOUT ROWID;
";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUsage {
    pub app: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayUsage {
    /// Local date, `YYYY-MM-DD`.
    pub day: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

pub fn init_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

/// Opens (creating if needed) the database in WAL mode so the GUI can read while the collector writes.
pub fn open(path: &Path) -> Result<Connection, AppError> {
    let dir = path.parent().expect("database path always has a parent directory");
    std::fs::create_dir_all(dir).map_err(io_error(dir))?;
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    let journal_mode: String = conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(AppError::WalUnavailable {
            path: path.to_path_buf(),
            journal_mode,
        });
    }
    init_schema(&conn)?;
    Ok(conn)
}

/// Adds per-app byte deltas to the totals of `day` in one transaction.
pub fn add_usage(conn: &mut Connection, day: &str, usages: &[AppUsage]) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO daily_usage (day, app, bytes_in, bytes_out) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (day, app) DO UPDATE SET
                 bytes_in  = bytes_in  + excluded.bytes_in,
                 bytes_out = bytes_out + excluded.bytes_out",
        )?;
        for usage in usages {
            stmt.execute(params![day, usage.app, usage.bytes_in, usage.bytes_out])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Months that have data, newest first, as `YYYY-MM`.
pub fn months(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT substr(day, 1, 7) AS month FROM daily_usage ORDER BY month DESC",
    )?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<Result<Vec<String>, rusqlite::Error>>()?)
}

/// Per-app totals of a `YYYY-MM` month, largest total first.
pub fn month_apps(conn: &Connection, month: &str) -> Result<Vec<AppUsage>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT app, SUM(bytes_in), SUM(bytes_out) FROM daily_usage
         WHERE substr(day, 1, 7) = ?1
         GROUP BY app
         ORDER BY SUM(bytes_in) + SUM(bytes_out) DESC",
    )?;
    let rows = stmt.query_map([month], |row| {
        Ok(AppUsage {
            app: row.get(0)?,
            bytes_in: row.get(1)?,
            bytes_out: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<AppUsage>, rusqlite::Error>>()?)
}

/// Daily totals of all apps in a `YYYY-MM` month, oldest day first. Days without data are absent.
pub fn month_days(conn: &Connection, month: &str) -> Result<Vec<DayUsage>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT day, SUM(bytes_in), SUM(bytes_out) FROM daily_usage
         WHERE substr(day, 1, 7) = ?1
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt.query_map([month], day_usage_from_row)?;
    Ok(rows.collect::<Result<Vec<DayUsage>, rusqlite::Error>>()?)
}

/// Daily totals of one app in a `YYYY-MM` month, oldest day first. Days without data are absent.
pub fn app_days(conn: &Connection, month: &str, app: &str) -> Result<Vec<DayUsage>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT day, bytes_in, bytes_out FROM daily_usage
         WHERE substr(day, 1, 7) = ?1 AND app = ?2
         ORDER BY day",
    )?;
    let rows = stmt.query_map([month, app], day_usage_from_row)?;
    Ok(rows.collect::<Result<Vec<DayUsage>, rusqlite::Error>>()?)
}

fn day_usage_from_row(row: &rusqlite::Row) -> Result<DayUsage, rusqlite::Error> {
    Ok(DayUsage {
        day: row.get(0)?,
        bytes_in: row.get(1)?,
        bytes_out: row.get(2)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(app: &str, bytes_in: u64, bytes_out: u64) -> AppUsage {
        AppUsage {
            app: app.to_string(),
            bytes_in,
            bytes_out,
        }
    }

    fn day(day: &str, bytes_in: u64, bytes_out: u64) -> DayUsage {
        DayUsage {
            day: day.to_string(),
            bytes_in,
            bytes_out,
        }
    }

    fn seeded() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        add_usage(&mut conn, "2026-10-31", &[usage("Chrome", 5, 1)]).unwrap();
        add_usage(&mut conn, "2026-11-01", &[usage("Chrome", 10, 2), usage("Drive", 3, 30)]).unwrap();
        add_usage(&mut conn, "2026-11-01", &[usage("Chrome", 100, 20)]).unwrap();
        add_usage(&mut conn, "2026-11-02", &[usage("Drive", 1, 1)]).unwrap();
        conn
    }

    #[test]
    fn lists_months_newest_first() {
        assert_eq!(months(&seeded()).unwrap(), vec!["2026-11", "2026-10"]);
    }

    #[test]
    fn sums_repeated_deltas_per_app_and_month() {
        assert_eq!(
            month_apps(&seeded(), "2026-11").unwrap(),
            vec![usage("Chrome", 110, 22), usage("Drive", 4, 31)]
        );
    }

    #[test]
    fn sums_days_across_apps() {
        assert_eq!(
            month_days(&seeded(), "2026-11").unwrap(),
            vec![day("2026-11-01", 113, 52), day("2026-11-02", 1, 1)]
        );
    }

    #[test]
    fn lists_days_of_one_app() {
        assert_eq!(
            app_days(&seeded(), "2026-11", "Drive").unwrap(),
            vec![day("2026-11-01", 3, 30), day("2026-11-02", 1, 1)]
        );
    }
}
