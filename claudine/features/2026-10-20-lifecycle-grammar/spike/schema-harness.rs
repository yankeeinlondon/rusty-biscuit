//! Throwaway spike harness: resolve a schema file, report the compiled JSON
//! Schema size, and time in-process resolution + validation (DMLS proxy).
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::resolve::resolve_schema;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");
    match mode {
        // size <schema-file>: resolve `$schema: <file>` from its own dir and print JSON size
        "size" => {
            let file = PathBuf::from(&args[2]);
            let base = file.parent().unwrap();
            // A `kind: schema` library exports nothing; wire the seven keys the trigger payload would.
            let name = file.file_name().unwrap().to_string_lossy().to_string();
            let mut map = serde_json::Map::new();
            for ev in ["initialize", "start", "blocked", "success", "failure", "finalize"] {
                map.insert(ev.into(), serde_json::Value::String(format!("stack@./{name}")));
            }
            map.insert("loop".into(), serde_json::Value::String(format!("loop@./{name}")));
            let value = serde_json::Value::Object(map);
            let t = Instant::now();
            let resolved = resolve_schema(&value, base);
            let elapsed = t.elapsed();
            match resolved {
                Ok(r) => {
                    let json = serde_json::to_string(&r.json_schema).unwrap();
                    println!("{}\tbytes={}\tresolve_ms={:.1}", file.display(), json.len(), elapsed.as_secs_f64() * 1000.0);
                    if let Some(out) = args.get(3) { std::fs::write(out, &json).unwrap(); }
                }
                Err(e) => println!("{}\tERROR: {e}", file.display()),
            }
        }
        // validate <doc.md> <boundary> [iterations]: trigger discovery + effective schema + validate
        "validate" => {
            let doc = PathBuf::from(&args[2]);
            let boundary = PathBuf::from(&args[3]);
            let iters: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
            let md = Markdown::try_from(doc.as_path()).unwrap();
            let mut times = Vec::new();
            for i in 0..iters {
                let t = Instant::now();
                let api = DarkmatterSchemas::new().with_trigger_discovery(&doc, &boundary).unwrap();
                let eff = api.effective_for(&md).unwrap();
                let t_eff = t.elapsed();
                let mut n_problems = 0;
                if let Some(eff) = eff {
                    let fm = frontmatter_json(&md);
                    let report = eff.validate(&fm);
                    n_problems = report.problems.len();
                    if i == 0 {
                        for p in report.problems.iter().take(6) { println!("  problem: {}", p.message); }
                    }
                }
                let total = t.elapsed();
                times.push((t_eff.as_secs_f64() * 1000.0, total.as_secs_f64() * 1000.0));
                if i == 0 { println!("{}: problems={} first_effective_ms={:.1} first_total_ms={:.1}", doc.display(), n_problems, times[0].0, times[0].1); }
            }
            let warm: Vec<f64> = times.iter().skip(1).map(|t| t.1).collect();
            if !warm.is_empty() {
                let mut w = warm.clone(); w.sort_by(|a, b| a.partial_cmp(b).unwrap());
                println!("  warm total_ms median={:.2} (n={})", w[w.len()/2], w.len());
            }
        }
        _ => eprintln!("usage: size <schema.yaml> [out.json] | validate <doc.md> <boundary> [iters]"),
    }
}

fn frontmatter_json(md: &Markdown) -> serde_json::Value {
    serde_json::Value::Object(md.frontmatter().as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}
