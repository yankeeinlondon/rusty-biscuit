//! The call chain's cycle, hop, and release rules, and runner scoping.

use super::*;

fn call<'a>(caller: &'a Path, overlay: &'a IndexMap<String, Value>) -> PrepCall<'a> {
    PrepCall {
        target: "@target.md",
        overlay,
        caller,
        file_resolution_context: crate::test_support::process_context(),
        property: "start[0].prep",
    }
}

#[test]
fn a_released_call_can_be_made_again_and_an_active_caller_cannot_be_called() {
    let overlay = IndexMap::new();
    let caller = PathBuf::from("/work/caller.md");
    let helper = PathBuf::from("/work/helper.md");
    for _ in 0..3 {
        let frame = enter_call(&call(&caller, &overlay), &helper).expect("a released call is legal again");
        assert_eq!(active_chain(), [caller.clone(), helper.clone()]);
        // The helper calling its active caller closes a cycle.
        let error = enter_call(&call(&helper, &overlay), &caller).err().expect("cycle");
        assert!(matches!(&error, CompositionError::LifecyclePrepCycle { chain, .. } if chain.len() == 2), "{error}");
        assert!(proxy_reaches_an_active_caller(&caller), "a handoff to an active caller is the same cycle");
        assert!(!proxy_reaches_an_active_caller(Path::new("/work/other.md")));
        drop(frame);
        assert!(active_chain().is_empty(), "returning releases the call");
    }
}

#[test]
fn a_chain_at_the_hop_limit_refuses_another_call() {
    let overlay = IndexMap::new();
    let documents: Vec<PathBuf> = (0..=MAX_PROXY_HOPS).map(|index| PathBuf::from(format!("/work/d{index}.md"))).collect();
    let mut frames = Vec::new();
    let mut refused = None;
    for pair in documents.windows(2) {
        match enter_call(&call(&pair[0], &overlay), &pair[1]) {
            Ok(frame) => frames.push(frame),
            Err(error) => { refused = Some(error); break; }
        }
    }
    let error = refused.expect("the chain stops at its limit");
    assert!(matches!(error, CompositionError::LifecyclePrepCycle { limit, .. } if limit == MAX_PROXY_HOPS));
}

struct Recording(std::sync::Mutex<Vec<String>>);

impl PrepRunner for Recording {
    fn run(&self, call: &PrepCall<'_>) -> Result<(), PrepFailure> {
        self.0.lock().unwrap().push(call.target.to_string());
        Ok(())
    }
}

#[test]
fn an_installed_runner_is_scoped_to_its_thread_and_inherited_explicitly() {
    assert!(current().is_none());
    let runner = Arc::new(Recording(std::sync::Mutex::new(Vec::new())));
    let scope = install(runner.clone());
    assert!(current().is_some());
    let inherited = inherit();
    std::thread::scope(|threads| {
        threads.spawn(|| assert!(current().is_none(), "a new thread inherits nothing by itself"));
        threads.spawn(|| {
            let _entered = inherited.enter();
            assert!(current().is_some());
        });
    });
    drop(scope);
    assert!(current().is_none(), "dropping the scope restores the previous runner");
}
