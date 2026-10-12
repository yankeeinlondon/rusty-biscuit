//! `::shell-block` body ranges for the standalone reflow helpers, which treat
//! them as opaque.

use std::ops::Range;

use crate::markdown::compose::block_pairs::{BlockOpenKind, scan_block_pairs};

pub(super) struct OpaqueBodyScan {
    bodies: Vec<Range<usize>>,
    malformed: bool,
}

impl OpaqueBodyScan {
    pub(super) fn scan(content: &str) -> Self {
        let Ok(mut pairs) = scan_block_pairs(content) else {
            return Self {
                bodies: Vec::new(),
                malformed: true,
            };
        };

        pairs.sort_unstable_by_key(|pair| pair.span.start);
        let mut bodies: Vec<Range<usize>> = Vec::new();
        for pair in pairs {
            if pair.kind != BlockOpenKind::Shell
                || bodies.iter().any(|body| {
                    body.start <= pair.span.start && pair.span.end <= body.end
                })
            {
                continue;
            }

            bodies.push(pair.body_span);
        }

        Self {
            bodies,
            malformed: false,
        }
    }

    pub(super) fn protected_ranges(&self, content_len: usize) -> Vec<Range<usize>> {
        if self.malformed {
            std::iter::once(0..content_len).collect()
        } else {
            self.bodies.clone()
        }
    }
}
