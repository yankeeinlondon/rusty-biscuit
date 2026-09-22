//! Initialization must use the document namespace and retain runtime writes.

#[cfg(unix)]
use crate::common;

#[cfg(unix)]
#[test]
fn loop_initialize_retains_document_context_and_mapping_writes() {
    use common::{CliProcessFixture, strip_ansi, write, write_executable};

    for fail in [false, true] {
        let fixture = CliProcessFixture::named("loop-initialize-state");
        fixture.initialize_repository();
        fixture.seed_user_config();
        let capture = fixture.cwd().join("delivered.txt");
        write_executable(
            &fixture.bin_dir().join("goose"),
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$CLAUDINE_PROMPT_CAPTURE\"\nexit 0\n",
        );
        let ending = if fail { "            - error: original initialize failure\n" } else { "" };
        let prompt = fixture.cwd().join("prompt.md");
        write(&prompt, &format!(r#"---
phase: 3
total_phases: 4
plan: fixes/case/plan.md
derived: "phase {{{{phase}}}}"
initialize:
    stack:
        - action:
            - set:
                epilog: "{{{{message_to_agent}}}}"
                message_to_agent: null
            - set:
                carried: "ready"
{ending}failure:
    message: "phase {{{{phase}}}} in {{{{parent_dir(plan)}}}}: {{{{err.msg}}}}"
    stderr: "plan={{{{plan}}}}; carried={{{{carried}}}}; cause={{{{err.msg}}}}"
loop:
    until: "phase >= total_phases"
    action: "increment(phase)"
---
phase={{{{phase}}}}; carried={{{{carried}}}}; epilog=[{{{{epilog}}}}]
derived=[{{{{derived}}}}]
"#));
        let output = fixture.command()
            .env("CLAUDINE_PROMPT_CAPTURE", &capture)
            .arg("compose").arg(&prompt).args(["--goose", "-y"])
            .output().unwrap();
        let rendered = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(!rendered.contains("unknown root"), "{rendered}");
        if fail {
            assert!(!output.status.success(), "{rendered}");
            assert!(rendered.contains("plan=fixes/case/plan.md; carried=ready; cause=original initialize failure"), "{rendered}");
            assert!(!capture.exists());
        } else {
            assert!(output.status.success(), "{rendered}");
            let delivered = std::fs::read_to_string(&capture).unwrap();
            for phase in [3, 4] {
                assert!(delivered.contains(&format!("phase={phase}; carried=ready; epilog=[]")), "{delivered}");
                assert!(delivered.contains(&format!("derived=[phase {phase}]")), "{delivered}");
            }
        }
        assert!(!fixture.audio_spool().exists());
    }
}
