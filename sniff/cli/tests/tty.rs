#[cfg(unix)]
#[test]
fn os_subcommand_runs_in_pty() -> Result<(), Box<dyn std::error::Error>> {
    use expectrl::{Eof, Expect, Regex, spawn};

    let bin = assert_cmd::cargo::cargo_bin("sniff");
    let mut session = spawn(format!("{} os", bin.display()))?;
    session.expect(Regex("Operating System"))?;
    session.expect(Eof)?;

    Ok(())
}
