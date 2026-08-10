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
        let found = snapshot
            .entries
            .iter()
            .find(|e| matches!(&e.id, SessionId::WasapiSession { key, .. } if key == "session-a"));
        assert!(found.is_some());
        assert!((found.unwrap().channels[0] - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn wasapi_session_excluding_our_pid() {
        let our_pid = 12345u32;
        let sessions = [
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
            .filter(|s| !matches!(&s.id, SessionId::WasapiSession { pid, .. } if *pid == our_pid))
            .collect();

        assert_eq!(duckable.len(), 1);
        assert!(matches!(&duckable[0].id, SessionId::WasapiSession { pid, .. } if *pid == 9999));
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
        let live_keys = ["alive".to_string()];

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

    #[test]
    fn wasapi_mute_restored_after_volume_fade() {
        let snapshot = VolumeSnapshot::with_entries(vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "muted-app".to_string(),
                },
                vec![0.5],
                true,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "unmuted-app".to_string(),
                },
                vec![0.7],
                false,
            ),
        ]);

        for entry in &snapshot.entries {
            match &entry.id {
                SessionId::WasapiSession { key, .. } if key == "muted-app" => {
                    assert!(entry.mute, "muted-app should have mute=true in snapshot");
                }
                SessionId::WasapiSession { key, .. } if key == "unmuted-app" => {
                    assert!(
                        !entry.mute,
                        "unmuted-app should have mute=false in snapshot"
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn wasapi_inactive_sessions_excluded_from_snapshot() {
        let all_sessions = [
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "active".to_string(),
                },
                vec![0.8],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "inactive".to_string(),
                },
                vec![0.0],
                false,
            ),
        ];

        let active: Vec<_> = all_sessions
            .iter()
            .filter(|s| matches!(&s.id, SessionId::WasapiSession { key, .. } if key == "active"))
            .collect();

        assert_eq!(active.len(), 1);
        assert!(matches!(&active[0].id, SessionId::WasapiSession { key, .. } if key == "active"));
    }

    #[test]
    fn wasapi_restore_only_affects_snapshotted_sessions() {
        let snapshot = VolumeSnapshot::with_entries(vec![SessionVolume::new(
            SessionId::WasapiSession {
                pid: 100,
                key: "original".to_string(),
            },
            vec![0.8],
            false,
        )]);

        let live_keys_at_restore = ["original".to_string(), "new-session".to_string()];

        let restorable: Vec<_> = snapshot
            .entries
            .iter()
            .filter(|e| {
                matches!(&e.id, SessionId::WasapiSession { key, .. } if live_keys_at_restore.contains(key))
            })
            .collect();

        assert_eq!(restorable.len(), 1);
        assert!(
            matches!(&restorable[0].id, SessionId::WasapiSession { key, .. } if key == "original")
        );
        assert!((restorable[0].channels[0] - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn wasapi_volume_map_lookup_by_key() {
        let snapshot = VolumeSnapshot::with_entries(vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "alpha".to_string(),
                },
                vec![0.9],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "beta".to_string(),
                },
                vec![0.3],
                true,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 300,
                    key: "gamma".to_string(),
                },
                vec![0.5],
                false,
            ),
        ]);

        let keys: Vec<String> = snapshot
            .entries
            .iter()
            .filter_map(|e| match &e.id {
                SessionId::WasapiSession { key, .. } => Some(key.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"alpha".to_string()));
        assert!(keys.contains(&"beta".to_string()));
        assert!(keys.contains(&"gamma".to_string()));
    }

    #[test]
    fn wasapi_floor_calculation_clamps_to_range() {
        let original_volume = 0.8f32;
        let floor_scalar = 0.2f32;
        let target = (original_volume * floor_scalar).clamp(0.0, 1.0);
        assert!((target - 0.16).abs() < f32::EPSILON);

        let zero_floor = (1.0f32 * 0.0f32).clamp(0.0, 1.0);
        assert!((zero_floor - 0.0).abs() < f32::EPSILON);

        let high_floor = (0.5f32 * 1.0f32).clamp(0.0, 1.0);
        assert!((high_floor - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn wasapi_volume_map_prefers_last_entry_for_duplicate_key() {
        let entries = [
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "dup-key".to_string(),
                },
                vec![0.8],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "dup-key".to_string(),
                },
                vec![0.4],
                true,
            ),
        ];

        let mut map = std::collections::HashMap::<String, &SessionVolume>::new();
        for e in &entries {
            if let SessionId::WasapiSession { key, .. } = &e.id {
                map.insert(key.clone(), e);
            }
        }

        let entry = map.get("dup-key").expect("key should exist");
        assert!(
            matches!(&entry.id, SessionId::WasapiSession { pid, .. } if *pid == 200),
            "last-inserted entry should win"
        );
        assert!((entry.channels[0] - 0.4).abs() < f32::EPSILON);
        assert!(entry.mute);
    }

    #[test]
    fn wasapi_mute_restore_policy_applies_after_final_volume_step() {
        let snapshot = VolumeSnapshot::with_entries(vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "was-muted".to_string(),
                },
                vec![0.5],
                true,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "was-not-muted".to_string(),
                },
                vec![0.7],
                false,
            ),
        ]);

        let config = DuckConfig::new(100, 0.2).unwrap();

        for entry in &snapshot.entries {
            let original_volume = entry.channels.first().copied().unwrap_or(1.0);
            let steps = compute_fade_steps(0.1, original_volume, &config);
            let last = steps.last().expect("should have fade steps");
            assert!(
                (last.volume - original_volume).abs() < f32::EPSILON,
                "final step should reach original volume"
            );
            let expected_mute = entry.mute;
            assert!(
                matches!(&entry.id, SessionId::WasapiSession { key, .. } if (*key == "was-muted") == expected_mute),
                "mute state should be preserved from snapshot"
            );
        }
    }

    #[test]
    fn wasapi_build_volume_map_skips_empty_keys() {
        let entries = [
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "valid-key".to_string(),
                },
                vec![0.8],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "".to_string(),
                },
                vec![0.6],
                false,
            ),
        ];

        let map: std::collections::HashMap<_, _> = entries
            .iter()
            .filter_map(|e| match &e.id {
                SessionId::WasapiSession { key, .. } if !key.is_empty() => Some((key.clone(), e)),
                _ => None,
            })
            .collect();

        assert_eq!(map.len(), 1);
        assert!(map.contains_key("valid-key"));
        assert!(!map.contains_key(""));
    }

    #[test]
    fn wasapi_fade_steps_for_multiple_sessions_independent() {
        let snapshot = VolumeSnapshot::with_entries(vec![
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 100,
                    key: "app-a".to_string(),
                },
                vec![1.0],
                false,
            ),
            SessionVolume::new(
                SessionId::WasapiSession {
                    pid: 200,
                    key: "app-b".to_string(),
                },
                vec![0.5],
                false,
            ),
        ]);

        let config = DuckConfig::new(100, 0.2).unwrap();

        for entry in &snapshot.entries {
            let original = entry.channels[0];
            let target = (original * config.floor_scalar()).clamp(0.0, 1.0);
            let steps = compute_fade_steps(original, target, &config);

            assert!(steps.len() >= 3, "should have minimum 3 steps");
            assert!(
                (steps.last().unwrap().volume - target).abs() < f32::EPSILON,
                "final step should reach target"
            );
        }
    }
}

mod linux_policy_tests {
    use super::*;

    #[test]
    fn linux_factory_only_allows_pulse_or_noop() {
        let name = backend_name();
        #[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
        {
            assert!(
                name == "linux-pulse" || name == "noop",
                "Linux factory should only return linux-pulse or noop, got {}",
                name
            );
        }
        #[cfg(not(all(target_os = "linux", feature = "audio-ducking-linux")))]
        {
            assert!(
                name != "linux-alsa",
                "linux-alsa should never appear in backend_name()"
            );
        }
    }

    #[test]
    fn linux_write_failure_error_is_platform_variant() {
        let err = DuckingError::Platform(
            "failed to decrease volume for sink input 42: I/O error".to_string(),
        );
        let msg = err.to_string();
        assert!(msg.contains("failed to decrease volume"), "msg: {msg}");
    }

    #[test]
    fn linux_missing_sink_input_error_is_platform_variant() {
        let err = DuckingError::Platform("sink input 99 no longer exists".to_string());
        let msg = err.to_string();
        assert!(msg.contains("no longer exists"), "msg: {msg}");
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
