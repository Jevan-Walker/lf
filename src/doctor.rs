//! `lf doctor` — check the cross-platform shell environment is ready.
use anyhow::Result;
use serde_json::{json, Map, Value};

use crate::util;

pub fn run(json_out: bool) -> Result<()> {
    let required = ["nu", "brush"];
    let optional = ["ffmpeg", "git", "cargo", "node"];

    let req: Vec<util::Probe> = required.iter().map(|n| util::probe(n)).collect();
    let opt: Vec<util::Probe> = optional.iter().map(|n| util::probe(n)).collect();
    let all = req.iter().chain(opt.iter());

    let missing: Vec<&str> = req.iter().filter(|p| !p.present).map(|p| p.name).collect();

    if json_out {
        let mut obj = Map::new();
        for p in all.clone() {
            obj.insert(
                p.name.to_string(),
                json!({
                    "present": p.present,
                    "version": p.version,
                    "path": p.path.as_ref().map(|x| x.display().to_string()),
                }),
            );
        }
        obj.insert("ready".to_string(), json!(missing.is_empty()));
        println!("{}", serde_json::to_string_pretty(&Value::Object(obj))?);
    } else {
        println!("{:<10} {:<8} {}", "tool", "status", "version / path");
        for p in all.clone() {
            if p.present {
                let v = p
                    .version
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("ok");
                let path = p
                    .path
                    .as_ref()
                    .map(|x| x.display().to_string())
                    .unwrap_or_default();
                println!("{:<10} ok      {v}  {path}", p.name);
            } else if required.contains(&p.name) {
                println!("{:<10} MISSING", p.name);
            } else {
                println!("{:<10} -", p.name);
            }
        }
    }

    if !missing.is_empty() {
        println!();
        println!("[doctor] required shells missing: {}", missing.join(", "));
        println!("[doctor] to fix: run  `lf install`");
        std::process::exit(1);
    }
    println!();
    println!("[doctor] OK — cross-platform shell environment ready (nushell + brush)");
    std::process::exit(0);
}