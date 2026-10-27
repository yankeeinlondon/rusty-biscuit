use super::*;

#[test]
fn nested_group_legs_keep_memory_private_but_reread_shared_files_between_steps() {
    struct CoordinatedShell {
        target: PathBuf,
        progress: Mutex<(bool, bool)>,
        changed: std::sync::Condvar,
    }
    impl TaskShellRunner for CoordinatedShell {
        fn run(&self, command: &str, _timeout: Duration, _interrupt: Option<&AtomicBool>,
            _live: Option<&Arc<TaskLiveOutput>>) -> Result<ShellCommandOutput, TaskShellError>
        {
            match command {
                "await-file" => {
                    let mut progress = self.progress.lock().unwrap();
                    progress.0 = true;
                    self.changed.notify_all();
                    let (progress, wait) = self.changed.wait_timeout_while(progress,
                        Duration::from_secs(2), |progress| !progress.1).unwrap();
                    assert!(progress.1 && !wait.timed_out(), "sibling never published the shared file");
                }
                "publish-file" => {
                    let progress = self.progress.lock().unwrap();
                    let (mut progress, wait) = self.changed.wait_timeout_while(progress,
                        Duration::from_secs(2), |progress| !progress.0).unwrap();
                    assert!(progress.0 && !wait.timed_out(), "reader never reached its first child");
                    fs::write(&self.target, "---\nstate: new-file\n---\n").unwrap();
                    progress.1 = true;
                    self.changed.notify_all();
                }
                other => panic!("unexpected command: {other}"),
            }
            Ok(ShellCommandOutput { stdout:command.into(), exit_code:0,
                timed_out:false, interrupted:false, runaway:None })
        }
    }
    let dir = TempDir::new().unwrap();
    // The discovered graph is frozen, so the sibling's edit is observed through
    // a file the step reads when it runs, not by rewriting the step itself.
    fs::write(dir.path().join("shared.md"), "---\nstate: old-file\n---\n").unwrap();
    // A referenced document reads its own properties, under the leg's
    // runtime writes: `own` shows the leg's write, `sibling` that the other
    // leg's write stayed private.
    write_yaml(dir.path(), "next.yaml", &json!({"own":"initial", "sibling":"initial", "sequence":[
        {"name":"observe", "side_effect":{"set":{"observed":"{{ frontmatter('shared.md', 'state') }};own={{ own }};sibling={{ sibling }}"}}}
    ]}));
    let source = write_source(dir.path(), "root.md", &[
        ("own", json!("initial")), ("sibling", json!("initial")),
        ("sequence", json!([{"name":"container", "group":{"name":"concurrent", "tasks":[
            {"name":"a", "sequence":[
                {"name":"mutate-and-wait", "sequence":[{"name":"wait", "shell":"await-file"}],
                    "initialize":[{"set":{"own":"a-write"}}]},
                {"name":"read", "sequence":"next.yaml"}
            ]},
            {"name":"b", "sequence":[
                {"name":"mutate", "side_effect":{"set":{"sibling":"b-write"}}},
                {"name":"publish", "shell":"publish-file"}
            ]}
        ]}}]))
    ], "Body.");
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let shell = CoordinatedShell { target:fixture._dir.path().join("shared.md"),
        progress:Mutex::new((false, false)), changed:std::sync::Condvar::new() };
    let runtime = Arc::new(RuntimeState::new());
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.runtime = Some(&runtime); wiring.shell = &shell;
    let outcome = fixture.execute(&wiring);
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(runtime.snapshot().mutations.get("observed"), Some(&json!("new-file;own=a-write;sibling=initial")));
    assert_eq!(runtime.snapshot().mutations.get("own"), Some(&json!("a-write")));
    assert_eq!(runtime.snapshot().mutations.get("sibling"), Some(&json!("b-write")));
    assert_eq!(outcome.result, json!({"a":[["await-file"],["{\"observed\":null}"]], "b":["{\"sibling\":\"initial\"}","publish-file"]}));
    assert_eq!(runtime.outputs_value(), json!([outcome.result]));
}

