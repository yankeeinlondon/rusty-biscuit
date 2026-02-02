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
