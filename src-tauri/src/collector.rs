use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::app_name::resolve_app_name;
use crate::db::{self, AppUsage};
use crate::error::{AppError, io_error};
use crate::nettop::{self, NETTOP_PATH, NettopLine, SCRIPT_PATH};
use crate::paths;

/// A delta-block row with its app name already resolved.
struct NamedSample {
    app: String,
    bytes_in: u64,
    bytes_out: u64,
}

/// Sums samples per app, dropping apps with no traffic.
fn aggregate_by_app(samples: &[NamedSample]) -> Vec<AppUsage> {
    let mut totals: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
    for sample in samples {
        let entry = totals.entry(sample.app.as_str()).or_insert((0, 0));
        entry.0 += sample.bytes_in;
        entry.1 += sample.bytes_out;
    }
    totals
        .into_iter()
        .filter(|(_, (bytes_in, bytes_out))| bytes_in + bytes_out > 0)
        .map(|(app, (bytes_in, bytes_out))| AppUsage {
            app: app.to_string(),
            bytes_in,
            bytes_out,
        })
        .collect()
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Runs nettop forever and adds every delta block's per-app bytes to today's totals.
/// Returns only with an error; the LaunchAgent's `KeepAlive` restarts the collector.
pub fn run() -> Result<(), AppError> {
    let mut conn = db::open(&paths::db_path()?)?;
    let nettop_path = Path::new(NETTOP_PATH);
    let mut child = nettop::command()
        .spawn()
        .map_err(io_error(Path::new(SCRIPT_PATH)))?;
    let stdout = child.stdout.take().ok_or(AppError::NettopStdoutMissing)?;
    // Held until this function returns; see `nettop::command`.
    let _stdin = child.stdin.take();

    // A header closes the previous block. The block closed by the second header is the first one,
    // which holds totals rather than deltas, so storing starts at the third header.
    let mut headers_seen: u64 = 0;
    let mut block: Vec<NamedSample> = Vec::new();
    // Lines are read as bytes: nettop cuts process names at a byte boundary, which can split a
    // multi-byte character. Lossy decoding only affects that name; pid and byte counts are ASCII.
    for line in BufReader::new(stdout).split(b'\n') {
        let bytes: Vec<u8> = line.map_err(io_error(nettop_path))?;
        match nettop::parse_line(&String::from_utf8_lossy(&bytes))? {
            NettopLine::Header => {
                if headers_seen >= 2 {
                    db::add_usage(&mut conn, &today(), &aggregate_by_app(&block))?;
                }
                block.clear();
                headers_seen += 1;
            }
            NettopLine::Row(sample) if sample.bytes_in + sample.bytes_out > 0 => {
                // Resolve right away: short-lived processes may exit before the block ends.
                block.push(NamedSample {
                    app: resolve_app_name(&sample),
                    bytes_in: sample.bytes_in,
                    bytes_out: sample.bytes_out,
                });
            }
            NettopLine::Row(_) => {}
        }
    }
    let status = child.wait().map_err(io_error(nettop_path))?;
    Err(AppError::NettopExited { status })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(app: &str, bytes_in: u64, bytes_out: u64) -> NamedSample {
        NamedSample {
            app: app.to_string(),
            bytes_in,
            bytes_out,
        }
    }

    #[test]
    fn aggregates_processes_of_same_app() {
        let samples = vec![
            named("Google Chrome", 10, 1),
            named("mDNSResponder", 2, 2),
            named("Google Chrome", 5, 4),
            named("idle", 0, 0),
        ];
        assert_eq!(
            aggregate_by_app(&samples),
            vec![
                AppUsage {
                    app: "Google Chrome".to_string(),
                    bytes_in: 15,
                    bytes_out: 5
                },
                AppUsage {
                    app: "mDNSResponder".to_string(),
                    bytes_in: 2,
                    bytes_out: 2
                },
            ]
        );
    }
}