#[test]
fn unhandled_nested_failure_finalizes_innermost_first_and_never_starts_later_children() {
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), json!({"name":"outer", "sequence":[
        {"name":"group", "group":{"name":"inner", "tasks":[
            {"name":"leg", "sequence":[{"name":"bad", "shell":"bad"}],
                "failure":[{"info":"leg-failure"}], "finalize":[{"info":"leg-finalize"}]}
        ], "failure":[{"info":"group-failure"}], "finalize":[{"info":"group-finalize"}]}},
        {"name":"later", "shell":"never"}
    ], "failure":[{"info":"outer-failure"}], "finalize":[{"info":"outer-finalize"}]}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default().stdout_for("bad", "partial").failing("bad");
    let runtime = Arc::new(RuntimeState::new());
    let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), ..Wiring::new(&recorder, &shell) });
    assert_eq!(outcome.status, TaskStatus::Failed);
    assert_eq!(shell.commands(), ["bad"]);
    assert_eq!(recorder.events(), ["info:leg-failure", "info:leg-finalize", "info:group-failure",
        "info:group-finalize", "info:outer-failure", "info:outer-finalize"]);
    assert_eq!(runtime.outputs_value(), json!([]));
}

#[test]
fn timed_out_nested_leg_uses_failure_retry_and_never_cancellation() {
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), json!({"name":"container", "group":{"name":"outer", "tasks":[
        {"name":"leg", "sequence":[{"name":"work", "shell":"attempt", "timeout":"1s"}],
            "failure":[{"info":"failure"},{"retry":{"max_attempts":1}}],
            "cancelled":[{"info":"cancelled"}], "finalize":[{"info":"finalize"}]}
    ]}}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::with_results(vec![
        ShellCommandOutput { stdout:"partial".into(), exit_code:1, timed_out:true, interrupted:false, runaway:None },
        ShellCommandOutput { stdout:"answer".into(), exit_code:0, timed_out:false, interrupted:false, runaway:None },
    ]);
    let runtime = Arc::new(RuntimeState::new());
    let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), ..Wiring::new(&recorder, &shell) });
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(shell.commands(), ["attempt", "attempt"]);
    assert_eq!(recorder.events(), ["info:failure", "info:finalize"]);
    assert_eq!(runtime.outputs_value(), json!([{"leg":["answer"]}]));
}

#[test]
fn inner_group_and_sequence_roots_are_lexical_and_restored_after_return() {
    let dir = TempDir::new().unwrap();
    write_source(dir.path(), "prompt.md", &[], "Prompt.");
    let source = one_step_source(dir.path(), json!({"name":"outer-sequence", "sequence":[
        {"name":"outer-group", "group":{"name":"outer-group", "variables":{"scope":"outer"}, "tasks":[
            {"name":"nested-sequence", "sequence":[
                {"name":"inner-group", "group":{"name":"inner-group", "variables":{"scope":"inner"}, "tasks":[
                    {"name":"inside", "prompt":"prompt.md"}
                ]}},
                {"name":"after-inner", "prompt":"prompt.md"}
            ]}
        ]}},
        {"name":"after-outer", "prompt":"prompt.md"}
    ]}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let prompt = FakePrompt::succeeding("answer");
    let runtime = Arc::new(RuntimeState::new());
    runtime.append_output("unrelated-outer-output");
    let outcome = fixture.execute(&Wiring { prompt:&prompt, runtime:Some(&runtime), ..Wiring::new(&recorder, &shell) });
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    let requests = prompt.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].lifecycle_group.as_ref().unwrap().get("scope"), Some(&json!("inner")));
    assert_eq!(requests[1].lifecycle_group.as_ref().unwrap().get("scope"), Some(&json!("outer")));
    assert!(requests[2].lifecycle_group.is_none());
    let first = requests[0].set_overrides.sequence_value().unwrap();
    let second = requests[1].set_overrides.sequence_value().unwrap();
    let third = requests[2].set_overrides.sequence_value().unwrap();
    assert_eq!(first["index"], 1);
    assert_eq!(first["count"], 2);
    assert_eq!(first["outputs"], json!([]));
    assert_eq!(second["index"], 2);
    assert_eq!(second["outputs"], json!([{"inside":"answer"}]));
    assert_eq!(third["index"], 2);
    assert_eq!(third["outputs"], json!([{"nested-sequence":[{"inside":"answer"},"answer"]}]));
    assert_eq!(runtime.outputs_value(), json!(["unrelated-outer-output", outcome.result]));
}

