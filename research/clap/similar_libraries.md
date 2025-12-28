While **clap** is the most popular and feature-rich command-line argument parser in the Rust ecosystem, it can be heavy in terms of compile times and binary size. Depending on your needs—such as faster compilation, smaller binaries, or a more functional programming style—here are the best alternatives.

---

### 1. bpaf

**bpaf** (Basic Post-fix Argument Parser) is a combinator-based argument parser. It is often considered the most powerful alternative to `clap`, offering a balance between a high-level declarative API and low-level control without the heavy compile-time tax of `clap`.

* **Pros:**
  * Extremely flexible; can parse complex, nested structures.
  * Significantly faster compile times than `clap`.
  * Supports both "Derive" and "Combinator" (functional) styles.
  * Produces very small binaries.
* **Cons:**
  * The combinator API has a steeper learning curve than `clap`.
  * Documentation can be dense due to the library's flexibility.

**Links:**

* **Repo:** [https://github.com/pacak/bpaf](https://github.com/pacak/bpaf)
* **Docs.rs:** [https://docs.rs/bpaf](https://docs.rs/bpaf)
* **Documentation Site:** [https://pacak.github.io/bpaf/](https://pacak.github.io/bpaf/)

---

### 2. argh

Developed by Google, **argh** is an opinionated, derive-based argument parser designed to be simple and lightweight. It prioritizes small binary sizes and fast compile times by strictly following a specific CLI philosophy (e.g., no support for obscure POSIX edge cases).

* **Pros:**
  * Minimalist and easy to learn.
  * Tiny footprint (great for CLI tools where binary size matters).
  * Strict adherence to a clean, consistent CLI style.
* **Cons:**
  * Very opinionated; it is difficult to customize the help output or behavior.
  * Does not support "short" flags (e.g., `-v`) for anything other than basic switches in some configurations.
  * Missing many of the "quality of life" features found in `clap`.

**Links:**

* **Repo:** [https://github.com/google/argh](https://github.com/google/argh)
* **Docs.rs:** [https://docs.rs/argh](https://docs.rs/argh)

---

### 3. pico-args

**pico-args** is the ultimate "no-frills" choice. It is a minimalist, zero-dependency library that provides a simple way to pull arguments out of the environment. Unlike `clap`, it does not use macros or build a tree; you simply "chip away" at the arguments.

* **Pros:**
  * Zero dependencies.
  * Fastest possible compile times.
  * No complex traits or macros; just a simple wrapper around `std::env::args`.
* **Cons:**
  * No automatic "Help" or "Version" generation; you must write the help text yourself.
  * Requires manual imperative code to parse arguments (no `derive` support).
  * Not suitable for highly complex CLI interfaces.

**Links:**

* **Repo:** [https://github.com/RazrFalcon/pico-args](https://github.com/RazrFalcon/pico-args)
* **Docs.rs:** [https://docs.rs/pico-args](https://docs.rs/pico-args)

---

### 4. gumdrop

**gumdrop** is a "pragmatic" argument parser that uses a derive macro to generate a parser. It aims to be a middle ground between the heavy-duty nature of `clap` and the manual effort of `pico-args`.

* **Pros:**
  * Easy to use with a clean `#[derive(Options)]` interface.
  * Generates help text automatically.
  * Very fast compilation compared to `clap`'s derive feature.
* **Cons:**
  * Does not support subcommands as robustly as `clap`.
  * Smaller community and less frequent updates than `clap` or `bpaf`.

**Links:**

* **Repo:** [https://github.com/murarth/gumdrop](https://github.com/murarth/gumdrop)
* **Docs.rs:** [https://docs.rs/gumdrop](https://docs.rs/gumdrop)

---

### 5. lexopt

**lexopt** is a minimalist, "un-opinionated" parser. It doesn't try to be a framework; it simply helps you follow the standard conventions for argument parsing (like handling `--`, `=` in long flags, and grouped short flags) while leaving the logic to you.

* **Pros:**
  * Extremely small and fast.
  * Correctly handles complex POSIX/GNU argument conventions that other small libraries miss.
  * No dependencies.
* **Cons:**
  * Very low-level; you have to write a `while` loop to match against arguments.
  * No automatic help generation or type conversion (you handle the strings yourself).

**Links:**

* **Repo:** [https://github.com/blythe-at/lexopt](https://github.com/blythe-at/lexopt)
* **Docs.rs:** [https://docs.rs/lexopt](https://docs.rs/lexopt)

---

### Summary Recommendation

|If you want...|Use...|
|:-------------|:-----|
|**The full feature set (standard)**|`clap`|
|**Maximum flexibility + Performance**|`bpaf`|
|**Google-style simplicity**|`argh`|
|**Zero dependencies / Absolute minimum**|`pico-args`|
|**Strict POSIX compliance without the bloat**|`lexopt`|