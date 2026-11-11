use crate::error::AppError;

use std::process::{Command, Stdio};

pub const NETTOP_PATH: &str = "/usr/bin/nettop";
/// `script` runs nettop on a pseudo-terminal: nettop block-buffers its output when writing to a pipe,
/// which would delay samples by minutes. On a terminal it flushes every line (ending in `\r\n`).
pub const SCRIPT_PATH: &str = "/usr/bin/script";

/// Endless per-process log (`-L 0`) in delta mode (`-d`), one sample every 10 s, raw byte counts (`-x`).
/// The first sample block holds totals of currently open connections; every later block holds bytes
/// moved since the previous block, including connections that closed in between.
pub const NETTOP_ARGS: [&str; 9] = [
    "-P",
    "-d",
    "-L",
    "0",
    "-s",
    "10",
    "-x",
    "-J",
    "bytes_in,bytes_out",
];

const HEADER_LINE: &str = ",bytes_in,bytes_out,";

/// nettop with [`NETTOP_ARGS`] on a pseudo-terminal, with stdin and stdout piped to the caller.
/// The caller must keep the child's stdin open: on stdin EOF `script` echoes `^D` into the output.
pub fn command() -> Command {
    let mut command = Command::new(SCRIPT_PATH);
    command
        .args(["-q", "/dev/null", NETTOP_PATH])
        .args(NETTOP_ARGS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    command
}

/// One line of `nettop` output produced with [`NETTOP_ARGS`].
#[derive(Debug, PartialEq, Eq)]
pub enum NettopLine {
    /// `,bytes_in,bytes_out,`: starts a new sample block.
    Header,
    /// `<process name>.<pid>,<bytes_in>,<bytes_out>,`
    Row(ProcessSample),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSample {
    /// Process name as nettop prints it (truncated to 15 characters).
    pub process_name: String,
    pub pid: i32,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

fn parse_error(line: &str, reason: &str) -> AppError {
    AppError::NettopParse {
        line: line.to_string(),
        reason: reason.to_string(),
    }
}

/// Parses one output line, ignoring the terminal's trailing `\r`. Fields are split from the right
/// because process names may contain commas.
pub fn parse_line(raw_line: &str) -> Result<NettopLine, AppError> {
    let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    if line == HEADER_LINE {
        return Ok(NettopLine::Header);
    }
    let body = line
        .strip_suffix(',')
        .ok_or_else(|| parse_error(line, "missing trailing comma"))?;
    let mut fields = body.rsplitn(3, ',');
    let bytes_out = fields.next().ok_or_else(|| parse_error(line, "missing bytes_out"))?;
    let bytes_in = fields.next().ok_or_else(|| parse_error(line, "missing bytes_in"))?;
    let name_and_pid = fields.next().ok_or_else(|| parse_error(line, "missing process"))?;
    let (process_name, pid) = name_and_pid
        .rsplit_once('.')
        .ok_or_else(|| parse_error(line, "missing .pid suffix"))?;
    Ok(NettopLine::Row(ProcessSample {
        process_name: process_name.to_string(),
        pid: pid.parse().map_err(|_| parse_error(line, "pid is not an integer"))?,
        bytes_in: bytes_in.parse().map_err(|_| parse_error(line, "bytes_in is not an unsigned integer"))?,
        bytes_out: bytes_out.parse().map_err(|_| parse_error(line, "bytes_out is not an unsigned integer"))?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header() {
        assert_eq!(parse_line(",bytes_in,bytes_out,").unwrap(), NettopLine::Header);
        assert_eq!(parse_line(",bytes_in,bytes_out,\r").unwrap(), NettopLine::Header);
    }

    #[test]
    fn parses_terminal_row() {
        let NettopLine::Row(sample) = parse_line("apsd.578,10,20,\r").unwrap() else {
            panic!("expected row");
        };
        assert_eq!((sample.pid, sample.bytes_in, sample.bytes_out), (578, 10, 20));
    }

    #[test]
    fn parses_row_with_spaces_and_dots_in_name() {
        assert_eq!(
            parse_line("2.1.322.34148,861837,405557,").unwrap(),
            NettopLine::Row(ProcessSample {
                process_name: "2.1.322".to_string(),
                pid: 34148,
                bytes_in: 861837,
                bytes_out: 405557,
            })
        );
        assert_eq!(
            parse_line("Google Chrome H.1921,45526945,3454765,").unwrap(),
            NettopLine::Row(ProcessSample {
                process_name: "Google Chrome H".to_string(),
                pid: 1921,
                bytes_in: 45526945,
                bytes_out: 3454765,
            })
        );
    }

    #[test]
    fn parses_row_with_comma_in_name() {
        let NettopLine::Row(sample) = parse_line("a,b.7,1,2,").unwrap() else {
            panic!("expected row");
        };
        assert_eq!(sample.process_name, "a,b");
    }

    #[test]
    fn parses_name_cut_inside_multibyte_character() {
        let raw: &[u8] = b"aaaaaaaaaaaaaa\xc4.41511,131072,131072,\r";
        let NettopLine::Row(sample) = parse_line(&String::from_utf8_lossy(raw)).unwrap() else {
            panic!("expected row");
        };
        assert_eq!((sample.pid, sample.bytes_in, sample.bytes_out), (41511, 131072, 131072));
        assert!(sample.process_name.starts_with("aaaaaaaaaaaaaa"));
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(parse_line("").is_err());
        assert!(parse_line("launchd.1,0,0").is_err());
        assert!(parse_line("launchd,0,0,").is_err());
        assert!(parse_line("launchd.1,-5,0,").is_err());
    }
}
