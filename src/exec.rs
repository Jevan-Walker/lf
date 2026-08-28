//! `lf exec` — a single, deterministic command-execution entry for AI agents.
//!
//! The agent calls `lf exec <command>` and never has to know whether the host
//! is Windows/macOS/Linux, Bash/PowerShell/cmd: `lf` dispatches to the right
//! shell (nushell by default, brush for bash/POSIX), passes through stdio and
//! the exit code, and appends each run to an audit log.
use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::util;

#[derive(Clone, Copy, PartialEq)]
pub enum ShellKind {
    Nushell,
    Bash,
}

impl ShellKind {
    fn bin(self) -> &'static str {
        match self {
            ShellKind::Nushell => "nu",
            ShellKind::Bash => "brush",
        }
    }
    fn label(self) -> &'static str {
        match self {
            ShellKind::Nushell => "nushell",
            ShellKind::Bash => "brush-bash",
        }
    }
}

/// Resolve which shell binary to use: explicit `--shell`, else fall back
/// nu → brush.
pub fn resolve(shell_opt: Option<&str>) -> Result<(ShellKind, PathBuf)> {
    match shell_opt {
        Some(chosen) => {
            let kind = match chosen {
                "nu" | "nushell" => ShellKind::Nushell,
                "brush" | "bash" => ShellKind::Bash,
                other => bail!("unknown shell '{other}' (options: nu, brush)"),
            };
            shell_path(kind)
        }
        None => {
            for kind in [ShellKind::Nushell, ShellKind::Bash] {
                if let Ok(p) = shell_path(kind) {
                    return Ok(p);
                }
            }
            bail!("no shell installed yet — run `lf install` (installs nushell + brush)")
        }
    }
}

fn shell_path(kind: ShellKind) -> Result<(ShellKind, PathBuf)> {
    let pr = util::probe(kind.bin());
    match pr.path {
        Some(p) => Ok((kind, p)),
        None => bail!(
            "shell '{}' not found. run `lf install {}` first",
            kind.bin(),
            kind.bin()
        ),
    }
}

pub fn run(shell_opt: Option<String>, extra: Vec<String>, no_log: bool) -> Result<i32> {
    let script = assemble_script(&extra);
    let (kind, bin) = resolve(shell_opt.as_deref())?;
    let cwd = std::env::current_dir().ok();

    eprintln!(
        "[lf::exec] shell={} path={} cwd={}",
        kind.label(),
        bin.display(),
        cwd.as_ref()
            .map(|c| c.display().to_string())
            .unwrap_or_else(|| "?".into())
    );

    let start = Instant::now();
    let status = Command::new(&bin)
        .arg("-c")
        .arg(&script)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;
    let dur_ms = start.elapsed().as_millis() as u64;

    let code = status.code().unwrap_or(if status.success() { 0 } else { 1 });

    if !no_log {
        log_execution(&kind, &script, cwd, code, dur_ms);
    }
    Ok(code)
}

/// Combine trailing args into one script string. If none were given, read a
/// script from stdin (so `cat x.nu | lf exec nu` works).
fn assemble_script(extra: &[String]) -> String {
    if extra.is_empty() {
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        buf
    } else {
        extra.join(" ")
    }
}

fn log_execution(
    kind: &ShellKind,
    script: &str,
    cwd: Option<PathBuf>,
    code: i32,
    dur_ms: u64,
) {
    let Ok(root) = util::lf_root() else {
        return;
    };
    let log_file = root.join("exec_log.jsonl");
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let rec = serde_json::json!({
        "ts_ms": ts,
        "shell": kind.label(),
        "cmd": script,
        "cwd": cwd.map(|c| c.display().to_string()),
        "exit": code,
        "dur_ms": dur_ms,
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        use std::io::Write;
        let _ = writeln!(f, "{rec}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_from_args_and_stdin() {
        let args = vec!["echo".to_string(), "hi".to_string()];
        assert_eq!(assemble_script(&args), "echo hi");
        // empty args -> reads stdin (empty here) -> empty string
        assert_eq!(assemble_script(&[]), "");
    }

    #[test]
    fn shell_label() {
        assert_eq!(ShellKind::Nushell.bin(), "nu");
        assert_eq!(ShellKind::Bash.label(), "brush-bash");
    }
}