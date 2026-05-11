//! Hardware-detection profiling entry point.
//!
//! Runs a full hardware detection pass in a loop so flamegraphs can
//! resolve audio / GPU / storage enumeration costs on the current
//! host.

use sniff::hardware::detect_hardware_with_request;
use sniff::request::HardwareRequest;
use std::env;

fn main() {
    let iterations: u32 = env::var("SNIFF_PROFILE_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let request = HardwareRequest::full();
    println!("profiling detect_hardware_with_request(full) x{iterations}");

    for _ in 0..iterations {
        let info = detect_hardware_with_request(&request).expect("hardware detect failed");
        std::hint::black_box(info);
    }
}
