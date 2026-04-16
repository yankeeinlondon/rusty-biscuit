The Claudine "execution line" (e.g., `Claudine ▸ OpenCode  YOLO   Compose   Op(commit)  prompt sourced from @prompts/commit.md`) appears to be showing up _after_ some work is being done. I suspect that work includes the pre-flight checks. This give the program the feel of being slow.

- The Claudine "execution line" should start immediately after the ENV variable changes (if shown) and other things should follow.
- before _starting_ the pre-flight checks we should log to STDERR with `Status` struct (set INFO and circular) `starting pre-flight checks`
- if any authorizations due to pre-flight checks is necessary this happens next
- when we're done with pre-flight checks (and they've passed) we should again post a Status message to STDERR of `pre-flight checks have passed`
