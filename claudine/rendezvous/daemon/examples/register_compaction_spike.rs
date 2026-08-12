//! Register history compaction spike (host-capability-broadcast S1 /
//! rendezvous data-model open question "register history compaction").
//!
//! Empirically answers three questions about Loro "state register"
//! documents (single-writer maps whose values are overwritten in place):
//!
//! 1. How fast does a register document grow with write count, and how
//!    does key churn shape (one hot key vs many keys) affect it?
//! 2. How much does shallow-snapshot re-basing reclaim, what does it
//!    cost, and can the owner keep writing (same peer id) afterwards?
//! 3. Which reader/sync scenarios survive a re-base, and which need a
//!    recovery path?
//!
//! Run from the repo root:
//!
//! ```sh
//! cargo run -p rendezvous-daemon --example register_compaction_spike
//! ```
//!
//! Findings are written up in
//! `claudine/features/2026-07-11-host-capability-broadcast/spike-register-compaction.md`.

use std::time::Instant;

use loro::{ExportMode, LoroDoc};

const OWNER_PEER: u64 = 1;
const REPO_COUNT: u64 = 20;

fn main() {
    println!("== register history compaction spike (loro 1.12.0, single-writer registers) ==");
    experiment_a_growth();
    experiment_b_rebase();
    experiment_c_sync_safety();
    println!("\ndone.");
}

// --- helpers -----------------------------------------------------------

fn owner_doc() -> LoroDoc {
    let doc = LoroDoc::new();
    doc.set_peer_id(OWNER_PEER).expect("set owner peer id");
    doc
}

/// Deterministic 40-hex-char pseudo commit hash (splitmix64-based) so the
/// workload is reproducible without a rand dependency.
fn pseudo_hash(seed: u64) -> String {
    let mut x = seed;
    let mut out = String::with_capacity(40);
    for _ in 0..5 {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        out.push_str(&format!("{:08x}", (z >> 32) as u32));
    }
    out
}

/// One `repos`-register write: a commit lands in one of REPO_COUNT repos,
/// overwriting that repo's head-commit hash. One commit per write, matching
/// how the daemon would refresh on change.
fn write_repo_tick(doc: &LoroDoc, tick: u64) {
    let map = doc.get_map("repos");
    let repo = format!("github.com/acme/repo-{:02}", tick % REPO_COUNT);
    map.insert(&repo, pseudo_hash(tick).as_str())
        .expect("insert repo head");
    doc.commit();
}

/// One `capability`-register volatile write: the same single key
/// (`available_storage`) overwritten every tick — the worst-case shape the
/// data-model doc warns about when a volatile field is not quantized.
fn write_capability_tick(doc: &LoroDoc, tick: u64) {
    let map = doc.get_map("capability");
    let gb = 500_000i64 - (tick as i64 % 1_000);
    map.insert("available_storage", gb).expect("insert storage");
    doc.commit();
}

/// Populate the ~30 cold hardware fields once, like a first detection pass.
fn seed_capability_fields(doc: &LoroDoc) {
    let map = doc.get_map("capability");
    map.insert("os", "macOS").unwrap();
    map.insert("os_version", "26.5.0").unwrap();
    map.insert("memory", 65_536i64).unwrap();
    map.insert("cpu_cores", 16i64).unwrap();
    map.insert("gpu", "metal").unwrap();
    map.insert("machine", "bare-metal").unwrap();
    map.insert("arch", "arm64").unwrap();
    for flag in [
        "avx", "avx2", "avx512bw", "avx512f", "avx512vl", "neon", "sse", "sse2", "sse3", "sse4_1",
        "sse4_2", "ssse3",
    ] {
        map.insert(flag, flag == "neon").unwrap();
    }
    map.insert("id", "node-aaaa-bbbb").unwrap();
    map.insert("name", "kens-macbook").unwrap();
    doc.commit();
}

fn export_len(doc: &LoroDoc, mode: ExportMode) -> usize {
    doc.export(mode).expect("export").len()
}

fn kib(bytes: usize) -> String {
    format!("{:.1}", bytes as f64 / 1024.0)
}

// --- experiment A: growth curves ---------------------------------------

