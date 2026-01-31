use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get the bh binary command
fn hash_cmd() -> Command {
    Command::cargo_bin("bh").unwrap()
}

mod xxhash_mode {
    use super::*;

    #[test]
    fn hashes_positional_content() {
        hash_cmd()
            .arg("hello world")
            .assert()
            .success()
            .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
    }

    #[test]
    fn same_content_produces_same_hash() {
        let output1 = hash_cmd()
            .arg("test content")
            .output()
            .expect("Failed to execute");

        let output2 = hash_cmd()
            .arg("test content")
            .output()
            .expect("Failed to execute");

        assert_eq!(output1.stdout, output2.stdout);
    }

    #[test]
    fn different_content_produces_different_hash() {
        let output1 = hash_cmd().arg("content1").output().expect("Failed");
        let output2 = hash_cmd().arg("content2").output().expect("Failed");

        assert_ne!(output1.stdout, output2.stdout);
    }
}

mod blake3_mode {
    use super::*;

    #[test]
    fn crypto_flag_produces_64_char_hex() {
        hash_cmd()
            .args(["--crypto", "test"])
            .assert()
            .success()
            .stdout(predicate::str::is_match("^[a-f0-9]{64}\n$").unwrap());
    }

    #[test]
    fn crypto_short_flag_works() {
        hash_cmd()
            .args(["-c", "test"])
            .assert()
            .success()
            .stdout(predicate::str::is_match("^[a-f0-9]{64}\n$").unwrap());
    }
}

mod password_mode {
    use super::*;

    #[test]
    fn password_flag_produces_phc_format() {
        hash_cmd()
            .args(["--password", "secret"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("$argon2id$"));
    }

    #[test]
    fn password_short_flag_works() {
        hash_cmd()
            .args(["-p", "secret"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("$argon2id$"));
    }

    #[test]
    fn password_with_stdin() {
        hash_cmd()
            .args(["--password", "-"])
            .write_stdin("mysecret")
            .assert()
            .success()
            .stdout(predicate::str::starts_with("$argon2id$"));
    }

    #[test]
    fn different_passwords_produce_different_hashes() {
        // Due to random salt, even same password produces different hashes
        let output1 = hash_cmd()
            .args(["--password", "same"])
            .output()
            .expect("Failed");

        let output2 = hash_cmd()
            .args(["--password", "same"])
            .output()
            .expect("Failed");

        // Hashes should differ due to random salt
        assert_ne!(output1.stdout, output2.stdout);
    }
}

mod file_mode {
    use super::*;

    #[test]
    fn file_flag_hashes_file_content() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "file content").unwrap();

        hash_cmd()
            .args(["--file", file_path.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
    }

    #[test]
    fn file_flag_short_version() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        hash_cmd()
            .args(["-f", file_path.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn file_flag_with_crypto() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        hash_cmd()
            .args(["--file", file_path.to_str().unwrap(), "--crypto"])
            .assert()
            .success()
            .stdout(predicate::str::is_match("^[a-f0-9]{64}\n$").unwrap());
    }

    #[test]
    fn file_not_found_error() {
        hash_cmd()
            .args(["--file", "/nonexistent/path/file.txt"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Failed to read file"));
    }
}

mod mutual_exclusivity {
    use super::*;

    #[test]
    fn crypto_and_password_mutually_exclusive() {
        hash_cmd()
            .args(["--crypto", "--password", "test"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("cannot be used with"));
    }

    #[test]
    fn file_and_content_mutually_exclusive() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        hash_cmd()
            .args(["--file", file_path.to_str().unwrap(), "extra_content"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("cannot be used with"));
    }
}

mod shell_completions {
    use super::*;

    #[test]
    fn bash_completions() {
        hash_cmd()
            .env("COMPLETE", "bash")
            .assert()
            .success()
            .stdout(predicate::str::contains("_bh()"));
    }

    #[test]
    fn zsh_completions() {
        hash_cmd()
            .env("COMPLETE", "zsh")
            .assert()
            .success()
            .stdout(predicate::str::contains("#compdef bh"));
    }

    #[test]
    fn fish_completions() {
        hash_cmd()
            .env("COMPLETE", "fish")
            .assert()
            .success()
            .stdout(predicate::str::contains("complete -c bh"));
    }

    #[test]
    fn invalid_shell_error() {
        hash_cmd()
            .env("COMPLETE", "invalid")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Unknown shell"));
    }
}

mod stdin_mode {
    use super::*;

    #[test]
    fn piped_input_without_args() {
        hash_cmd()
            .write_stdin("piped content")
            .assert()
            .success()
            .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
    }

    #[test]
    fn empty_stdin_error() {
        hash_cmd()
            .args(["--password", "-"])
            .write_stdin("")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Empty input"));
    }
}

mod help_and_version {
    use super::*;

    #[test]
    fn help_flag() {
        hash_cmd()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Hash content using various algorithms"));
    }

    #[test]
    fn version_flag() {
        hash_cmd()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains("bh"));
    }

    #[test]
    fn help_contains_completion_instructions() {
        hash_cmd()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("COMPLETE=bash"));
    }
}
