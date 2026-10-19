//! A native executable that records the argument vector it was started with.
//!
//! Tests that must prove exactly which arguments a program received (one
//! argument per placeholder, empty arguments kept, no shell re-parsing) run
//! this instead of a shell builtin or a script, so the evidence is the same on
//! macOS, Linux, native Windows, and WSL2.
//!
//! The recorder body lives here; each consuming package declares one thin
//! `[[bin]]` whose `main` calls [`main`], and its tests find that binary at
//! run time (`biscuit_test_harness::bin_exe!`). A run appends one record to the
//! file named by [`ARGV_RECORDER_FILE`] and exits `0` without writing to
//! stdout by default. Optional `ARGV_RECORDER_STDOUT` and
//! `ARGV_RECORDER_STDERR` values emit stream probes after recording, for portable
//! runner and redirection tests. [`read_argv_records`] reads every record back.
//!
//! ## Record format
//!
//! Each invocation appends one record: the argument count, then each argument
//! as its byte length followed by its UTF-8 bytes. Every number is a
//! little-endian `u32`. The program's own name (`argv[0]`) is not recorded.
//! Lengths, not separators, delimit arguments, so an argument holding a
//! newline, a NUL-free control character, or nothing at all reads back
//! exactly.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

/// The environment variable naming the file each run appends its record to.
pub const ARGV_RECORDER_FILE: &str = "ARGV_RECORDER_FILE";

/// The recorder's entry point, for a consuming package's thin `[[bin]]`.
///
/// Exits `2` with a message on stderr when [`ARGV_RECORDER_FILE`] is unset or
/// the record cannot be written, so a misconfigured test fails loudly instead
/// of recording nothing.
pub fn main() -> ExitCode {
    let Some(path) = std::env::var_os(ARGV_RECORDER_FILE) else {
        eprintln!("argv recorder: {ARGV_RECORDER_FILE} is not set");
        return ExitCode::from(2);
    };
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(&encode_record(&args)));
    match written {
        Ok(()) => {
            if let Some(stdout) = std::env::var_os("ARGV_RECORDER_STDOUT") {
                print!("{}", stdout.to_string_lossy());
                std::io::stdout().flush().expect("flush recorder stdout");
            }
            if let Some(stderr) = std::env::var_os("ARGV_RECORDER_STDERR") {
                eprint!("{}", stderr.to_string_lossy());
                std::io::stderr().flush().expect("flush recorder stderr");
            }
            ExitCode::SUCCESS
        },
        Err(error) => {
            eprintln!("argv recorder: cannot write {}: {error}", Path::new(&path).display());
            ExitCode::from(2)
        }
    }
}

/// One record in the [record format](self#record-format).
pub fn encode_record(args: &[String]) -> Vec<u8> {
    let mut record = Vec::new();
    record.extend_from_slice(&length(args.len()).to_le_bytes());
    for arg in args {
        record.extend_from_slice(&length(arg.len()).to_le_bytes());
        record.extend_from_slice(arg.as_bytes());
    }
    record
}

fn length(len: usize) -> u32 {
    u32::try_from(len).expect("an argv recorder field fits in u32")
}

/// Every record in `path`, in the order the runs wrote them.
///
/// ## Errors
///
/// The read error, or [`std::io::ErrorKind::InvalidData`] for a truncated
/// record or an argument that is not UTF-8. A missing file is
/// [`std::io::ErrorKind::NotFound`]: nothing ran.
pub fn read_argv_records(path: &Path) -> std::io::Result<Vec<Vec<String>>> {
    decode_records(&std::fs::read(path)?)
}

/// Decodes a whole recorder file.
///
/// ## Errors
///
/// [`std::io::ErrorKind::InvalidData`] for a truncated record or an argument
/// that is not UTF-8.
pub fn decode_records(mut bytes: &[u8]) -> std::io::Result<Vec<Vec<String>>> {
    fn invalid(message: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("argv recorder file: {message}"))
    }
    fn take<'a>(bytes: &mut &'a [u8], count: usize) -> std::io::Result<&'a [u8]> {
        if bytes.len() < count {
            return Err(invalid("truncated record"));
        }
        let (head, rest) = bytes.split_at(count);
        *bytes = rest;
        Ok(head)
    }
    fn number(bytes: &mut &[u8]) -> std::io::Result<usize> {
        let field: [u8; 4] = take(bytes, 4)?.try_into().expect("took four bytes");
        Ok(u32::from_le_bytes(field) as usize)
    }

    let mut records = Vec::new();
    while !bytes.is_empty() {
        let count = number(&mut bytes)?;
        let mut args = Vec::with_capacity(count);
        for _ in 0..count {
            let len = number(&mut bytes)?;
            let arg = String::from_utf8(take(&mut bytes, len)?.to_vec()).map_err(|_| invalid("argument is not UTF-8"))?;
            args.push(arg);
        }
        records.push(args);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_every_argument_boundary() {
        let first: Vec<String> = ["a b", "", "x; rm -rf ~", "line\nbreak", "naïve", "%!"]
            .map(str::to_string)
            .to_vec();
        let second: Vec<String> = Vec::new();
        let mut file = encode_record(&first);
        file.extend(encode_record(&second));
        assert_eq!(decode_records(&file).unwrap(), vec![first, second]);
    }

    #[test]
    fn a_truncated_record_is_invalid_data() {
        let mut file = encode_record(&["abc".to_string()]);
        file.pop();
        let error = decode_records(&file).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
