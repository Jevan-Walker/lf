//! Shared helpers for `lf`.
use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// ~/.lf
pub fn lf_root() -> Result<PathBuf> {
    Ok(home_dir()?.join(".lf"))
}

/// ~/.lf/bin — user-local location for installed shells.
pub fn bin_dir() -> Result<PathBuf> {
    Ok(lf_root()?.join("bin"))
}

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("cannot locate home directory")
}

pub fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Look for a binary on PATH.
pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    let target = exe(name);
    for dir in env::split_paths(&path_var) {
        let cand = dir.join(&target);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// Look for a binary on PATH or in ~/.lf/bin.
pub fn probe(name: &'static str) -> Probe {
    let path = find_in_path(name).or_else(|| {
        let dir = bin_dir().ok()?;
        let p = dir.join(exe(name));
        p.is_file().then_some(p)
    });
    let present = path.is_some();
    let version = path
        .as_ref()
        .and_then(|p| command_output(p, "--version"))
        .map(|v| clean_version(name, &v));
    Probe {
        name,
        path,
        version,
        present,
    }
}

/// Strip a leading program-name token from a `--version` string so we don't
/// print e.g. "brush brush 0.4.0" in tables that already prefix the name.
pub fn clean_version(name: &str, version: &str) -> String {
    let v = version.trim();
    if let Some(rest) = v.strip_prefix(name) {
        if rest.starts_with(' ') {
            return rest.trim().to_string();
        }
    }
    v.to_string()
}

pub struct Probe {
    pub name: &'static str,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub present: bool,
}

pub fn command_output(program: &Path, arg: &str) -> Option<String> {
    let out = Command::new(program).arg(arg).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let data = if out.stdout.is_empty() {
        out.stderr
    } else {
        out.stdout
    };
    let s = String::from_utf8_lossy(&data);
    Some(s.trim().to_string())
}

pub fn download(url: &str) -> Result<Vec<u8>> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(120))
        .timeout(Duration::from_secs(300));
    for var in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(p) = env::var(var) {
            if !p.trim().is_empty() {
                if let Ok(proxy) = ureq::Proxy::new(&p) {
                    builder = builder.proxy(proxy);
                    break;
                }
            }
        }
    }
    let agent = builder.build();
    let resp = agent
        .get(url)
        .set("User-Agent", "lf/0.1.0 (cross-platform shell env setup)")
        .call()
        .map_err(|e| anyhow!("GET {url}: {e}"))?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        bail!("GET {url}: HTTP {status}");
    }
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(512 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .context("reading download")?;
    Ok(bytes)
}

/// Fetch the latest release tag_name for a GitHub repo via the API.
pub fn github_latest_tag(repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let data = download(&url)?;
    let v: serde_json::Value = serde_json::from_slice(&data)?;
    v.get("tag_name")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("no tag_name in GitHub API response for {repo}"))
}

pub fn hex_sha256(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// Find the expected SHA256 hash of `asset` inside a SHA256SUMS file or a
/// `.sha256` sidecar.
pub fn sum_hash(sums: &[u8], asset: &str) -> Option<String> {
    let text = String::from_utf8_lossy(sums);
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        match toks[..] {
            [hash, name] if name == asset => return some_hash(hash),
            [hash] if hash.len() == 64 => return some_hash(hash),
            _ => {}
        }
    }
    None
}

fn some_hash(h: &str) -> Option<String> {
    let s = h.to_ascii_lowercase();
    (s.len() == 64).then_some(s)
}

/// Extract a file named `basename` from a `.zip` or `.tar.gz` archive.
pub fn extract_binary(archive: &[u8], asset: &str, basename: &str, out: &Path) -> Result<()> {
    if asset.ends_with(".zip") {
        extract_zip(archive, basename, out)
    } else if asset.ends_with(".tar.gz") {
        extract_targz(archive, basename, out)
    } else {
        bail!("unsupported archive format: {asset}")
    }
}

fn extract_zip(archive: &[u8], basename: &str, out: &Path) -> Result<()> {
    let mut arc =
        zip::ZipArchive::new(std::io::Cursor::new(archive)).context("open zip")?;
    for i in 0..arc.len() {
        let mut f = arc.by_index(i).context("read zip entry")?;
        if let Some(base) = Path::new(f.name()).file_name() {
            if base.to_string_lossy() == basename {
                let mut dest = File::create(out)
                    .with_context(|| format!("create {}", out.display()))?;
                std::io::copy(&mut f, &mut dest)?;
                return Ok(());
            }
        }
    }
    bail!("{basename} not found in zip archive")
}

