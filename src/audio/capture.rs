use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleRate, Stream, StreamConfig};

use crate::error::RecordingError;
use crate::types::RmsLevel;

/// Configuration for audio capture.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Input device name (None = system default).
    pub device_name: Option<String>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Maximum recording duration.
    pub max_duration: Duration,
}

/// Handle to an active audio capture session.
///
/// Created by [`AudioCapture::start`]. The stream records
/// until [`AudioCapture::stop`] is called, returning the
/// captured audio samples.
pub struct AudioCapture {
    stream: Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    stream_error: Arc<AtomicBool>,
    sample_rate: u32,
    started_at: Instant,
    max_duration: Duration,
}

/// Raw captured audio data (PCM mono i16).
#[derive(Debug, Clone)]
pub struct CapturedAudio {
    /// Raw PCM samples (mono, i16).
    pub samples: Vec<i16>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Recording duration.
    pub duration: Duration,
}

/// Find an input device by name, or the system default.
fn find_device(name: Option<&str>) -> Result<Device, RecordingError> {
    let host = cpal::default_host();
    match name {
        Some(name) => {
            let devices = host
                .input_devices()
                .map_err(|e| RecordingError::DeviceError(e.to_string()))?;
            for device in devices {
                if device.name().ok().as_deref() == Some(name) {
                    return Ok(device);
                }
            }
            Err(RecordingError::DeviceError(format!(
                "Device not found: {name}"
            )))
        }
        None => host
            .default_input_device()
            .ok_or(RecordingError::NoMicrophone),
    }
}

impl AudioCapture {
    /// Start capturing audio from the specified device.
    ///
    /// Spawns a cpal input stream that collects mono i16 samples
    /// into an internal buffer. Call [`stop`](Self::stop) to end
    /// recording and retrieve the captured audio.
    ///
    /// # Errors
    /// Returns `RecordingError` if the device is unavailable or
    /// the stream cannot be created.
    pub fn start(config: &CaptureConfig) -> Result<Self, RecordingError> {
        let device = find_device(config.device_name.as_deref())?;

        let stream_config = StreamConfig {
            channels: 1,
            sample_rate: SampleRate(config.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);
        let stream_error = Arc::new(AtomicBool::new(false));
        let stream_error_clone = Arc::clone(&stream_error);

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buffer) = samples_clone.lock() {
                        buffer.extend_from_slice(data);
                    }
                },
                move |err| {
                    tracing::error!("Audio stream error: {err}");
                    stream_error_clone.store(true, Ordering::Relaxed);
                },
                None,
            )
            .map_err(|e| RecordingError::DeviceError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| RecordingError::DeviceError(e.to_string()))?;

        Ok(Self {
            stream,
            samples,
            stream_error,
            sample_rate: config.sample_rate,
            started_at: Instant::now(),
            max_duration: config.max_duration,
        })
    }

    /// Get the current RMS level of the most recent ~50ms of audio.
    #[must_use]
    pub fn current_rms(&self) -> RmsLevel {
        let samples = self
            .samples
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let window_size = (self.sample_rate as usize) / 20; // 50ms
        let recent = if samples.len() > window_size {
            &samples[samples.len() - window_size..]
        } else {
            &samples
        };
        calculate_rms(recent)
    }

    /// Get elapsed recording duration.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Check if max duration has been exceeded.
    #[must_use]
    pub fn is_max_duration_reached(&self) -> bool {
        self.started_at.elapsed() >= self.max_duration
    }

    /// Check if the audio stream encountered an error (e.g., device disconnected).
    #[must_use]
    pub fn has_stream_error(&self) -> bool {
        self.stream_error.load(Ordering::Relaxed)
    }

    /// Stop recording and return captured audio.
    #[must_use]
    pub fn stop(self) -> CapturedAudio {
        drop(self.stream);
        let duration = self.started_at.elapsed();
        let samples = match Arc::try_unwrap(self.samples) {
            Ok(mutex) => mutex.into_inner().unwrap_or_else(|e| {
                tracing::warn!("Audio sample mutex was poisoned, recovering samples");
                e.into_inner()
            }),
            Err(arc) => arc
                .lock()
                .unwrap_or_else(|e| {
                    tracing::warn!("Audio sample mutex was poisoned, recovering samples");
                    e.into_inner()
                })
                .clone(),
        };

        CapturedAudio {
            samples,
            sample_rate: self.sample_rate,
            duration,
        }
    }
}

/// Calculate RMS level from PCM samples.
pub(crate) fn calculate_rms(samples: &[i16]) -> RmsLevel {
    if samples.is_empty() {
        return RmsLevel::new(0.0);
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|&s| {
            let normalized = f64::from(s) / f64::from(i16::MAX);
            normalized * normalized
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let rms = (sum_sq / samples.len() as f64).sqrt();
    #[allow(clippy::cast_possible_truncation)]
    RmsLevel::new(rms as f32)
}
