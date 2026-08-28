//! `lf init` — generate cross-platform shell config (nushell env/config,
//! brush/bash rc) with agent-friendly aliases and PATH, as the base layer
//! for driving tools like ffmpeg from any host.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util;

pub fn run(apply: bool) -> Result<()> {
    let root = util::lf_root()?;
    std::fs::create_dir_all(&root).context("create ~/.lf")?;

    let bin = util::bin_dir()?;
    let cargo_bin = home_cargo_bin();

    let env_nu_path = root.join("env.nu");
    std::fs::write(&env_nu_path, env_nu_content(&bin, &cargo_bin)).context("write env.nu")?;
    println!("[init] wrote {}", env_nu_path.display());

    let brush_rc = root.join("brush.rc");
    std::fs::write(&brush_rc, brush_rc_content(&bin, &cargo_bin)).context("write brush.rc")?;
    println!("[init] wrote {}", brush_rc.display());

    if apply {
        patch_nushell(&env_nu_path)?;
        patch_bashrc(&brush_rc)?;
        println!("[init] applied. Restart your shell (new terminal) to load configs.");
    } else {
        println!("[init] dry-run done. Re-run with `--apply` to wire them into your shell.");
    }
    Ok(())
}

fn env_nu_content(bin: &Path, cargo_bin: &Path) -> String {
    let bin = bin.display();
    let cargo_bin = cargo_bin.display();
    format!(
        r#"# lf-generated nushell env (editable) - cross-platform agent shell base
# keep locally-installed toolchains + lf bin at the front of PATH
$env.PATH = ($env.PATH | prepend [ "{bin}", "{cargo_bin}" ] | uniq)

# deterministic, parse-friendly default for agents
$env.LF = "1"
$env.LF_SHELL = "nushell"

# ---- tool aliases (foundation for AI driving ffmpeg etc.) ----
alias ll = ls -la
alias lh = ls --human-readable

# ffmpeg convenience wrappers (edit to taste)
export def "ff to-mp4" [input: string, output: string] {{
  ffmpeg -y -i $input -c:v libx264 -c:a aac $output
}}
export def "ff to-gif" [input: string, output: string, fps: int = 15] {{
  ffmpeg -y -i $input -vf $"fps=($fps)" -loop 0 $output
}}
export def "ff probe" [file: string] {{
  ffprobe -v error -show_format -show_streams $file
}}
export def "ff frames" [input: string, output: string, fps: int = 30] {{
  ffmpeg -y -i $input -vf $"fps=($fps)" "{{output}}/frame_%04d.png"
}}
"#
    )
}

fn brush_rc_content(bin: &Path, cargo_bin: &Path) -> String {
    let bin = bin.display();
    let cargo_bin = cargo_bin.display();
    format!(
        r#"# lf-generated for brush / bash (editable) - cross-platform shell base
export PATH="{bin}:{cargo_bin}:$PATH"
export LF_SHELL=brush

alias ll='ls -la'

# ffmpeg helpers (POSIX/bash)
ff-to-mp4() {{ ffmpeg -y -i "$1" -c:v libx264 -c:a aac "$2"; }}
ff-probe()  {{ ffprobe -v error -show_format -show_streams "$1"; }}
"#
    )
}

/// `~/.cargo/bin` (where cargo installs binaries like brush).
fn home_cargo_bin() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".cargo").join("bin"))
        .unwrap_or_else(|| PathBuf::from("~/.cargo/bin"))
}

/// Locate the nushell env file so we can `source` our env.nu into it.
/// (Calls `nu -c 'print $nu.env-path'` for a cross-platform answer.)
fn nu_env_path() -> Option<PathBuf> {
    let nu = util::find_in_path("nu")?;
    let out = Command::new(&nu)
        .args(["-c", "print $nu.env-path"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s.starts_with("Error") {
        return None;
    }
    Some(PathBuf::from(s))
}

fn patch_nushell(env_nu: &Path) -> Result<()> {
    let Some(target) = nu_env_path() else {
        println!("[init] nushell not available - run `lf install nu` then `lf init --apply` again");
        return Ok(());
    };
    let line = format!("source {}", env_nu.display());
    patch_file(&target, &line, "nushell env.nu")
}

fn patch_bashrc(brush_rc: &Path) -> Result<()> {
    let rc = util::home_dir()?.join(".bashrc");
    let line = format!(
        "[ -f {} ] && source {}",
        brush_rc.display(),
        brush_rc.display()
    );
    patch_file(&rc, &line, ".bashrc")
}

/// Append `line` to `file` once (backing up first), unless already present.
fn patch_file(file: &Path, line: &str, label: &str) -> Result<()> {
    if let Ok(content) = std::fs::read_to_string(file) {
        if content.lines().any(|l| l.trim() == line.trim()) {
            println!("[init] {label} already wired ({})", file.display());
            return Ok(());
        }
    }
    // back up before modifying a user config file
    if file.exists() {
        let bak = file.with_extension("lf-bak");
        std::fs::copy(file, &bak).ok();
    }
    let mut content = std::fs::read_to_string(file).unwrap_or_default();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(line);
    content.push('\n');
    std::fs::write(file, content)
        .with_context(|| format!("write {label} at {}", file.display()))?;
    println!("[init] {label} wired -> {line}");
    Ok(())
}