#[test]
fn container_failure_proxy_skips_source_finalize_and_runs_target_finalize() {
    let dir = TempDir::new().unwrap();
    write_yaml(dir.path(), "target.yaml", &json!({"sequence":[{"name":"target", "shell":"target"}],
        "success":[{"info":"target-success"}], "finalize":[{"info":"target-finalize"}]}));
    let source = write_source(dir.path(), "root.md", &[("sequence", json!([
        {"name":"source", "sequence":[{"name":"bad", "shell":"bad"}],
            "failure":[{"info":"source-failure"},{"proxy":"target.yaml"}],
            "finalize":[{"info":"source-finalize-forbidden"}]},
        {"name":"preapproved-target", "sequence":"target.yaml"}
    ]))], "Body.");
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default().stdout_for("bad", "partial").failing("bad")
        .stdout_for("target", "answer");
    let runtime = Arc::new(RuntimeState::new());
    let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), ..Wiring::new(&recorder, &shell) });
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(recorder.events(), ["info:source-failure", "info:target-success", "info:target-finalize"]);
    assert_eq!(shell.commands(), ["bad", "target"]);
    assert_eq!(runtime.outputs_value(), json!([["answer"]]));
}

#[test]
fn nested_sequence_legs_reject_shared_inline_writeback_during_preflight() {
    let dir = TempDir::new().unwrap();
    write_source(dir.path(), "target.md", &[("prompt", json!("Generate."))], "Body.");
    let source = one_step_source(dir.path(), json!({"name":"container", "group":{"name":"writers", "tasks":[
        {"name":"a", "sequence":[{"name":"write-a", "prompt":"target.md"}]},
        {"name":"b", "sequence":[{"name":"write-b", "prompt":"target.md"}]}
    ]}}));
    let error = match Fixture::build(dir, &source) {
        Ok(_) => panic!("nested concurrent inline writers must be rejected before execution"),
        Err(error) => error,
    };
    assert!(matches!(error, CompositionError::SequenceWriteBackCollision { .. }), "{error}");
    assert!(error.to_string().contains("target.md"));
}

macro_rules! repeated_retry {
    ($name:ident, $final_code:expr) => {
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let source = write_source(dir.path(), "root.md", &[
                ("retained", json!("initial")), ("recovered", json!("initial")),
                ("sequence", json!([{"name":"retrying", "sequence":[
                    {"name":"first", "shell":"first"},
                    {"name":"second", "side_effect":{"set":{"retained":"yes"}}},
                    {"name":"third", "shell":"third"}
                ], "initialize":[{"info":"initialize"}],
                "start":[{"info":"start={{ length(seq.outputs) }};retained={{ retained }};recovered={{ recovered }}"}],
                "failure":[{"set":{"recovered":"yes"}}, {"retry":{"max_attempts":2,"delay":"0s","with":{"attempt_note":"second"}}}],
                "finalize":[{"info":"finalize={{ attempt_note }}"}]}]))
            ], "Body.");
            let original = fs::read_to_string(&source).unwrap();
            let fixture = Fixture::build(dir, &source).unwrap();
            let recorder = Recorder::default();
            let shell = FakeTaskShell::with_results((0..3).flat_map(|attempt| [
                ShellCommandOutput { stdout:"first-result".into(), exit_code:0, timed_out:false, interrupted:false, runaway:None },
                ShellCommandOutput { stdout:"third-result".into(), exit_code:if attempt == 2 {$final_code} else {1},
                    timed_out:false, interrupted:false, runaway:None }
            ]).collect());
            let runtime = Arc::new(RuntimeState::new());
            let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), ..Wiring::new(&recorder, &shell) });
            assert_eq!(shell.commands(), ["first","third","first","third","first","third"]);
            assert_eq!(recorder.events(), ["info:initialize", "info:start=0;retained=initial;recovered=initial",
                "info:start=0;retained=yes;recovered=yes", "info:start=0;retained=yes;recovered=yes", "info:finalize=second"]);
            assert_eq!(runtime.snapshot().mutations.get("retained"), Some(&json!("yes")));
            assert_eq!(runtime.snapshot().mutations.get("recovered"), Some(&json!("yes")));
            assert_eq!(fs::read_to_string(&source).unwrap(), original);
            if $final_code == 0 {
                assert!(outcome.succeeded(), "{}", failure_message(&outcome));
                assert_eq!(outcome.result.as_array().unwrap().len(), 3);
                assert_eq!(outcome.result[0], "first-result");
                assert_eq!(outcome.result[2], "third-result");
                assert_eq!(runtime.outputs_value(), json!([outcome.result]));
            } else {
                assert_eq!(outcome.status, TaskStatus::Failed);
                assert!(!outcome.output_committed);
                assert_eq!(runtime.outputs_value(), json!([]));
            }
        }
    };
}
repeated_retry!(two_retries_retain_mutations_and_publish_exactly_three_children_once, 0);
repeated_retry!(exhausting_two_retries_keeps_failure_and_publishes_nothing, 1);

