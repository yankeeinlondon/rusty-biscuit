//! Full library-detection entry point for flamegraph profiling.
//!
//! Runs the library's default `DetectionPlan` against the current
//! working directory in a tight loop so `cargo flamegraph` has stable
//! stacks to sample. Network is pinned to `interfaces_only` to keep
//! wall-clock time off external HTTP latency.

use sniff::DetectionPlan;
use sniff::detect_with_plan;
use sniff::request::NetworkRequest;
use std::env;

fn main() {
    let iterations: u32 = env::var("SNIFF_PROFILE_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    println!("profiling detect_with_plan (full) x{iterations}");

    for i in 0..iterations {
        let plan = DetectionPlan::new().network(NetworkRequest::interfaces_only());
        let result = detect_with_plan(plan).expect("detect failed");
        if i == 0 {
            println!(
                "first run: os={}, hw={}, net={}, fs={}",
                result.os.is_some(),
                result.hardware.is_some(),
                result.network.is_some(),
                result.filesystem.is_some(),
            );
        }
    }
}