fn extract_targz(archive: &[u8], basename: &str, out: &Path) -> Result<()> {
    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(archive));
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries().context("read tar archive")? {
        let mut e = entry.context("tar entry")?;
        if let Some(p) = e.path().ok() {
            if let Some(base) = p.file_name() {
                if base.to_string_lossy() == basename {
                    let mut dest = File::create(out)
                        .with_context(|| format!("create {}", out.display()))?;
                    std::io::copy(&mut e, &mut dest)?;
                    return Ok(());
                }
            }
        }
    }
    bail!("{basename} not found in tar.gz archive")
}

#[cfg(unix)]
pub fn make_executable(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(p)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(p, perms)?;
    Ok(())
}

/// Add a directory to the user PATH (persistent, per-user), unless present.
#[cfg(windows)]
pub fn add_to_path_if_missing(dir: &Path) -> Result<()> {
    let dir_str = dir.to_string_lossy().replace('\'', "''");
    let script = format!(
        r#"$dir='{dir_str}'
$user = [Environment]::GetEnvironmentVariable('Path','User')
$list = @($user -split ';' | Where-Object {{ $_ -ne '' }})
if ($list -notcontains $dir) {{
  $new = (($list + $dir) -join ';')
  [Environment]::SetEnvironmentVariable('Path', $new, 'User')
  Write-Output "[path] added {dir_str} to user PATH (open a new terminal)"
}} else {{
  Write-Output "[path] {dir_str} already on user PATH"
}}
"#
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .context("spawn powershell")?;
    if !status.success() {
        bail!("failed to update user PATH via PowerShell");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn add_to_path_if_missing(dir: &Path) -> Result<()> {
    let line = format!("export PATH=\"{}:$PATH\"  # lf", dir.display());
    let rc = home_dir()?.join(".bashrc");
    let content = std::fs::read_to_string(&rc).unwrap_or_default();
    if content.lines().any(|l| l.trim() == line.trim()) {
        println!("[path] {} already on PATH ({})", dir.display(), rc.display());
        return Ok(());
    }
    let mut new = content;
    if !new.ends_with('\n') {
        new.push('\n');
    }
    new.push_str(&line);
    new.push('\n');
    std::fs::write(&rc, new).context("append to .bashrc")?;
    println!(
        "[path] appended to {} (restart shell or `source ~/.bashrc`)",
        rc.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targz_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let enc =
                flate2::write::GzEncoder::new(&mut out, flate2::Compression::default());
            let mut ar = tar::Builder::new(enc);
            for (name, data) in entries {
                let mut hdr = tar::Header::new_gnu();
                hdr.set_size(data.len() as u64);
                hdr.set_mode(0o755);
                hdr.set_cksum();
                ar.append_data(&mut hdr, name, *data).unwrap();
            }
            ar.finish().unwrap();
        }
        out
    }

    #[test]
    fn extract_binary_from_targz() {
        let bytes = targz_with(&[
            ("nu-0.115.1-x86_64/nu", b"hello-nu"),
            ("other-file", b"ignored"),
        ]);
        let dir = std::env::temp_dir().join("lf_test_extract");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("nu");
        extract_binary(&bytes, "release.tar.gz", "nu", &out).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"hello-nu");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sum_hash_accepts_sums_and_sidecar() {
        let h64 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let sums = format!("{h64}  nu-0.115.1-x86_64.zip\n0f0f nu-other\n");
        let got = sum_hash(sums.as_bytes(), "nu-0.115.1-x86_64.zip").unwrap();
        assert_eq!(got, h64);
        // bare sidecar line (just the hash)
        let single = format!("{h64}\n");
        assert_eq!(
            sum_hash(single.as_bytes(), "whatever").unwrap(),
            h64
        );
        // unmatched asset -> error
        assert!(sum_hash(sums.as_bytes(), "missing.zip").is_none());
    }

    #[test]
    fn clean_version_strips_name_prefix() {
        assert_eq!(
            clean_version("brush", "brush 0.4.0 (cargo:0.4.0)"),
            "0.4.0 (cargo:0.4.0)"
        );
        assert_eq!(clean_version("nu", "0.115.1"), "0.115.1");
        assert_eq!(
            clean_version("git", "git version 2.55.0"),
            "version 2.55.0"
        );
    }
}