#[test]
fn cleanup_errors_stay_secondary_to_the_original_container_failure() {
    for group in [false, true] {
        let dir = TempDir::new().unwrap();
        let work = json!({"name":"bad", "shell":"bad"});
        let container = if group {
            json!({"name":"outer", "group":{"name":"checks", "tasks":[work],
                "finalize":[{"error":"cleanup failure"}]}})
        } else {
            json!({"name":"outer", "sequence":[work], "finalize":[{"error":"cleanup failure"}]})
        };
        let source = one_step_source(dir.path(), container);
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default().stdout_for("bad", "partial").failing("bad");
        let runtime = Arc::new(RuntimeState::new());
        let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), ..Wiring::new(&recorder, &shell) });
        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(!outcome.error.as_ref().unwrap().message().contains("cleanup failure"));
        assert!(outcome.secondary_errors.iter().any(|error| error.message().contains("cleanup failure")), "{outcome:?}");
        assert_eq!(runtime.outputs_value(), json!([]));
    }
}

#[test]
fn body_preparation_failure_runs_enclosing_failure_and_finalize() {
    struct RejectedBody;
    impl PromptTaskRunner for RejectedBody {
        fn validate(&self, _request: &PromptTaskRequest) -> Result<(), CompositionError> {
            Err(CompositionError::SequenceInvalid("body validation refused".into()))
        }
        fn run(&self, _request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError> {
            panic!("a rejected body cannot launch")
        }
    }
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), json!({"name":"container", "sequence":["body"],
        "failure":[{"info":"failure"}], "finalize":[{"info":"finalize"}]}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new());
    let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), prompt:&RejectedBody, ..Wiring::new(&recorder, &shell) });
    assert_eq!(outcome.status, TaskStatus::Failed);
    assert!(outcome.error.unwrap().message().contains("body validation refused"));
    assert_eq!(recorder.events(), ["info:failure", "info:finalize"]);
    assert_eq!(runtime.outputs_value(), json!([]));
}

