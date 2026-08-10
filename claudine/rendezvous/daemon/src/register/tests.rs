use super::*;
use serde_json::json;
use tempfile::TempDir;

struct Harness {
    store: RegisterStore,
    storage: Storage,
    _tmp: TempDir,
}

fn build_harness() -> Harness {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("registers.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([42u8; 32]));
    let store = RegisterStore::new(storage.clone(), identity).expect("store");
    Harness {
        store,
        storage,
        _tmp: tmp,
    }
}

fn fields(pairs: &[(&str, JsonValue)]) -> serde_json::Map<String, JsonValue> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

#[test]
fn upsert_writes_then_skips_unchanged() {
    let harness = build_harness();
    let doc_id = harness.store.local_capability_id();

    let first = harness
        .store
        .upsert_local_fields(
            &doc_id,
            &fields(&[("os", json!("macOS")), ("cpu_cores", json!(16))]),
        )
        .expect("first write");
    assert!(first);

    // Identical detection pass: nothing written, register untouched.
    let second = harness
        .store
        .upsert_local_fields(
            &doc_id,
            &fields(&[("os", json!("macOS")), ("cpu_cores", json!(16))]),
        )
        .expect("second write");
    assert!(!second);

    // A single changed field writes again.
    let third = harness
        .store
        .upsert_local_fields(&doc_id, &fields(&[("cpu_cores", json!(32))]))
        .expect("third write");
    assert!(third);

    let value = harness
        .store
        .deep_value(&doc_id)
        .expect("read")
        .expect("present");
    assert_eq!(value["os"], json!("macOS"));
    assert_eq!(value["cpu_cores"], json!(32));
    assert_eq!(harness.storage.register_count().expect("count"), 1);
}

#[test]
fn mutate_local_fields_is_atomic_and_write_on_change() {
    let harness = build_harness();
    let doc_id = harness.store.local_capability_id();

    // The closure sees the current (empty) map and returns a
    // complete desired set; the returned `T` rides back out.
    let (changed, count) = harness
        .store
        .mutate_local_fields(&doc_id, |current| {
            assert!(current.is_empty(), "fresh register starts empty");
            let mut next = current.clone();
            next.insert("os".into(), json!("macOS"));
            next.insert("cpu_cores".into(), json!(16));
            let n = next.len() as u64;
            Ok((next, n))
        })
        .expect("first mutate");
    assert!(changed, "first mutate writes");
    assert_eq!(count, 2);

    // Returning the identical map touches nothing (write-on-change).
    let (changed, _) = harness
        .store
        .mutate_local_fields(&doc_id, |current| {
            let next = current.clone();
            Ok((next, ()))
        })
        .expect("idempotent mutate");
    assert!(!changed, "unchanged desired map must not write");

    // Omitting a key from the desired set deletes it (delete-missing).
    let (changed, _) = harness
        .store
        .mutate_local_fields(&doc_id, |current| {
            let mut next = current.clone();
            next.remove("cpu_cores");
            Ok((next, ()))
        })
        .expect("delete mutate");
    assert!(changed);
    let value = harness
        .store
        .deep_value(&doc_id)
        .expect("read")
        .expect("present");
    assert_eq!(value["os"], json!("macOS"));
    assert!(
        value.get("cpu_cores").is_none(),
        "omitted key deleted: {value}"
    );
}

#[test]
fn upsert_rejects_foreign_owner_and_unsupported_values() {
    let harness = build_harness();
    let foreign = DocumentId::capability("someone-else");
    let result = harness
        .store
        .upsert_local_fields(&foreign, &fields(&[("os", json!("Linux"))]));
    assert!(matches!(result, Err(RegisterError::NotOwner { .. })));

    let local = harness.store.local_capability_id();
    let result = harness
        .store
        .upsert_local_fields(&local, &fields(&[("bad", json!(["array"]))]));
    assert!(matches!(
        result,
        Err(RegisterError::UnsupportedValue { .. })
    ));
}

#[test]
fn rehydrates_from_storage() {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("registers.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([42u8; 32]));
    let doc_id;
    {
        let store = RegisterStore::new(storage.clone(), Arc::clone(&identity)).expect("store");
        doc_id = store.local_capability_id();
        store
            .upsert_local_fields(&doc_id, &fields(&[("memory", json!(65_536))]))
            .expect("write");
    }
    let reopened = RegisterStore::new(storage, identity).expect("reopen");
    let value = reopened
        .deep_value(&doc_id)
        .expect("read")
        .expect("present");
    assert_eq!(value["memory"], json!(65_536));
}

/// Simulates a second node's store so remote-import paths can be
/// exercised without a network.
fn remote_store(seed: u8) -> (RegisterStore, DocumentId, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let storage = Storage::open(tmp.path().join("registers.redb")).expect("storage");
    let identity = Arc::new(NodeIdentity::from_seed([seed; 32]));
    let store = RegisterStore::new(storage, identity).expect("store");
    let doc_id = store.local_capability_id();
    (store, doc_id, tmp)
}

#[test]
fn snapshot_then_delta_round_trip_between_stores() {
    let local = build_harness();
    let (remote, remote_doc, _tmp) = remote_store(7);

    remote
        .upsert_local_fields(&remote_doc, &fields(&[("os", json!("Linux"))]))
        .expect("remote write");

    // Bootstrap: full snapshot for a peer with no copy.
    let exported = remote
        .export_updates_since(&remote_doc, None)
        .expect("export")
        .expect("doc exists");
    assert_eq!(exported.kind, PayloadKind::Snapshot);
    let staged = local
        .store
        .stage_remote(&remote_doc, &exported.bytes, exported.kind)
        .expect("stage");
    assert!(
        local
            .store
            .commit_staged(&remote_doc, staged)
            .expect("commit")
    );

    // Incremental: remote changes a field; local follows via delta.
    remote
        .upsert_local_fields(&remote_doc, &fields(&[("cpu_cores", json!(8))]))
        .expect("remote update");
    let local_vv = local
        .store
        .state_vector(&remote_doc)
        .expect("vv")
        .expect("replica present");
    let exported = remote
        .export_updates_since(&remote_doc, Some(&local_vv))
        .expect("export")
        .expect("doc exists");
    assert_eq!(exported.kind, PayloadKind::Delta);
    let staged = local
        .store
        .stage_remote(&remote_doc, &exported.bytes, exported.kind)
        .expect("stage");
    assert!(
        local
            .store
            .commit_staged(&remote_doc, staged)
            .expect("commit")
    );

    let value = local
        .store
        .deep_value(&remote_doc)
        .expect("read")
        .expect("present");
    assert_eq!(value["os"], json!("Linux"));
    assert_eq!(value["cpu_cores"], json!(8));
}

#[test]
fn foreign_peer_ops_are_rejected() {
    let local = build_harness();
    let owner = "victim-node";
    let doc_id = DocumentId::capability(owner);

    // An attacker crafts a register for `victim-node` but writes it
    // with a peer id NOT derived from that owner.
    let forged = LoroDoc::new();
    forged.set_peer_id(owner_peer_id("attacker-node")).unwrap();
    forged
        .get_map(FIELDS_CONTAINER)
        .insert("os", "pwned")
        .unwrap();
    forged.commit();
    let bytes = forged.export(ExportMode::Snapshot).unwrap();

    let result = local
        .store
        .stage_remote(&doc_id, &bytes, PayloadKind::Snapshot);
    assert!(matches!(result, Err(RegisterError::ForeignWriter { .. })));
}

#[test]
fn stale_replica_recovers_via_snapshot_replace() {
    let local = build_harness();
    let (remote, remote_doc, _tmp) = remote_store(9);

    remote
        .upsert_local_fields(&remote_doc, &fields(&[("os", json!("Linux"))]))
        .expect("w1");
    let exported = remote
        .export_updates_since(&remote_doc, None)
        .expect("export")
        .expect("doc");
    let staged = local
        .store
        .stage_remote(&remote_doc, &exported.bytes, exported.kind)
        .expect("stage");
    local
        .store
        .commit_staged(&remote_doc, staged)
        .expect("commit");
    let stale_vv = local
        .store
        .state_vector(&remote_doc)
        .expect("vv")
        .expect("present");

    // Remote advances twice, then re-bases: the stale replica's
    // version now predates the shallow root.
    remote
        .upsert_local_fields(&remote_doc, &fields(&[("cpu_cores", json!(8))]))
        .expect("w2");
    remote
        .upsert_local_fields(&remote_doc, &fields(&[("memory", json!(1024))]))
        .expect("w3");
    {
        let mut inner = remote.inner.lock();
        let doc = inner.get(&remote_doc.as_path()).unwrap().clone();
        let rebased = remote
            .rebase(&doc, owner_peer_id(remote_doc.owner_node_id()))
            .expect("rebase");
        inner.insert(remote_doc.as_path(), rebased);
    }

    let exported = remote
        .export_updates_since(&remote_doc, Some(&stale_vv))
        .expect("export")
        .expect("doc");
    assert_eq!(exported.kind, PayloadKind::SnapshotReplace);

    let staged = local
        .store
        .stage_remote(&remote_doc, &exported.bytes, exported.kind)
        .expect("stage replace");
    assert!(
        local
            .store
            .commit_staged(&remote_doc, staged)
            .expect("commit replace")
    );
    let value = local
        .store
        .deep_value(&remote_doc)
        .expect("read")
        .expect("present");
    assert_eq!(value["os"], json!("Linux"));
    assert_eq!(value["cpu_cores"], json!(8));
    assert_eq!(value["memory"], json!(1024));
}

#[test]
fn field_deletion_propagates_via_delta() {
    let local = build_harness();
    let (remote, remote_doc, _tmp) = remote_store(11);

    remote
        .upsert_local_fields(&remote_doc, &fields(&[("a", json!(1)), ("b", json!(2))]))
        .expect("seed");
    let exported = remote
        .export_updates_since(&remote_doc, None)
        .expect("export")
        .expect("doc");
    let staged = local
        .store
        .stage_remote(&remote_doc, &exported.bytes, exported.kind)
        .expect("stage");
    local
        .store
        .commit_staged(&remote_doc, staged)
        .expect("commit");

    // Remote deletes a field; local follows via delta.
    assert!(
        remote
            .remove_local_fields(&remote_doc, &["a"])
            .expect("remove")
    );
    let local_vv = local
        .store
        .state_vector(&remote_doc)
        .expect("vv")
        .expect("present");
    let exported = remote
        .export_updates_since(&remote_doc, Some(&local_vv))
        .expect("export")
        .expect("doc");
    assert_eq!(exported.kind, PayloadKind::Delta);
    assert!(!exported.bytes.is_empty(), "delete must produce a delta");
    let staged = local
        .store
        .stage_remote(&remote_doc, &exported.bytes, exported.kind)
        .expect("stage");
    assert!(
        local
            .store
            .commit_staged(&remote_doc, staged)
            .expect("commit")
    );

    let value = local
        .store
        .deep_value(&remote_doc)
        .expect("read")
        .expect("present");
    assert!(
        value.get("a").is_none(),
        "deleted field must propagate: {value}"
    );
    assert_eq!(value["b"], json!(2));
}
