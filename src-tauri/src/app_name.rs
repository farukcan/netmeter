use crate::nettop::ProcessSample;

/// Names a process after the outermost `.app` bundle in its executable path, so helper processes
/// count toward their parent app (Chrome helpers → "Google Chrome"). Executables outside a bundle
/// use their file name (`/usr/sbin/mDNSResponder` → "mDNSResponder").
pub fn app_name_from_path(path: &str) -> String {
    match path.split('/').find_map(|component| component.strip_suffix(".app")) {
        Some(bundle) => bundle.to_string(),
        None => path.rsplit_once('/').map_or(path, |(_, file)| file).to_string(),
    }
}

/// Executable path of a process; `None` when it is unavailable (the process exited, the kernel
/// denied access, or the path is not valid UTF-8).
fn process_path(pid: i32) -> Option<String> {
    let mut buffer: Vec<u8> = vec![0; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: `buffer` is valid for writes of `buffer.len()` bytes for the whole call.
    let written = unsafe { libc::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if written <= 0 {
        return None;
    }
    buffer.truncate(written as usize);
    String::from_utf8(buffer).ok()
}

/// App name for a sample. A process whose executable path is unavailable (typically because it
/// exited before the lookup) keeps nettop's 15-character process name.
pub fn resolve_app_name(sample: &ProcessSample) -> String {
    process_path(sample.pid).map_or_else(
        || sample.process_name.clone(),
        |path| app_name_from_path(&path),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_helpers_under_outermost_bundle() {
        assert_eq!(
            app_name_from_path(
                "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/1/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper"
            ),
            "Google Chrome"
        );
    }

    #[test]
    fn uses_file_name_outside_bundles() {
        assert_eq!(app_name_from_path("/usr/sbin/mDNSResponder"), "mDNSResponder");
        assert_eq!(app_name_from_path("curl"), "curl");
    }

    #[test]
    fn resolves_own_process_by_pid() {
        let sample = ProcessSample {
            process_name: "unused".to_string(),
            pid: std::process::id() as i32,
            bytes_in: 0,
            bytes_out: 0,
        };
        assert_ne!(resolve_app_name(&sample), "unused");
    }

    #[test]
    fn keeps_nettop_name_for_missing_process() {
        let sample = ProcessSample {
            process_name: "gone".to_string(),
            pid: i32::MAX,
            bytes_in: 0,
            bytes_out: 0,
        };
        assert_eq!(resolve_app_name(&sample), "gone");
    }
}