fn experiment_a_growth() {
    println!("\n-- A. document growth vs write count --");
    println!("   (full = ExportMode::Snapshot; shallow = ShallowSnapshot at current frontier;");
    println!("    state = StateOnly. sizes in KiB, one commit per write)");
    let checkpoints = [100u64, 1_000, 5_000, 10_000, 25_000, 50_000];

    type WriteFn = fn(&LoroDoc, u64);
    let shapes: [(&str, WriteFn); 2] = [
        ("repos (20 keys, rotating)", write_repo_tick),
        ("capability (1 hot key)", write_capability_tick),
    ];
    for (label, write) in shapes {
        println!("\n   register shape: {label}");
        println!(
            "   {:>8} {:>8} {:>10} {:>10} {:>10} {:>12}",
            "writes", "ops", "full KiB", "shallow", "state", "B/write"
        );
        let doc = owner_doc();
        if label.starts_with("capability") {
            seed_capability_fields(&doc);
        }
        let mut tick = 0u64;
        let mut prev = (0u64, 0usize);
        for &cp in &checkpoints {
            while tick < cp {
                write(&doc, tick);
                tick += 1;
            }
            let frontier = doc.oplog_frontiers();
            let full = export_len(&doc, ExportMode::Snapshot);
            let shallow = export_len(&doc, ExportMode::shallow_snapshot(&frontier));
            let state = export_len(&doc, ExportMode::state_only(None));
            let marginal = if cp > prev.0 {
                (full.saturating_sub(prev.1)) as f64 / (cp - prev.0) as f64
            } else {
                0.0
            };
            println!(
                "   {:>8} {:>8} {:>10} {:>10} {:>10} {:>12.1}",
                cp,
                doc.len_ops(),
                kib(full),
                kib(shallow),
                kib(state),
                marginal
            );
            prev = (cp, full);
        }
    }
}

// --- experiment B: re-base mechanics ------------------------------------

fn experiment_b_rebase() {
    println!("\n-- B. shallow-snapshot re-base mechanics (repos register @ 10k writes) --");
    let doc = owner_doc();
    for tick in 0..10_000 {
        write_repo_tick(&doc, tick);
    }

    let t = Instant::now();
    let full = doc.export(ExportMode::Snapshot).expect("full export");
    let full_ms = t.elapsed().as_millis();

    let frontier = doc.oplog_frontiers();
    let t = Instant::now();
    let shallow = doc
        .export(ExportMode::shallow_snapshot(&frontier))
        .expect("shallow export");
    let shallow_ms = t.elapsed().as_millis();

    println!(
        "   full snapshot: {} KiB in {full_ms}ms; shallow: {} KiB in {shallow_ms}ms ({}x smaller)",
        kib(full.len()),
        kib(shallow.len()),
        full.len() / shallow.len().max(1)
    );
    println!("   (debug-profile timings — indicative only)");

    // Re-base: adopt the shallow snapshot as a fresh doc, as the daemon
    // would when swapping its in-memory doc + persisted redb snapshot.
    let rebased = LoroDoc::new();
    let t = Instant::now();
    rebased.import(&shallow).expect("import shallow");
    println!(
        "   import shallow into fresh doc: {}ms; is_shallow={}, shallow_since_vv={:?}",
        t.elapsed().as_millis(),
        rebased.is_shallow(),
        rebased.shallow_since_vv()
    );
    println!(
        "   state preserved across re-base: {}",
        doc.get_deep_value() == rebased.get_deep_value()
    );

    // Can the owner keep writing as the SAME peer after re-basing?
    match rebased.set_peer_id(OWNER_PEER) {
        Ok(()) => println!("   set_peer_id(owner) on re-based doc: OK"),
        Err(e) => println!("   set_peer_id(owner) on re-based doc: ERR {e}"),
    }
    for tick in 10_000..10_100 {
        write_repo_tick(&rebased, tick);
    }
    println!(
        "   +100 writes post-re-base: snapshot {} KiB (was {} KiB pre-re-base)",
        kib(export_len(&rebased, ExportMode::Snapshot)),
        kib(full.len())
    );

    // redb replay path: the daemon persists snapshot bytes and reloads via
    // LoroDoc::from_snapshot. Does that path accept a shallow doc's bytes?
    match rebased.export(ExportMode::Snapshot) {
        Ok(bytes) => match LoroDoc::from_snapshot(&bytes) {
            Ok(reloaded) => println!(
                "   persist/reload shallow doc via Snapshot export + from_snapshot: OK (is_shallow={})",
                reloaded.is_shallow()
            ),
            Err(e) => println!("   from_snapshot(shallow doc bytes): ERR {e}"),
        },
        Err(e) => println!("   Snapshot export of shallow doc: ERR {e}"),
    }
}

// --- experiment C: sync-safety matrix ------------------------------------

