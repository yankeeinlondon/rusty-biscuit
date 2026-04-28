            panic!("expected Structured");
        };
        assert_eq!(record.tags.get("provider-id").unwrap(), "zai");
        assert_eq!(record.tags.get("session.id").unwrap(), "s1");
        assert_eq!(record.tags.get("model").unwrap(), "k2p6 message");
        assert_eq!(record.message, "");
    }

    #[test]
    fn rate_limit_without_retry_error_is_warning_even_before_stdout() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        
        // This is a 1308 but NOT wrapped in AI_RetryError
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached.\"}}"}}"#;
        
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, .. } => {
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        
        assert!(
            rx.try_recv().is_err(),
            "early-termination signal NOT expected for non-fatal rate limit",
        );
    }