#[test]
fn sibling_failure_cancels_opted_container_descendants_and_preserves_other_legs() {
    struct CoordinatedShell {
        ready: Mutex<usize>,
        changed: std::sync::Condvar,
        commands: Mutex<Vec<String>>,
    }
    impl TaskShellRunner for CoordinatedShell {
        fn run(&self, command: &str, _timeout: Duration, _interrupt: Option<&AtomicBool>,
            _live: Option<&Arc<TaskLiveOutput>>) -> Result<ShellCommandOutput, TaskShellError>
        {
            self.commands.lock().unwrap().push(command.into());
            let mut ready = self.ready.lock().unwrap();
            *ready += 1;
            self.changed.notify_all();
            let (ready, wait) = self.changed.wait_timeout_while(ready,
                Duration::from_secs(2), |ready| *ready < 4).unwrap();
            assert!(*ready >= 4 && !wait.timed_out(), "all four running leaves must be admitted");
            drop(ready);
            let mut interrupted = false;
            if command.starts_with("wait-") {
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                while !crate::composition::lifecycle::cancellation::requested() {
                    assert!(std::time::Instant::now() < deadline, "descendant never received cancellation");
                    std::thread::sleep(Duration::from_millis(2));
                }
                let cause = crate::composition::lifecycle::cancellation::cause().unwrap();
                assert!(matches!(cause, crate::composition::lifecycle::cancellation::CancellationCause::SiblingFailure {
                    ref group, ref leg, .. } if group == "atomic" && leg == "bad"));
                interrupted = true;
                *self.ready.lock().unwrap() += 1;
                self.changed.notify_all();
            } else if command == "complete" {
                let ready = self.ready.lock().unwrap();
                let (ready, wait) = self.changed.wait_timeout_while(ready,
                    Duration::from_secs(2), |ready| *ready < 5).unwrap();
                assert!(*ready >= 5 && !wait.timed_out(), "failure must stop admission before plain finishes");
            }
            Ok(ShellCommandOutput { stdout:command.into(), exit_code:if command == "fail" { 1 } else { 0 },
                timed_out:false, interrupted, runaway:None })
        }
    }
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), json!({"name":"root", "group":{"name":"atomic", "max_parallel":3, "tasks":[
        {"name":"opted", "sequence":[{"name":"nested-group", "group":{"name":"descendants", "tasks":[
            {"name":"a", "sequence":[{"name":"a-work", "shell":"wait-a"}], "finalize":[{"info":"a-finalize"}]},
            {"name":"b", "sequence":[{"name":"b-work", "shell":"wait-b"}], "cancelled":[], "finalize":[{"info":"b-finalize"}]}
        ], "finalize":[{"info":"descendants-finalize"}]}}],
            "cancelled":[{"info":"opted-cancelled"}], "finalize":[{"info":"opted-finalize"}]},
        {"name":"plain", "sequence":[{"name":"plain-work", "shell":"complete"}],
            "success":[{"info":"plain-success"}], "finalize":[{"info":"plain-finalize"}]},
        {"name":"bad", "shell":"fail"},
        {"name":"unadmitted", "sequence":[], "cancelled":[], "finalize":[{"info":"never"}]}
    ], "failure":[{"info":"failed={{ as_csv(group.failed) }};cancelled={{ as_csv(group.cancelled) }}"}],
       "finalize":[{"info":"atomic-finalize"}]}}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let shell = CoordinatedShell { ready:Mutex::new(0), changed:std::sync::Condvar::new(), commands:Mutex::new(Vec::new()) };
    let runtime = Arc::new(RuntimeState::new());
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = &shell; wiring.runtime = Some(&runtime);
    let outcome = fixture.execute(&wiring);
    assert_eq!(outcome.status, TaskStatus::Failed, "{}", failure_message(&outcome));
    assert_eq!(runtime.outputs_value(), json!([]));
    let events = recorder.events();
    for marker in ["info:a-finalize", "info:b-finalize", "info:plain-success", "info:plain-finalize"] {
        assert_eq!(events.iter().filter(|event| event.as_str() == marker).count(), 1, "{events:?}");
    }
    let position = |event: &str| events.iter().position(|item| item == event).unwrap();
    assert!(position("info:a-finalize") < position("info:descendants-finalize"));
    assert!(position("info:b-finalize") < position("info:descendants-finalize"));
    assert!(position("info:descendants-finalize") < position("info:opted-cancelled"));
    assert!(position("info:opted-cancelled") < position("info:opted-finalize"));
    assert!(position("info:opted-finalize") < position("info:atomic-finalize"));
    assert!(events.contains(&"info:failed=bad;cancelled=opted, unadmitted".into()), "{events:?}");
    assert!(!events.iter().any(|event| event == "info:never"));
}

#[test]
fn interrupted_sequence_keeps_cleanup_errors_secondary_and_never_retries() {
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), json!({"name":"outer", "sequence":[
        {"name":"inner", "sequence":[{"name":"work", "shell":"interrupt"}],
         "cancelled":[{"shell":"cleanup"}],
         "finalize":[{"info":"inner-finalize"},{"retry":{"max_attempts":1,"with":{"retried":true}}}]},
        {"name":"later", "shell":"never"}
    ], "failure":[{"info":"never-failure"}], "cancelled":[{"info":"outer-cancelled"}],
       "finalize":[{"info":"outer-finalize"}]}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    recorder.fail_stack_shell();
    let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
        stdout:"partial".into(), exit_code:130, timed_out:false, interrupted:true, runaway:None,
    }]);
    let runtime = Arc::new(RuntimeState::new());
    let outcome = fixture.execute(&Wiring {runtime:Some(&runtime), ..Wiring::new(&recorder, &shell)});
    assert_eq!(outcome.status, TaskStatus::Interrupted);
    assert_eq!(outcome.stdout, "partial");
    assert!(outcome.error.is_none());
    assert_eq!(shell.commands(), ["interrupt"]);
    assert_eq!(outcome.secondary_errors.len(), 2, "{:?}", outcome.secondary_errors);
    assert!(outcome.secondary_errors.iter().any(|error| error.info.msg.contains("cannot recover a cancelled")));
    assert!(!runtime.snapshot().mutations.contains_key("retried"));
    assert_eq!(runtime.outputs_value(), json!([]));
    let events = recorder.events();
    assert_eq!(events, ["stack-shell:cleanup", "info:inner-finalize", "info:outer-cancelled", "info:outer-finalize"]);
}