fn experiment_c_sync_safety() {
    println!("\n-- C. sync safety around a re-base --");
    println!("   timeline: writes 0..500 (stale reader syncs) .. 500..1000 (head reader");
    println!("   syncs) -> owner re-bases at 1000 -> writes 1000..1100");

    // Owner history 0..1000, capturing reader states along the way.
    let owner = owner_doc();
    for tick in 0..500 {
        write_repo_tick(&owner, tick);
    }
    let stale_reader = LoroDoc::new();
    stale_reader
        .import(&owner.export(ExportMode::Snapshot).unwrap())
        .expect("stale reader sync");
    let stale_vv = stale_reader.oplog_vv();
    // Pre-re-base delta payload for C4 (ops the shallow root already covers).
    let old_delta = owner.export(ExportMode::all_updates()).unwrap();

    for tick in 500..1_000 {
        write_repo_tick(&owner, tick);
    }
    let head_reader = LoroDoc::new();
    head_reader
        .import(&owner.export(ExportMode::Snapshot).unwrap())
        .expect("head reader sync");
    let head_vv = head_reader.oplog_vv();

    // Owner re-bases at 1000 and keeps writing to 1100.
    let frontier = owner.oplog_frontiers();
    let rebased = LoroDoc::new();
    rebased
        .import(
            &owner
                .export(ExportMode::shallow_snapshot(&frontier))
                .unwrap(),
        )
        .expect("owner re-base");
    rebased.set_peer_id(OWNER_PEER).expect("owner peer id");
    for tick in 1_000..1_100 {
        write_repo_tick(&rebased, tick);
    }

    // C1 — reader exactly at the re-base point asks for updates-since.
    match rebased.export(ExportMode::updates(&head_vv)) {
        Ok(delta) => {
            let status = head_reader.import(&delta);
            println!(
                "   C1 head reader (vv == re-base point) updates-since: export OK ({} B), import {:?}, converged={}",
                delta.len(),
                status.map(|s| format!("success={:?} pending={:?}", s.success, s.pending)),
                head_reader.get_deep_value() == rebased.get_deep_value()
            );
        }
        Err(e) => println!("   C1 head reader updates-since: export ERR {e}"),
    }

    // C2a — reader BEHIND the re-base point asks for updates-since.
    match rebased.export(ExportMode::updates(&stale_vv)) {
        Ok(delta) => {
            let status = stale_reader.import(&delta);
            println!(
                "   C2a stale reader (vv < re-base point) updates-since: export OK ({} B), import {:?}, converged={}",
                delta.len(),
                status.map(|s| format!("success={:?} pending={:?}", s.success, s.pending)),
                stale_reader.get_deep_value() == rebased.get_deep_value()
            );
        }
        Err(e) => println!("   C2a stale reader updates-since: export ERR {e}"),
    }

    // C2b — recovery attempt: send the stale reader a current shallow snapshot.
    let recovery_frontier = rebased.oplog_frontiers();
    let recovery = rebased
        .export(ExportMode::shallow_snapshot(&recovery_frontier))
        .expect("recovery shallow export");
    let status = stale_reader.import(&recovery);
    println!(
        "   C2b stale reader imports current shallow snapshot: {:?}, converged={}, reader is_shallow={}",
        status.map(|s| format!("success={:?} pending={:?}", s.success, s.pending)),
        stale_reader.get_deep_value() == rebased.get_deep_value(),
        stale_reader.is_shallow()
    );

    // C2c — fallback recovery: stale reader discards its replica entirely.
    let fresh_recovery = LoroDoc::new();
    let status = fresh_recovery.import(&recovery);
    println!(
        "   C2c stale reader discards + re-adopts shallow snapshot: {:?}, converged={}",
        status.map(|s| format!("success={:?} pending={:?}", s.success, s.pending)),
        fresh_recovery.get_deep_value() == rebased.get_deep_value()
    );

    // C3 — brand-new reader bootstraps from shallow snapshot + follows deltas.
    let fresh = LoroDoc::new();
    fresh.import(&recovery).expect("fresh bootstrap");
    let fresh_vv = fresh.oplog_vv();
    for tick in 1_100..1_150 {
        write_repo_tick(&rebased, tick);
    }
    let delta = rebased
        .export(ExportMode::updates(&fresh_vv))
        .expect("delta");
    fresh.import(&delta).expect("fresh delta import");
    println!(
        "   C3 fresh reader: shallow bootstrap + later delta: converged={}",
        fresh.get_deep_value() == rebased.get_deep_value()
    );

    // C4 — an out-of-order OLD delta (pre-re-base ops) reaches a shallow reader.
    let status = fresh.import(&old_delta);
    println!(
        "   C4 shallow reader receives pre-re-base delta: {:?}, still converged={}",
        status.map(|s| format!("success={:?} pending={:?}", s.success, s.pending)),
        fresh.get_deep_value() == rebased.get_deep_value()
    );
}
