//! `lf install` — install nushell and brush cross-platform, user-local
//! (no admin rights needed), with SHA256 verification.
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::util;

pub fn run(targets: &[String], force: bool, update_path: bool) -> Result<()> {
    let names: Vec<&str> = if targets.is_empty() {
        vec!["nu", "brush"]
    } else {
        targets.iter().map(|s| s.as_str()).collect()
    };
    let bin = util::bin_dir()?;
    std::fs::create_dir_all(&bin).context("create ~/.lf/bin")?;

    for name in &names {
        match *name {
            "nu" => install_nushell(&bin, force)?,
            "brush" => install_brush(&bin, force)?,
            other => println!("[skip] unknown install target '{other}' (expected: nu, brush)"),
        }
    }

    if update_path {
        util::add_to_path_if_missing(&bin)?;
    } else {
        println!(
            "[path] PATH update skipped (--no-path); binaries live in {}",
            bin.display()
        );
    }
    Ok(())
}

fn is_installed(name: &str, bin: &Path) -> bool {
    util::find_in_path(name).is_some() || bin.join(util::exe(name)).is_file()
}

fn install_nushell(bin: &Path, force: bool) -> Result<()> {
    let exe = util::exe("nu");
    if !force && is_installed("nu", bin) {
        let where_ = util::find_in_path("nu").unwrap_or_else(|| bin.join(&exe));
        println!("[nu] already installed at {}", where_.display());
        return Ok(());
    }

    let tag = util::github_latest_tag("nushell/nushell")?;
    let (target, ext) = nushell_target()?;
    let asset = format!("nu-{tag}-{target}.{ext}");
    let base = "https://github.com/nushell/nushell/releases/download";
    let url = format!("{base}/{tag}/{asset}");
    println!("[nu] downloading {asset} ...");
    let data = util::download(&url)?;

    let sums = util::download(&format!("{base}/{tag}/SHA256SUMS"))?;
    let expected = util::sum_hash(&sums, &asset).context("asset missing from SHA256SUMS")?;
    let got = util::hex_sha256(&data);
    if !got.eq_ignore_ascii_case(&expected) {
        bail!("SHA256 mismatch for {asset} (expected {expected}, got {got})");
    }
    println!("[nu] checksum OK");

    let out = bin.join(&exe);
    util::extract_binary(&data, &asset, &exe, &out)?;
    #[cfg(unix)]
    util::make_executable(&out)?;

    let ver = util::command_output(&out, "--version").unwrap_or_default();
    println!("[nu] installed {} ({ver})", out.display());
    Ok(())
}

fn install_brush(bin: &Path, force: bool) -> Result<()> {
    let exe = util::exe("brush");
    if !force && is_installed("brush", bin) {
        let where_ = util::find_in_path("brush").unwrap_or_else(|| bin.join(&exe));
        println!("[brush] already installed at {}", where_.display());
        return Ok(());
    }

    #[cfg(windows)]
    {
        install_brush_on_windows()
    }
    #[cfg(not(windows))]
    {
        install_brush_from_release(bin)
    }
}

#[cfg(windows)]
fn install_brush_on_windows() -> Result<()> {
    if util::find_in_path("cargo").is_none() {
        bail!(
            "brush has no official Windows release binary yet (Linux/macOS only).\n\
             Install Rust first (https://rustup.rs) then rerun `lf install brush`,\n\
             which will run `cargo install --locked brush-shell`."
        );
    }
    println!(
        "[brush] building from source via `cargo install --locked brush-shell` (grab a coffee) ..."
    );
    let status = Command::new("cargo")
        .args(["install", "--locked", "brush-shell"])
        .status()
        .context("run cargo install")?;
    if !status.success() {
        bail!("cargo install brush-shell failed");
    }
    match util::find_in_path("brush") {
        Some(p) => {
            let ver = util::command_output(&p, "--version").unwrap_or_default();
            println!("[brush] installed {} ({ver})", p.display());
        }
        None => println!(
            "[brush] built OK — look for `brush` under ~/.cargo/bin and add it to PATH if needed"
        ),
    }
    Ok(())
}

#[cfg(not(windows))]
fn install_brush_from_release(bin: &Path) -> Result<()> {
    let exe = util::exe("brush");
    let tag = util::github_latest_tag("reubeno/brush")?;
    let target = brush_target()?;
    let asset = format!("brush-{target}.tar.gz");
    let base = "https://github.com/reubeno/brush/releases/download";
    let url = format!("{base}/{tag}/{asset}");
    println!("[brush] downloading {asset} ...");
    let data = util::download(&url)?;

    let sums = util::download(&format!("{url}.sha256"));
    if let Ok(sums) = sums {
        if let Some(expected) = util::sum_hash(&sums, &asset) {
            let got = util::hex_sha256(&data);
            if !got.eq_ignore_ascii_case(&expected) {
                bail!("SHA256 mismatch for {asset}");
            }
            println!("[brush] checksum OK");
        }
    }

    let out = bin.join(&exe);
    util::extract_binary(&data, &asset, &exe, &out)?;
    #[cfg(unix)]
    util::make_executable(&out)?;

    let ver = util::command_output(&out, "--version").unwrap_or_default();
    println!("[brush] installed {} ({ver})", out.display());
    Ok(())
}

#[cfg(not(windows))]
fn brush_target() -> Result<&'static str> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        (os, arch) => {
            bail!("no brush release for {os}/{arch}; try `cargo install --locked brush-shell`")
        }
    })
}

fn nushell_target() -> Result<(&'static str, &'static str)> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => ("x86_64-pc-windows-msvc", "zip"),
        ("windows", "aarch64") => ("aarch64-pc-windows-msvc", "zip"),
        ("linux", "x86_64") => ("x86_64-unknown-linux-gnu", "tar.gz"),
        ("linux", "aarch64") => ("aarch64-unknown-linux-gnu", "tar.gz"),
        ("macos", "x86_64") => ("x86_64-apple-darwin", "tar.gz"),
        ("macos", "aarch64") => ("aarch64-apple-darwin", "tar.gz"),
        (os, arch) => bail!("unsupported platform for nushell: {os}/{arch}"),
    })
}