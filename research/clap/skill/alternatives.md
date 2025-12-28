# When to use Clap vs. Alternatives

### Use `clap` when:

* You need a professional CLI with standard GNU/POSIX behavior.
* You want automatic, high-quality `--help` and shell completions.
* You have complex nested subcommands or complex validation rules.

### Consider alternatives when:

|Library|Use Case|Key Benefit|
|:------|:-------|:----------|
|**bpaf**|High performance / complex logic|Faster compile times, very flexible combinators.|
|**argh**|Simple, small binaries|Tiny footprint, Google-style opinionated CLI.|
|**pico-args**|Zero dependencies|Smallest possible binary size; manual parsing.|
|**lexopt**|Absolute control|Minimalist, no-macro approach for POSIX compliance.|

### Binary Size & Compile Time

`clap` is feature-rich but adds to the binary size and compilation time (due to proc-macros). For embedded systems or extremely small "one-off" tools, `argh` or `pico-args` are often preferred.