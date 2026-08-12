//! Feature-gated fixtures for service-listing benchmarks.

use super::{ENRICHMENT_CHUNK, Service, systemd};

/// Precomputed synthetic output for one systemd service-listing iteration.
pub struct SyntheticSystemdListing {
    listing: String,
    enrichment: Vec<String>,
    service_count: usize,
}

impl SyntheticSystemdListing {
    /// Build a deterministic all-running service inventory.
    pub fn new(service_count: usize) -> Self {
        let mut listing = String::new();
        let mut enrichment = Vec::with_capacity(service_count.div_ceil(ENRICHMENT_CHUNK));

        for service in 0..service_count {
            listing.push_str(&format!(
                "service-{service:05}.service loaded active running Synthetic service {service}\n"
            ));
        }
        for start in (0..service_count).step_by(ENRICHMENT_CHUNK) {
            let end = (start + ENRICHMENT_CHUNK).min(service_count);
            let mut output = String::new();
            for service in start..end {
                output.push_str(&format!(
                    "Id=service-{service:05}.service\nMainPID={}\n\n",
                    service + 100
                ));
            }
            enrichment.push(output);
        }

        Self {
            listing,
            enrichment,
            service_count,
        }
    }

    /// Create the small mutable cursor needed by one measured iteration.
    pub fn iteration(&self) -> SyntheticSystemdIteration<'_> {
        SyntheticSystemdIteration {
            fixture: self,
            runner_calls: 0,
            enrichment_calls: 0,
            max_chunk: 0,
        }
    }

    /// Number of services represented by the fixture.
    pub fn service_count(&self) -> usize {
        self.service_count
    }
}

/// Per-iteration runner state for a [`SyntheticSystemdListing`].
pub struct SyntheticSystemdIteration<'a> {
    fixture: &'a SyntheticSystemdListing,
    runner_calls: usize,
    enrichment_calls: usize,
    max_chunk: usize,
}

impl SyntheticSystemdIteration<'_> {
    fn dispatch(&mut self, args: &[&str]) -> Option<String> {
        self.runner_calls += 1;
        if args.first() == Some(&"list-units") {
            return Some(self.fixture.listing.clone());
        }
        if args.first() != Some(&"show") {
            return None;
        }

        let output = self.fixture.enrichment.get(self.enrichment_calls)?.clone();
        self.enrichment_calls += 1;
        self.max_chunk = self.max_chunk.max(args.len().saturating_sub(3));
        Some(output)
    }
}

/// Result and structural work evidence from one synthetic listing iteration.
pub struct SyntheticSystemdResult {
    /// Parsed services after PID projection.
    pub services: Vec<Service>,
    /// Listing plus enrichment runner calls.
    pub runner_calls: usize,
    /// Enrichment chunks dispatched by the production orchestrator.
    pub enrichment_calls: usize,
    /// Largest number of units passed to one enrichment call.
    pub max_chunk: usize,
}

/// Run the production systemd parser, batching, runner dispatch, and projection.
pub fn run_systemd_listing(iteration: &mut SyntheticSystemdIteration<'_>) -> SyntheticSystemdResult {
    let services = systemd::list_systemd_services_with(&mut |args| iteration.dispatch(args));
    SyntheticSystemdResult {
        services,
        runner_calls: iteration.runner_calls,
        enrichment_calls: iteration.enrichment_calls,
        max_chunk: iteration.max_chunk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_service_workloads_preserve_cardinality_and_chunk_bounds() {
        for service_count in [500, 2_000] {
            let fixture = SyntheticSystemdListing::new(service_count);
            let mut iteration = fixture.iteration();
            let result = run_systemd_listing(&mut iteration);
            let enrichment_calls = service_count.div_ceil(ENRICHMENT_CHUNK);

            assert_eq!(fixture.service_count(), service_count);
            assert_eq!(result.services.len(), service_count);
            assert!(result.services.iter().all(|service| service.running));
            assert!(result.services.iter().all(|service| service.pid.is_some()));
            assert_eq!(result.enrichment_calls, enrichment_calls);
            assert_eq!(result.runner_calls, 1 + enrichment_calls);
            assert_eq!(result.max_chunk, ENRICHMENT_CHUNK);
        }
    }
}
