pub mod chat_completions;
pub mod config;

use std::sync::mpsc::Sender;

use crate::error::PostProcessingError;

use self::chat_completions::ChatCompletionsClient;
use self::config::{PostProcessorConfig, ProcessorName};

/// A single text post-processor backed by an LLM chat completions API.
pub struct PostProcessor {
    /// Human-readable name for progress display and error messages.
    name: ProcessorName,
    /// HTTP client for the chat completions API.
    client: ChatCompletionsClient,
}

impl PostProcessor {
    /// Create a post-processor from configuration.
    #[must_use]
    pub fn from_config(config: &PostProcessorConfig) -> Self {
        Self {
            name: config.name.clone(),
            client: ChatCompletionsClient::new(config),
        }
    }

    /// Get the processor name.
    #[must_use]
    pub fn name(&self) -> &ProcessorName {
        &self.name
    }

    /// Process text through the chat completions API.
    ///
    /// # Errors
    /// Returns `PostProcessingError` on network, auth, provider, or empty response failures.
    pub fn process(&self, text: &str) -> Result<String, PostProcessingError> {
        self.client.send(text)
    }
}

/// Result of running the processing pipeline.
#[derive(Debug)]
#[non_exhaustive]
pub enum PipelineResult {
    /// All processors succeeded; contains the final text.
    Processed {
        /// The processed text.
        text: String,
    },
    /// No processors configured; contains the original text.
    Skipped {
        /// The original text, unchanged.
        text: String,
    },
    /// A processor failed; contains the original text and error info.
    Failed {
        /// The original text before any processing.
        original_text: String,
        /// Name of the processor that failed.
        processor_name: String,
        /// The error that occurred.
        error: PostProcessingError,
    },
}

/// Progress messages sent from the pipeline to the UI during execution.
#[derive(Debug)]
#[non_exhaustive]
pub enum PipelineProgress {
    /// A processing step is about to begin.
    StepStarted {
        /// Zero-based index of the current step.
        index: usize,
        /// Total number of steps in the pipeline.
        total: usize,
        /// Human-readable name of the processor.
        name: String,
    },
    /// Pipeline completed successfully.
    Done {
        /// The final processed text.
        text: String,
    },
    /// Pipeline failed; contains fallback text.
    Failed {
        /// Name of the processor that failed.
        processor_name: String,
        /// Human-readable error description.
        error: String,
        /// Original text before any processing.
        original_text: String,
    },
}

/// An ordered sequence of post-processors that run sequentially.
pub struct ProcessingPipeline {
    processors: Vec<PostProcessor>,
}

impl ProcessingPipeline {
    /// Build a pipeline from a list of processor configs.
    #[must_use]
    pub fn from_configs(configs: &[PostProcessorConfig]) -> Self {
        let processors = configs.iter().map(PostProcessor::from_config).collect();
        Self { processors }
    }

    /// Returns `true` if the pipeline has no processors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    /// Run the pipeline, sending progress updates via the channel.
    ///
    /// Each processor runs sequentially. On failure, the pipeline stops
    /// and sends a `Failed` message with the original text.
    pub fn run(&self, text: &str, progress: &Sender<PipelineProgress>) {
        if self.processors.is_empty() {
            let _ = progress.send(PipelineProgress::Done {
                text: text.to_owned(),
            });
            return;
        }

        let original = text.to_owned();
        let mut current = text.to_owned();
        let total = self.processors.len();

        for (index, processor) in self.processors.iter().enumerate() {
            let _ = progress.send(PipelineProgress::StepStarted {
                index,
                total,
                name: processor.name().to_string(),
            });

            match processor.process(&current) {
                Ok(result) => {
                    current = result;
                }
                Err(e) => {
                    let _ = progress.send(PipelineProgress::Failed {
                        processor_name: processor.name().to_string(),
                        error: e.to_string(),
                        original_text: original,
                    });
                    return;
                }
            }
        }

        let _ = progress.send(PipelineProgress::Done { text: current });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: run a pipeline of mock transforms and collect progress messages.
    fn run_mock_pipeline(
        transforms: Vec<(&str, Result<String, PostProcessingError>)>,
        input: &str,
    ) -> Vec<PipelineProgress> {
        let (tx, rx) = std::sync::mpsc::channel();
        let total = transforms.len();

        // We can't use PostProcessor directly (it wraps a real HTTP client),
        // so we simulate the pipeline logic manually for unit tests.
        let original = input.to_owned();
        let mut current = input.to_owned();

        for (index, (name, result_fn)) in transforms.into_iter().enumerate() {
            let _ = tx.send(PipelineProgress::StepStarted {
                index,
                total,
                name: name.to_owned(),
            });

            match result_fn {
                Ok(text) => {
                    current = text;
                }
                Err(e) => {
                    let _ = tx.send(PipelineProgress::Failed {
                        processor_name: name.to_owned(),
                        error: e.to_string(),
                        original_text: original,
                    });
                    drop(tx);
                    return rx.into_iter().collect();
                }
            }
        }

        let _ = tx.send(PipelineProgress::Done { text: current });
        drop(tx);
        rx.into_iter().collect()
    }

    #[test]
    fn empty_pipeline_sends_done_with_original() {
        let (tx, rx) = std::sync::mpsc::channel();
        let pipeline = ProcessingPipeline {
            processors: Vec::new(),
        };
        pipeline.run("hello", &tx);
        drop(tx);

        let msgs: Vec<_> = rx.into_iter().collect();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0], PipelineProgress::Done { text } if text == "hello"));
    }

    #[test]
    fn pipeline_mock_three_steps_success() {
        let msgs = run_mock_pipeline(
            vec![
                ("Grammar", Ok("fixed grammar".to_owned())),
                ("Translate", Ok("translated".to_owned())),
                ("Format", Ok("formatted".to_owned())),
            ],
            "raw text",
        );

        // 3 StepStarted + 1 Done = 4
        assert_eq!(msgs.len(), 4);
        assert!(
            matches!(&msgs[0], PipelineProgress::StepStarted { index: 0, total: 3, name } if name == "Grammar")
        );
        assert!(
            matches!(&msgs[1], PipelineProgress::StepStarted { index: 1, total: 3, name } if name == "Translate")
        );
        assert!(
            matches!(&msgs[2], PipelineProgress::StepStarted { index: 2, total: 3, name } if name == "Format")
        );
        assert!(matches!(&msgs[3], PipelineProgress::Done { text } if text == "formatted"));
    }

    #[test]
    fn pipeline_mock_failure_at_step_2() {
        let msgs = run_mock_pipeline(
            vec![
                ("Grammar", Ok("fixed".to_owned())),
                ("Translate", Err(PostProcessingError::Timeout)),
            ],
            "original text",
        );

        // 2 StepStarted + 1 Failed = 3
        assert_eq!(msgs.len(), 3);
        assert!(
            matches!(&msgs[0], PipelineProgress::StepStarted { index: 0, total: 2, name } if name == "Grammar")
        );
        assert!(
            matches!(&msgs[1], PipelineProgress::StepStarted { index: 1, total: 2, name } if name == "Translate")
        );
        assert!(
            matches!(&msgs[2], PipelineProgress::Failed { processor_name, original_text, .. } if processor_name == "Translate" && original_text == "original text")
        );
    }
}
