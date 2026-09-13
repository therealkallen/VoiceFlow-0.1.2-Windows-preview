use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};
use shared_protocol::CapturedAudio;

use crate::host::{AudioCaptureAdapter, AudioPreparationSummary, AudioStartSummary};

#[derive(Default)]
pub struct CpalAudioCaptureAdapter {
    backend: Mutex<Option<Host>>,
    active_capture: Mutex<Option<ActiveCapture>>,
}

struct ActiveCapture {
    stream: Stream,
    sample_rate_hz: u32,
    channels: u16,
    samples: Arc<Mutex<Vec<f32>>>,
    stream_error: Arc<Mutex<Option<String>>>,
}

impl AudioCaptureAdapter for CpalAudioCaptureAdapter {
    fn snapshot_source(&self) -> Option<speech_engine::AudioSnapshot> {
        let capture = self.active_capture.lock().ok()?;
        let capture = capture.as_ref()?;
        let samples = Arc::clone(&capture.samples);
        let rate = capture.sample_rate_hz;
        let channels = capture.channels;
        Some(Box::new(move || {
            let samples = samples.lock().ok()?;
            // Do not copy short recordings, which retain the existing whole-clip path.
            if samples.len() <= rate as usize * channels as usize * 25 {
                return None;
            }
            let copy = samples.clone();
            drop(samples);
            Some(CapturedAudio::from_samples(rate, channels, copy))
        }))
    }

    fn prepare(&self) -> AudioPreparationSummary {
        let prepare_started_at = std::time::Instant::now();
        let backend_create_started_at = std::time::Instant::now();
        let mut backend = match self.backend.lock() {
            Ok(backend) => backend,
            Err(_) => {
                return AudioPreparationSummary {
                    prepare_ms: elapsed_ms(prepare_started_at),
                    prepare_succeeded: false,
                    ..AudioPreparationSummary::default()
                };
            }
        };
        if backend.is_none() {
            *backend = Some(cpal::default_host());
        }
        let backend_create_ms = elapsed_ms(backend_create_started_at);
        let device_discovery_started_at = std::time::Instant::now();
        let prepare_succeeded = backend
            .as_ref()
            .and_then(HostTrait::default_input_device)
            .and_then(|device| device.default_input_config().ok())
            .is_some();

        AudioPreparationSummary {
            prepare_ms: elapsed_ms(prepare_started_at),
            prepare_succeeded,
            backend_create_ms,
            device_discovery_ms: elapsed_ms(device_discovery_started_at),
        }
    }

    fn start(&self, _session_id: u64) -> Result<AudioStartSummary, String> {
        let total_started_at = std::time::Instant::now();
        let mut slot = self
            .active_capture
            .lock()
            .map_err(|_| "failed to lock audio capture state".to_string())?;
        if slot.is_some() {
            return Err("audio capture is already active".to_string());
        }

        let backend_create_started_at = std::time::Instant::now();
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| "failed to lock audio backend state".to_string())?;
        if backend.is_none() {
            *backend = Some(cpal::default_host());
        }
        let backend_create_ms = elapsed_ms(backend_create_started_at);
        let device_discovery_started_at = std::time::Instant::now();
        let device = backend
            .as_ref()
            .and_then(HostTrait::default_input_device)
            .ok_or_else(|| "no default input device is available".to_string())?;
        let supported_config = device
            .default_input_config()
            .map_err(|error| format!("failed to read default input config: {error}"))?;
        let device_discovery_ms = elapsed_ms(device_discovery_started_at);
        drop(backend);
        let stream_config: StreamConfig = supported_config.clone().into();
        let sample_rate_hz = stream_config.sample_rate.0;
        let channels = stream_config.channels;
        let samples = Arc::new(Mutex::new(Vec::new()));
        let stream_error = Arc::new(Mutex::new(None));

        let capture_start_started_at = std::time::Instant::now();
        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => build_input_stream_f32(
                &device,
                &stream_config,
                Arc::clone(&samples),
                Arc::clone(&stream_error),
            )?,
            SampleFormat::I16 => build_input_stream_i16(
                &device,
                &stream_config,
                Arc::clone(&samples),
                Arc::clone(&stream_error),
            )?,
            SampleFormat::U16 => build_input_stream_u16(
                &device,
                &stream_config,
                Arc::clone(&samples),
                Arc::clone(&stream_error),
            )?,
            other => {
                return Err(format!("unsupported input sample format: {other:?}"));
            }
        };

        stream
            .play()
            .map_err(|error| format!("failed to start input stream: {error}"))?;

        *slot = Some(ActiveCapture {
            stream,
            sample_rate_hz,
            channels,
            samples,
            stream_error,
        });

        Ok(AudioStartSummary {
            total_ms: elapsed_ms(total_started_at),
            backend_create_ms,
            device_discovery_ms,
            capture_start_ms: elapsed_ms(capture_start_started_at),
        })
    }

    fn stop(&self, _session_id: u64) -> Result<CapturedAudio, String> {
        let active_capture = self
            .active_capture
            .lock()
            .map_err(|_| "failed to lock audio capture state".to_string())?
            .take()
            .ok_or_else(|| "audio capture is not active".to_string())?;

        let ActiveCapture {
            stream,
            sample_rate_hz,
            channels,
            samples,
            stream_error,
        } = active_capture;

        drop(stream);

        if let Some(error) = stream_error
            .lock()
            .map_err(|_| "failed to lock audio error state".to_string())?
            .take()
        {
            return Err(error);
        }

        let captured_samples = samples
            .lock()
            .map_err(|_| "failed to lock captured audio samples".to_string())?
            .clone();

        Ok(CapturedAudio::from_samples(
            sample_rate_hz,
            channels,
            captured_samples,
        ))
    }
}

fn elapsed_ms(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn build_input_stream_f32(
    device: &Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    stream_error: Arc<Mutex<Option<String>>>,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend_from_slice(data);
                }
            },
            move |error| {
                if let Ok(mut slot) = stream_error.lock() {
                    *slot = Some(format!("input stream error: {error}"));
                }
            },
            None,
        )
        .map_err(|error| format!("failed to build f32 input stream: {error}"))
}

fn build_input_stream_i16(
    device: &Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    stream_error: Arc<Mutex<Option<String>>>,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[i16], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend(data.iter().map(|sample| *sample as f32 / i16::MAX as f32));
                }
            },
            move |error| {
                if let Ok(mut slot) = stream_error.lock() {
                    *slot = Some(format!("input stream error: {error}"));
                }
            },
            None,
        )
        .map_err(|error| format!("failed to build i16 input stream: {error}"))
}

fn build_input_stream_u16(
    device: &Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    stream_error: Arc<Mutex<Option<String>>>,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[u16], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend(
                        data.iter()
                            .map(|sample| (*sample as f32 - 32768.0) / 32768.0),
                    );
                }
            },
            move |error| {
                if let Ok(mut slot) = stream_error.lock() {
                    *slot = Some(format!("input stream error: {error}"));
                }
            },
            None,
        )
        .map_err(|error| format!("failed to build u16 input stream: {error}"))
}
