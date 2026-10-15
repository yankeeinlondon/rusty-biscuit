"""Run the isolated design proof without modifying production or tracked artifacts."""
from pathlib import Path
import os
import subprocess
import tempfile

root = Path(__file__).resolve().parent
env = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"), CARGO_TERM_COLOR="never")

def run(*args, expected=0, contains=None):
    result = subprocess.run(args, cwd=root, env=env, capture_output=True, text=True, check=False)
    output = result.stdout + result.stderr
    if result.returncode != expected or (contains and contains not in output):
        raise RuntimeError(f"{' '.join(args)}: exit {result.returncode}\n{output}")
    return result.stdout

run("cargo", "nextest", "run", "--lib")
print("PASS: 5 L1 tests selected and passed by nextest")
source = run("cargo", "run", "--quiet", "--bin", "emitted_path_source")
with tempfile.TemporaryDirectory(prefix="schematic-registration-consumer-") as directory:
    consumer = Path(directory)
    (consumer / "src").mkdir()
    (consumer / "Cargo.toml").write_text(
        '[package]\nname="spike-consumer"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\nschematic-registration-spike={path='
        + '"' + root.as_posix() + '"}\n', encoding="utf-8")
    (consumer / "src/main.rs").write_text(source, encoding="utf-8")
    run("cargo", "check", "--manifest-path", str(consumer / "Cargo.toml"), "--offline")
print("PASS: isolated consumer compiled every emitted alias/reexport/container/foreign/generic path")
for example, message in [
    ("mismatched_type", "mismatched types"),
    ("misspelled_path", "cannot find type `DoesNotExist`"),
    ("private_descriptor", "error[E0451]"),
]:
    run("cargo", "check", "--example", example, "--features", "negative-cases", expected=101, contains=message)
    print(f"PASS: {example} rejected by rustc ({message})")
print("PASS: 5 L1 tests, isolated consumer, and 3 expected compiler rejections")