#[test]
fn cleanup_shell_timeout_stays_secondary_to_cancellation() {
    struct TimeoutShell;
    impl ShellRunner for TimeoutShell {
        fn run(&self, command: &str) -> Result<i32, ShellRunError> {
            Err(ShellRunError::Spawn { command: command.into(),
                source:std::io::Error::new(std::io::ErrorKind::TimedOut, "cleanup deadline exceeded") })
        }
        fn run_structured(&self, _chain: &darkmatter::markdown::compose::shell_expansion::template::FilledChain)
            -> Result<i32, ShellRunError>
        { self.run("cleanup") }
    }
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), json!({"name":"container", "sequence":[
        {"name":"child", "shell":"interrupt"}
    ], "cancelled":[{"shell":"cleanup"}], "failure":[{"info":"never"}],
       "finalize":[{"info":"finalized"}]}));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::with_results(vec![ShellCommandOutput {
        stdout:String::new(), exit_code:130, interrupted:true, timed_out:false, runaway:None,
    }]);
    let runtime = Arc::new(RuntimeState::new());
    let outcome = fixture.execute(&Wiring { runtime:Some(&runtime), lifecycle_shell:&TimeoutShell,
        ..Wiring::new(&recorder, &shell) });
    assert_eq!(outcome.status, TaskStatus::Interrupted);
    assert!(outcome.error.is_none());
    assert_eq!(outcome.secondary_errors.len(), 1);
    assert!(outcome.secondary_errors[0].info.msg.contains("cleanup deadline exceeded"));
    assert_eq!(recorder.events(), ["info:finalized"]);
    assert_eq!(runtime.outputs_value(), json!([]));
}

/// Each leg's live view is the group-start state plus only that leg's writes:
/// four concurrent legs write the same document key, every leg reads its own
/// value back in a later stage, and the authored definition (which holds the
/// group, its 40 legs, and their padding) stays readable and on disk.
#[test]
fn small_leg_writes_stay_private_over_a_large_inline_definition() {
    const LEGS: usize = 40;
    let dir = TempDir::new().unwrap();
    let padding = "x".repeat(2_048);
    let document = dir.path().join("seq.md").to_string_lossy().into_owned();
    let tasks: Vec<Value> = (0..LEGS).map(|index| json!({
        "name": format!("leg-{index}"), "shell": format!("work-{index}"),
        "setup": [{ "set_frontmatter": [document, "own", format!("leg-{index}")] }],
        "teardown": [{ "set": { format!("seen_{index}"): "{{ own }}|{{ title }}" } }],
    })).collect();
    let variables: Map<String, Value> = (0..LEGS).map(|index| (format!("pad_{index}"), json!(padding))).collect();
    let sequence = json!([{ "name": "fan-out", "group": { "name": "legs", "max_parallel": 4,
        "variables": variables, "tasks": tasks } }]);
    let source = write_source(dir.path(), "seq.md",
        &[("title", json!("T")), ("own", json!("initial")), ("sequence", sequence.clone())], "Body.\n");
    let fixture = Fixture::build(dir, &source).unwrap().mutating_its_directory();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new());
    let mut wiring = Wiring::new(&recorder, &shell);
    wiring.runtime = Some(&runtime);

    let outcome = fixture.execute(&wiring);

    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(shell.commands().len(), LEGS, "every leg ran");
    let mutations = runtime.snapshot().mutations;
    for index in 0..LEGS {
        assert_eq!(mutations.get(&format!("seen_{index}")), Some(&json!(format!("leg-{index}|T"))),
            "leg {index} must read its own write and the authored values, never a sibling's write");
    }
    let written = darkmatter::markdown::Markdown::try_from(Path::new(&source)).unwrap();
    let on_disk = written.frontmatter().as_map();
    let own = on_disk.get("own").and_then(Value::as_str).unwrap_or_default();
    assert!(own.starts_with("leg-"), "a leg's write reached the file: {own:?}");
    assert_eq!(on_disk.get("sequence"), Some(&sequence), "the authored definition is untouched");
}
