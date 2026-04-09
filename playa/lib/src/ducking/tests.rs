//! Unit tests for the ducking module.

use super::*;

mod config_tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = DuckConfig::default();
        assert_eq!(config.ramp_ms(), 1000);
        assert!((config.floor_scalar() - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn new_config_accepts_valid_values() {
        let config = DuckConfig::new(500, 0.5).unwrap();
        assert_eq!(config.ramp_ms(), 500);
        assert!((config.floor_scalar() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn new_config_rejects_zero_ramp() {
        let result = DuckConfig::new(0, 0.5);
        assert!(matches!(result, Err(DuckingError::InvalidRampDuration(0))));
    }

    #[test]
    fn new_config_rejects_negative_floor() {
        let result = DuckConfig::new(1000, -0.1);
        assert!(matches!(result, Err(DuckingError::InvalidFloorScalar(f)) if f < 0.0));
    }

    #[test]
    fn new_config_rejects_floor_above_one() {
        let result = DuckConfig::new(1000, 1.5);
        assert!(matches!(result, Err(DuckingError::InvalidFloorScalar(f)) if f > 1.0));
    }

    #[test]
    fn new_config_accepts_floor_at_boundaries() {
        // Floor of 0.0 (complete silence) is valid
        assert!(DuckConfig::new(1000, 0.0).is_ok());
        // Floor of 1.0 (no ducking) is valid
        assert!(DuckConfig::new(1000, 1.0).is_ok());
    }

    #[test]
    fn config_is_copy() {
        let config = DuckConfig::default();
        let copied = config;
        assert_eq!(config.ramp_ms(), copied.ramp_ms());
    }
}

mod types_tests {
    use super::*;

    #[test]
    fn session_volume_average_mono() {
        let vol = SessionVolume::new(SessionId::MacEndpoint, vec![0.8], false);
        assert!((vol.average_volume() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn session_volume_average_stereo() {
        let vol = SessionVolume::new(SessionId::MacEndpoint, vec![0.6, 0.8], false);
        assert!((vol.average_volume() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn session_volume_average_empty() {
        let vol = SessionVolume::new(SessionId::MacEndpoint, vec![], false);
        assert!((vol.average_volume() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn session_volume_scaled() {
        let vol = SessionVolume::new(SessionId::MacEndpoint, vec![1.0, 0.8], false);
        let scaled = vol.scaled(0.5);
        assert!((scaled.channels[0] - 0.5).abs() < f32::EPSILON);
        assert!((scaled.channels[1] - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn volume_snapshot_is_empty() {
        let snapshot = VolumeSnapshot::new();
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.len(), 0);
    }

    #[test]
    fn volume_snapshot_push() {
        let mut snapshot = VolumeSnapshot::new();
        snapshot.push(SessionVolume::new(SessionId::MacEndpoint, vec![1.0], false));
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.len(), 1);
    }

    #[test]
    fn volume_snapshot_with_entries() {
        let entries = vec![
            SessionVolume::new(SessionId::MacEndpoint, vec![1.0], false),
            SessionVolume::new(SessionId::AlsaMaster, vec![0.8, 0.8], false),
        ];
        let snapshot = VolumeSnapshot::with_entries(entries);
        assert_eq!(snapshot.len(), 2);
    }

    #[test]
    fn session_id_wasapi_equality() {
        let id1 = SessionId::WasapiSession {
            pid: 1234,
            key: "test".to_string(),
        };
        let id2 = SessionId::WasapiSession {
            pid: 1234,
            key: "test".to_string(),
        };
        let id3 = SessionId::WasapiSession {
            pid: 5678,
            key: "test".to_string(),
        };
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn session_id_pulse_equality() {
        let id1 = SessionId::PulseSinkInput {
            index: 42,
            name: "firefox".to_string(),
        };
        let id2 = SessionId::PulseSinkInput {
            index: 42,
            name: "firefox".to_string(),
        };
        let id3 = SessionId::PulseSinkInput {
            index: 42,
            name: "chrome".to_string(),
        };
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}

mod windows_policy_tests {
    use super::*;

    #[test]
    fn wasapi_session_key_based_snapshot_restore_matching() {
        // Sessions should match by key, not by PID alone
        let snapshot = VolumeSnapshot::with_entries(vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "session-a".to_string(),
                },
                vec![0.8],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "session-b".to_string(),
                },
                vec![0.6],
                false,
            ),
        ]);

        // Verify we can find sessions by key
        let found = snapshot.entries.iter().find(|e| {
            matches!(&e.id, SessionId::WasapiSession { key, .. } if key == "session-a")
        });
        assert!(found.is_some());
        assert!((found.unwrap().channels[0] - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn wasapi_session_excluding_our_pid() {
        let our_pid = 12345u32;
        let sessions = vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: our_pid,
                    key: "self".to_string(),
                },
                vec![1.0],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 9999,
                    key: "other".to_string(),
                },
                vec![0.7],
                false,
            ),
        ];

        // Filter out our PID (mirrors the backend's enumeration logic)
        let duckable: Vec<_> = sessions
            .iter()
            .filter(|s| {
                !matches!(&s.id, SessionId::WasapiSession { pid, .. } if *pid == our_pid)
            })
            .collect();

        assert_eq!(duckable.len(), 1);
        assert!(
            matches!(&duckable[0].id, SessionId::WasapiSession { pid, .. } if *pid == 9999)
        );
    }

    #[test]
    fn wasapi_skipping_missing_sessions_on_restore() {
        // Snapshot had two sessions, but one disappeared
        let snapshot = VolumeSnapshot::with_entries(vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "alive".to_string(),
                },
                vec![0.8],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "gone".to_string(),
                },
                vec![0.6],
                false,
            ),
        ]);

        // Simulate live sessions: only "alive" still exists
        let live_keys = vec!["alive".to_string()];

        // Filter snapshot to only sessions that still exist
        let restorable: Vec<_> = snapshot
            .entries
            .iter()
            .filter(|e| {
                matches!(&e.id, SessionId::WasapiSession { key, .. } if live_keys.contains(key))
            })
            .collect();

        assert_eq!(restorable.len(), 1);
        assert!(
            matches!(&restorable[0].id, SessionId::WasapiSession { key, .. } if key == "alive")
        );
    }

    #[test]
    fn wasapi_empty_snapshot_is_valid() {
        // No active sessions should produce an empty snapshot
        let snapshot = VolumeSnapshot::new();
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.len(), 0);
    }
}

mod error_tests {
    use super::*;

    #[test]
    fn error_display_invalid_ramp() {
        let err = DuckingError::InvalidRampDuration(0);
        assert!(err.to_string().contains("ramp duration must be > 0"));
    }

    #[test]
    fn error_display_invalid_floor() {
        let err = DuckingError::InvalidFloorScalar(1.5);
        assert!(err.to_string().contains("floor scalar must be in range"));
    }

    #[test]
    fn error_display_snapshot_failed() {
        let err = DuckingError::SnapshotFailed("test error".to_string());
        assert!(err.to_string().contains("failed to snapshot"));
    }

    #[test]
    fn error_display_backend_unavailable() {
        let err = DuckingError::BackendUnavailable("no pulse".to_string());
        assert!(err.to_string().contains("backend unavailable"));
    }
}
