#[cfg(unix)]
#[test]
fn os_subcommand_runs_in_pty() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;

    use expectrl::{Eof, Expect, Regex, Session};

    let fixture = common::SniffCliFixture::named("sniff-os-pty");
    let mut command = fixture.command_std();
    command.arg("os");
    let mut session = Session::spawn(command)?;
    session.set_expect_timeout(Some(Duration::from_secs(10)));
    session.expect(Regex("Operating System"))?;
    session.expect(Eof)?;

    Ok(())
}

#[cfg(unix)]
mod common;
