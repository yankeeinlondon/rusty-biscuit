//! X2 of the `2026-10-25-pipeline-work-reduction` spike: completion walks the
//! tree once per file alternative. Measured at one large size only, as wall
//! time against the alternative count (each alternative is one walk of the
//! shared root; see `match_alternative_files`). Run explicitly:
//!
//! ```text
//! cargo test -p claudine-cli --release --bin claudine -- --ignored --nocapture \
//!     --test-threads=1 x2_spike
//! ```

use std::time::{Duration, Instant};

use super::*;

/// The median of `runs` timings of `scenario`.
fn median(runs: usize, mut scenario: impl FnMut() -> usize) -> (Duration, usize) {
    let mut times = Vec::with_capacity(runs);
    let mut found = 0;
    for _ in 0..runs {
        let started = Instant::now();
        found = std::hint::black_box(scenario());
        times.push(started.elapsed());
    }
    times.sort();
    (times[runs / 2], found)
}

#[test]
#[ignore = "measurement spike; run explicitly"]
fn x2_spike_alternative_walks() {
    // 20 directories of 500 Markdown files and 100 other files: 12,000 files.
    let tmp = TempDir::new().unwrap();
    seed_repo(tmp.path());
    for directory in 0..20 {
        let dir = tmp.path().join(format!("area-{directory}"));
        fs::create_dir_all(&dir).unwrap();
        for file in 0..500 {
            fs::write(dir.join(format!("guide-{file}.md")), "# guide\n").unwrap();
        }
        for file in 0..100 {
            fs::write(dir.join(format!("data-{file}.json")), "{}\n").unwrap();
        }
    }
    let ctx = ScopeContext::discover_from(tmp.path());
    let patterns = [
        "**/*.md",
        "**/guide-1*.md",
        "**/guide-2*.md",
        "**/guide-3*.md",
        "**/guide-4*.md",
        "**/guide-5*.md",
        "**/guide-6*.md",
        "**/guide-7*.md",
    ];
    for count in [1usize, 2, 4, 8] {
        let alternatives: Vec<Vec<Vec<String>>> = patterns[..count]
            .iter()
            .map(|pattern| vec![vec![(*pattern).to_string()]])
            .collect();
        let (time, found) = median(7, || file_candidate_paths(&alternatives, &ctx).len());
        println!(
            "SPIKE item=X2 size=files=12000/alternatives={count} time_ns={} walks={count} found={found}",
            time.as_nanos()
        );
    }
}
