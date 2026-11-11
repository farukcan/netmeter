use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde::Serialize;

use crate::error::{AppError, io_error};
use crate::paths::{self, COLLECTOR_BIN_NAME, LAUNCH_AGENT_LABEL};

const LAUNCHCTL_PATH: &str = "/bin/launchctl";
/// `launchctl print` exit code for a service launchd has not loaded.
const SERVICE_NOT_FOUND_EXIT_CODE: i32 = 113;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorStatus {
    /// The LaunchAgent plist exists, so the collector starts at every login.
    pub installed: bool,
    /// launchd has the collector loaded and its process is running.
    pub running: bool,
    pub pid: Option<i32>,
}

fn gui_domain() -> String {
    // SAFETY: getuid has no preconditions and cannot fail.
    format!("gui/{}", unsafe { libc::getuid() })
}

fn service_target() -> String {
    format!("{}/{LAUNCH_AGENT_LABEL}", gui_domain())
}

fn launchctl(args: &[&str]) -> Result<Output, AppError> {
    Command::new(LAUNCHCTL_PATH)
        .args(args)
        .output()
        .map_err(io_error(Path::new(LAUNCHCTL_PATH)))
}

fn launchctl_error(args: &[&str], output: &Output) -> AppError {
    AppError::Launchctl {
        args: args.join(" "),
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    }
}

fn launchctl_checked(args: &[&str]) -> Result<(), AppError> {
    let output = launchctl(args)?;
    if output.status.success() {
        return Ok(());
    }
    Err(launchctl_error(args, &output))
}

/// `launchctl print` output of the collector service; `None` when launchd has not loaded it.
fn print_service() -> Result<Option<String>, AppError> {
    let target = service_target();
    let args = ["print", target.as_str()];
    let output = launchctl(&args)?;
    if output.status.code() == Some(SERVICE_NOT_FOUND_EXIT_CODE) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(launchctl_error(&args, &output));
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
}

/// Reads the service's top-level `pid = N` line, which is absent while the job is not running.
fn parse_pid(print_output: &str) -> Option<i32> {
    print_output
        .lines()
        .find_map(|line| line.strip_prefix("\tpid = "))
        .and_then(|pid| pid.trim().parse().ok())
}

fn render_plist(program: &Path, log: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LAUNCH_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{program}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Background</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>
"#,
        program = program.display(),
        log = log.display(),
    )
}

pub fn status() -> Result<CollectorStatus, AppError> {
    let installed = paths::launch_agent_plist_path()?.exists();
    let pid = print_service()?.as_deref().and_then(parse_pid);
    Ok(CollectorStatus {
        installed,
        running: pid.is_some(),
        pid,
    })
}

/// Copies the collector built next to the GUI executable into the data directory, writes the
/// LaunchAgent plist and (re)loads it, so the collector runs now and at every login whether or not
/// the GUI is open.
pub fn install() -> Result<(), AppError> {
    let exe = std::env::current_exe().map_err(io_error(Path::new("<current executable>")))?;
    let source = exe.with_file_name(COLLECTOR_BIN_NAME);
    if !source.exists() {
        return Err(AppError::CollectorBinaryMissing { path: source });
    }
    let data_dir = paths::data_dir()?;
    fs::create_dir_all(&data_dir).map_err(io_error(&data_dir))?;

    // Copy, then rename over the target, so a running collector keeps its old file intact.
    let target = paths::installed_collector_path()?;
    let staging = target.with_extension("new");
    fs::copy(&source, &staging).map_err(io_error(&staging))?;
    fs::rename(&staging, &target).map_err(io_error(&target))?;

    let plist_path = paths::launch_agent_plist_path()?;
    let plist_dir = plist_path.parent().expect("plist path always has a parent directory");
    fs::create_dir_all(plist_dir).map_err(io_error(plist_dir))?;
    fs::write(&plist_path, render_plist(&target, &paths::collector_log_path()?))
        .map_err(io_error(&plist_path))?;

    if print_service()?.is_some() {
        launchctl_checked(&["bootout", &service_target()])?;
    }
    launchctl_checked(&["bootstrap", &gui_domain(), &plist_path.to_string_lossy()])
}

/// Stops the collector and removes its LaunchAgent plist. Collected data and the installed binary stay.
pub fn uninstall() -> Result<(), AppError> {
    if print_service()?.is_some() {
        launchctl_checked(&["bootout", &service_target()])?;
    }
    let plist_path = paths::launch_agent_plist_path()?;
    if plist_path.exists() {
        fs::remove_file(&plist_path).map_err(io_error(&plist_path))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_top_level_pid_only() {
        let output = "gui/501/local.netmeter.collector = {\n\tactive count = 1\n\tpid = 4242\n\tendpoints = {\n\t\tpid = 1\n\t}\n}";
        assert_eq!(parse_pid(output), Some(4242));
        assert_eq!(parse_pid("gui/501/x = {\n\tstate = not running\n}"), None);
    }

    #[test]
    fn plist_points_at_program_and_log() {
        let plist = render_plist(Path::new("/a b/collector"), Path::new("/a b/collector.log"));
        assert!(plist.contains("<string>/a b/collector</string>"));
        assert!(plist.contains("<string>/a b/collector.log</string>"));
        assert!(plist.contains(LAUNCH_AGENT_LABEL));
    }
}
