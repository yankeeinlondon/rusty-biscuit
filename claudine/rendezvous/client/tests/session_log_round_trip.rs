//! Phase-2 end-to-end validation.
//!
//! The test client appends entries to a session-log via the daemon's
//! gRPC interface and verifies that (a) the entries materialise in the
//! in-memory Loro doc (read-back via `list_chunk_entries`), (b) the
//! redb file on disk holds at least one snapshot, and (c) the entries
//! eventually surface in the DuckDB projection.

use std::time::Duration;

use rendezvous_client::connect;
use rendezvous_core::{
    AppendEntryRequest, ChunkConfig, ListChunkEntriesRequest, ListSessionChunksRequest,
    QueryProjectionRequest,
};
use rendezvous_daemon::batcher::BatcherConfig;
use rendezvous_core::local_endpoint::test_support::private_endpoint;
use rendezvous_daemon::local_transport::spawn_local_server;
use rendezvous_daemon::server::DaemonConfig;
use rendezvous_daemon::storage::Storage;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::sleep;

fn fast_config(tmp: &TempDir, chunk_config: ChunkConfig) -> DaemonConfig {
    let mut config = DaemonConfig::with_data_dir(tmp.path().join("data")).without_networking();
    config.batcher_config = BatcherConfig {
        flush_interval: Duration::from_millis(50),
        flush_size: 16,
    };
    config.chunk_config = chunk_config;
    config
}

#[tokio::test]
async fn append_persists_to_redb_and_eventually_to_duckdb() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = private_endpoint(tmp.path(), "daemon");
    let config = fast_config(&tmp, ChunkConfig::default());
    let storage_path = config.storage_path.clone();

    let handle = spawn_local_server(socket.clone(), config).expect("spawn");
    let node_id = handle.node_id();
    let mut client = connect(&socket).await.expect("connect");

    for i in 0..5 {
        client
            .append_entry(AppendEntryRequest {
                owner_node_id: String::new(),
                session_id: "session-1".into(),
                source: "test-client".into(),
                level: "info".into(),
                message: format!("hello-{i}"),
                metadata_json: String::new(),
            })
            .await
            .expect("append rpc");
    }

    // Loro doc (source of truth) should hold all five entries immediately.
    let chunk_id = format!("session/{node_id}/session-1/part/0");
    let listed = client
        .list_chunk_entries(ListChunkEntriesRequest {
            chunk_id: chunk_id.clone(),
        })
        .await
        .expect("list-entries")
        .into_inner();
    assert_eq!(listed.entries.len(), 5);
    assert_eq!(listed.entries[0].message, "hello-0");
    assert_eq!(listed.entries[4].sequence, 4);

    let chunks = client
        .list_session_chunks(ListSessionChunksRequest {
            owner_node_id: node_id.clone(),
            session_id: "session-1".into(),
        })
        .await
        .expect("list-chunks")
        .into_inner();
    assert_eq!(chunks.chunk_ids, vec![chunk_id]);

    // DuckDB projection: wait for the batcher to flush before reading.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let rows = client
            .query_projection(QueryProjectionRequest {
                owner_node_id: node_id.clone(),
                session_id: "session-1".into(),
            })
            .await
            .expect("query")
            .into_inner();
        if rows.rows.len() == 5 {
            let messages: Vec<_> = rows.rows.iter().map(|r| r.message.clone()).collect();
            assert_eq!(messages, (0..5).map(|i| format!("hello-{i}")).collect::<Vec<_>>());
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "duckdb projection never caught up: still {} of 5 rows",
                rows.rows.len(),
            );
        }
        sleep(Duration::from_millis(50)).await;
    }

    handle.shutdown().await.expect("shutdown");

    // After the daemon releases its exclusive lock, the redb file on
    // disk must contain at least one persisted snapshot. This is the
    // "data lands in redb" half of the Phase-2 validation.
    assert_redb_has_snapshots(&storage_path);
}

fn assert_redb_has_snapshots(path: &PathBuf) {
    let storage = Storage::open(path).expect("re-open redb for assertion");
    let count = storage.snapshot_count().expect("snapshot count");
    assert!(
        count >= 1,
        "expected at least one snapshot in {}, got {count}",
        path.display(),
    );
}

#[tokio::test]
async fn chunk_rotation_creates_new_chunk_at_threshold() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = private_endpoint(tmp.path(), "daemon");
    // Force rotation after every two appends so we deterministically
    // observe a chunk transition.
    let config = fast_config(&tmp, ChunkConfig::new(2, 16 * 1024));

    let handle = spawn_local_server(socket.clone(), config).expect("spawn");
    let node_id = handle.node_id();
    let mut client = connect(&socket).await.expect("connect");

    let mut rotated_seen = false;
    for i in 0..5 {
        let response = client
            .append_entry(AppendEntryRequest {
                owner_node_id: String::new(),
                session_id: "session-7".into(),
                source: "test".into(),
                level: "info".into(),
                message: format!("m-{i}"),
                metadata_json: String::new(),
            })
            .await
            .expect("append")
            .into_inner();
        if response.rotated_chunk {
            rotated_seen = true;
        }
    }
    assert!(rotated_seen, "at least one append should have rotated");

    let chunks = client
        .list_session_chunks(ListSessionChunksRequest {
            owner_node_id: node_id,
            session_id: "session-7".into(),
        })
        .await
        .expect("list-chunks")
        .into_inner();
    // 5 entries with a cap of 2 → chunks 0, 1, 2.
    assert_eq!(chunks.chunk_ids.len(), 3);

    handle.shutdown().await.expect("shutdown");
}
