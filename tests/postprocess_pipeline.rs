use std::sync::mpsc;

use voice_type::config::Secret;
use voice_type::error::PostProcessingError;
use voice_type::postprocess::config::{PostProcessorConfig, ProcessorName};
use voice_type::postprocess::{PipelineProgress, ProcessingPipeline};

/// Helper: build a config pointing at a non-routable endpoint so it fails fast.
fn dummy_config(name: &str, system_prompt: &str) -> PostProcessorConfig {
    PostProcessorConfig {
        name: ProcessorName::new(name).unwrap(),
        system_prompt: system_prompt.to_owned(),
        api_key: Secret::from_string("test-key"),
        model: "gpt-4o-mini".to_owned(),
        endpoint: "http://192.0.2.1".to_owned(), // TEST-NET, non-routable
        timeout: std::time::Duration::from_millis(200),
        temperature: Some(0.3),
        max_tokens: Some(1024),
    }
}

#[test]
fn empty_pipeline_from_configs() {
    let pipeline = ProcessingPipeline::from_configs(&[]);
    assert!(pipeline.is_empty());

    let (tx, rx) = mpsc::channel();
    pipeline.run("hello world", &tx);
    drop(tx);

    let msgs: Vec<_> = rx.into_iter().collect();
    assert_eq!(msgs.len(), 1);
    assert!(matches!(
        &msgs[0],
        PipelineProgress::Done { text } if text == "hello world"
    ));
}

#[test]
fn pipeline_construction_from_configs() {
    let configs = vec![
        dummy_config("Grammar", "Fix grammar."),
        dummy_config("Translate", "Translate to English."),
    ];
    let pipeline = ProcessingPipeline::from_configs(&configs);
    assert!(!pipeline.is_empty());
}

#[test]
fn single_processor_network_failure_reports_name() {
    let configs = vec![dummy_config("Grammar", "Fix grammar.")];
    let pipeline = ProcessingPipeline::from_configs(&configs);

    let (tx, rx) = mpsc::channel();
    pipeline.run("test input", &tx);
    drop(tx);

    let msgs: Vec<_> = rx.into_iter().collect();

    // Should have StepStarted + Failed
    assert!(
        msgs.len() >= 2,
        "Expected at least 2 messages, got {}",
        msgs.len()
    );

    // First message: StepStarted for "Grammar"
    assert!(matches!(
        &msgs[0],
        PipelineProgress::StepStarted { index: 0, total: 1, name } if name == "Grammar"
    ));

    // Last message: Failed with processor name and original text
    let last = msgs.last().unwrap();
    match last {
        PipelineProgress::Failed {
            processor_name,
            original_text,
            error,
        } => {
            assert_eq!(processor_name, "Grammar");
            assert_eq!(original_text, "test input");
            assert!(!error.is_empty(), "Error message should not be empty");
        }
        _ => panic!("Expected PipelineProgress::Failed, got {last:?}"),
    }
}

#[test]
fn multi_processor_failure_at_step1_preserves_original() {
    let configs = vec![
        dummy_config("Grammar", "Fix grammar."),
        dummy_config("Translate", "Translate to English."),
        dummy_config("Format", "Format nicely."),
    ];
    let pipeline = ProcessingPipeline::from_configs(&configs);

    let (tx, rx) = mpsc::channel();
    pipeline.run("original text here", &tx);
    drop(tx);

    let msgs: Vec<_> = rx.into_iter().collect();

    // First processor should fail (non-routable endpoint), so we get:
    // StepStarted(Grammar) + Failed(Grammar)
    assert!(msgs.len() >= 2);
    assert!(matches!(
        &msgs[0],
        PipelineProgress::StepStarted { index: 0, total: 3, name } if name == "Grammar"
    ));

    let last = msgs.last().unwrap();
    match last {
        PipelineProgress::Failed {
            processor_name,
            original_text,
            ..
        } => {
            assert_eq!(processor_name, "Grammar");
            assert_eq!(original_text, "original text here");
        }
        _ => panic!("Expected PipelineProgress::Failed, got {last:?}"),
    }
}

#[test]
fn pipeline_progress_protocol_via_channel() {
    // Verify the progress message protocol by manually sending messages
    // through a channel, simulating what a real pipeline would do.
    let (tx, rx) = mpsc::channel();

    // Simulate a 3-step pipeline completing successfully
    tx.send(PipelineProgress::StepStarted {
        index: 0,
        total: 3,
        name: "Grammar".to_owned(),
    })
    .unwrap();
    tx.send(PipelineProgress::StepStarted {
        index: 1,
        total: 3,
        name: "Translate".to_owned(),
    })
    .unwrap();
    tx.send(PipelineProgress::StepStarted {
        index: 2,
        total: 3,
        name: "Format".to_owned(),
    })
    .unwrap();
    tx.send(PipelineProgress::Done {
        text: "final output".to_owned(),
    })
    .unwrap();
    drop(tx);

    let msgs: Vec<_> = rx.into_iter().collect();
    assert_eq!(msgs.len(), 4);
    assert!(matches!(&msgs[3], PipelineProgress::Done { text } if text == "final output"));
}

#[test]
fn pipeline_progress_failure_protocol() {
    // Simulate a pipeline that fails at step 2
    let (tx, rx) = mpsc::channel();

    tx.send(PipelineProgress::StepStarted {
        index: 0,
        total: 2,
        name: "Grammar".to_owned(),
    })
    .unwrap();
    tx.send(PipelineProgress::StepStarted {
        index: 1,
        total: 2,
        name: "Translate".to_owned(),
    })
    .unwrap();
    tx.send(PipelineProgress::Failed {
        processor_name: "Translate".to_owned(),
        error: PostProcessingError::AuthenticationError.to_string(),
        original_text: "raw transcription".to_owned(),
    })
    .unwrap();
    drop(tx);

    let msgs: Vec<_> = rx.into_iter().collect();
    assert_eq!(msgs.len(), 3);

    match &msgs[2] {
        PipelineProgress::Failed {
            processor_name,
            error,
            original_text,
        } => {
            assert_eq!(processor_name, "Translate");
            assert!(error.contains("authentication"));
            assert_eq!(original_text, "raw transcription");
        }
        _ => panic!("Expected Failed"),
    }
}
