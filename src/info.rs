//! `lf info` — fastfetch-like system information, also as agent-readable JSON.
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::path::Path;
use sysinfo::{Disks, System};

use crate::util;

pub fn run(json_out: bool) -> Result<()> {
    let info = os_info::get();
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = info.os_type().to_string();
    let os_version = info.version().to_string();
    let os_edition = info.edition().map(|s| s.to_string());
    let kernel = System::kernel_version().unwrap_or_default();
    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let user = whoami::username().unwrap_or_else(|_| "unknown".to_string());
    let arch = std::env::consts::ARCH.to_string();

    let cpu = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let cores = sys.cpus().len();
    let physical = sys.physical_core_count().unwrap_or(cores);

    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();

    let (disk_total, disk_free) = root_disk().unwrap_or((0, 0));
    let disk_used = disk_total.saturating_sub(disk_free);

    let nu = util::probe("nu");
    let brush = util::probe("brush");
    let git = util::probe("git");
    let ffmpeg = util::probe("ffmpeg");
    let cargo = util::probe("cargo");

    if json_out {
        let out = json!({
            "os": {
                "name": os_name,
                "version": os_version,
                "edition": os_edition,
                "arch": arch
            },
            "kernel": kernel,
            "host": hostname,
            "user": user,
            "cpu": { "model": cpu, "threads": cores, "physical_cores": physical },
            "memory": {
                "total_bytes": mem_total,
                "used_bytes": mem_used,
                "total_gib": gib(mem_total),
                "used_gib": gib(mem_used)
            },
            "disk": {
                "total_bytes": disk_total,
                "used_bytes": disk_used,
                "free_bytes": disk_free,
                "total_gib": gib(disk_total),
                "used_gib": gib(disk_used)
            },
            "shells": probes_obj(&[&nu, &brush]),
            "tools": probes_obj(&[&git, &ffmpeg, &cargo]),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!(
        "{:>8}: {}",
        "OS",
        fmt_os(&os_name, &os_version, os_edition.as_deref())
    );
    println!("{:>8}: {}", "Kernel", kernel);
    println!("{:>8}: {}", "Host", hostname);
    println!("{:>8}: {}", "User", user);
    println!("{:>8}: {}", "Arch", arch);
    println!(
        "{:>8}: {}",
        "CPU",
        format!("{} ({} threads / {physical} physical)", cpu, cores)
    );
    println!(
        "{:>8}: {} / {}",
        "RAM",
        gib(mem_used),
        gib(mem_total)
    );
    println!(
        "{:>8}: {} used / {} total",
        "Disk",
        gib(disk_used),
        gib(disk_total)
    );
    println!("{:>8}: {}", "Shells", fmt_probe_line(&nu));
    println!("{:>8}  {}", "", fmt_probe_line(&brush));
    println!("{:>8}: {}", "Tools", fmt_tools(&[&git, &ffmpeg, &cargo]));
    Ok(())
}

fn fmt_os(name: &str, version: &str, edition: Option<&str>) -> String {
    let mut s = format!("{name} {version}");
    if let Some(e) = edition {
        if !e.is_empty() {
            s.push_str(&format!(" ({e})"));
        }
    }
    s.trim().to_string()
}

fn fmt_probe_line(p: &util::Probe) -> String {
    if p.present {
        let v = p.version.as_deref().unwrap_or("?");
        let path = p
            .path
            .as_ref()
            .map(|x| x.display().to_string())
            .unwrap_or_default();
        format!("{} {v}  {}", p.name, path)
    } else {
        format!("{} MISSING (run: lf install)", p.name)
    }
}

fn fmt_tools(ps: &[&util::Probe]) -> String {
    ps.iter()
        .map(|p| {
            let st = if p.present {
                p.version.as_deref().unwrap_or("ok").to_string()
            } else {
                "missing".to_string()
            };
            format!("{} {st}", p.name)
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn probes_obj(ps: &[&util::Probe]) -> Value {
    let mut m = Map::new();
    for p in ps {
        m.insert(
            p.name.to_string(),
            json!({
                "present": p.present,
                "version": p.version,
                "path": p.path.as_ref().map(|x| x.display().to_string()),
            }),
        );
    }
    Value::Object(m)
}

fn gib(bytes: u64) -> String {
    format!("{:.1} GiB", bytes as f64 / 1073741824.0)
}

fn root_disk() -> Option<(u64, u64)> {
    let disks = Disks::new_with_refreshed_list();
    let list = disks.list();
    if list.is_empty() {
        return None;
    }
    let pick = if cfg!(windows) {
        let cur = std::env::current_dir()
            .ok()
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_default();
        let drive = cur
            .chars()
            .next()
            .map(|c| format!("{}:\\", c).to_uppercase())
            .unwrap_or_default();
        list.iter()
            .find(|d| {
                d.mount_point()
                    .to_string_lossy()
                    .to_uppercase()
                    .starts_with(&drive)
            })
            .or_else(|| list.first())
    } else {
        list.iter()
            .find(|d| d.mount_point() == Path::new("/"))
            .or_else(|| list.first())
    };
    pick.map(|d| (d.total_space(), d.available_space()))
}