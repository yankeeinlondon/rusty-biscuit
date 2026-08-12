use std::ops::Range;

use crate::markdown::compose::block_pairs::{BlockOpenKind, scan_block_pairs};

pub(super) struct OpaqueBody {
    pub(super) span: Range<usize>,
    pub(super) container_prefix: String,
}

pub(super) struct OpaqueBodyScan {
    bodies: Vec<OpaqueBody>,
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
        let mut bodies: Vec<OpaqueBody> = Vec::new();
        for pair in pairs {
            if pair.kind != BlockOpenKind::Shell
                || bodies.iter().any(|body| {
                    body.span.start <= pair.span.start && pair.span.end <= body.span.end
                })
            {
                continue;
            }

            let prefix_end = pair
                .opening_text
                .find("::shell-block")
                .expect("the shared scanner only classifies shell-block openers");
            bodies.push(OpaqueBody {
                span: pair.body_span,
                container_prefix: pair.opening_text[..prefix_end].to_string(),
            });
        }

        Self {
            bodies,
            malformed: false,
        }
    }

    pub(super) fn bodies(&self) -> &[OpaqueBody] {
        &self.bodies
    }

    pub(super) fn malformed(&self) -> bool {
        self.malformed
    }

    pub(super) fn protected_ranges(&self, content_len: usize) -> Vec<Range<usize>> {
        if self.malformed {
            std::iter::once(0..content_len).collect()
        } else {
            self.bodies.iter().map(|body| body.span.clone()).collect()
        }
    }
}
