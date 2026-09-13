use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use shared_protocol::{AsrDiagnostics, AsrLatencyMetric, CapturedAudio, EngineRequest};

use crate::{Transcriber, TranscriberPreparation, TranscriptionOutput};

#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const TARGET_SAMPLE_RATE_HZ: usize = 16_000;
const SEGMENTATION_THRESHOLD_MS: usize = 25_000;
const PROTECTED_HEAD_MS: usize = 15_000;
const VAD_FRAME_MS: usize = 20;
const VAD_MIN_RMS: f32 = 0.0025;
const VAD_MAX_RMS: f32 = 0.006;
const VAD_AUDIO_RMS_MULTIPLIER: f32 = 0.5;
const MIN_VOICED_RUN_MS: usize = 60;
const MERGE_GAP_MS: usize = 900;
const PRE_PADDING_MS: usize = 300;
const POST_PADDING_MS: usize = 500;
const PREFERRED_CHUNK_MS: usize = 20_000;
const MAX_CHUNK_MS: usize = 25_000;
const HARD_CUT_OVERLAP_MS: usize = 400;
const SHORT_TRIM_PRE_PADDING_MS: usize = 300;
const SHORT_TRIM_POST_PADDING_MS: usize = 500;
const SHORT_TRIM_MIN_SPEECH_MS: usize = 120;
const ASR_DEBUG_TRANSCRIPTS_ENV: &str = "VOICEFLOW_ASR_DEBUG_TRANSCRIPTS";
const WARMUP_AUDIO_MS: usize = 250;

#[derive(Clone, Debug)]
pub struct LocalAsrWorkerConfig {
    pub python_executable: PathBuf,
    pub worker_script: PathBuf,
    pub debug_script: PathBuf,
    pub startup_timeout: Duration,
    pub request_timeout: Duration,
    pub max_restart_attempts: u8,
}

impl Default for LocalAsrWorkerConfig {
    fn default() -> Self {
        let root = env::current_exe().ok().and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        let worker_script = env::var_os("VOICEFLOW_ASR_WORKER").filter(|value| !value.is_empty())
            .map(PathBuf::from).unwrap_or_else(|| root.join("runtime/asr/worker.py"));
        Self {
            python_executable: env::var_os("VOICEFLOW_ASR_PYTHON").filter(|value| !value.is_empty())
                .map(PathBuf::from).unwrap_or_else(|| root.join("runtime/python/python.exe")),
            debug_script: worker_script.clone(),
            worker_script,
            startup_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
            max_restart_attempts: 1,
        }
    }
}

#[derive(Default)]
pub struct LocalAsrWorkerTranscriber {
    config: LocalAsrWorkerConfig,
    process: Mutex<Option<WorkerProcess>>,
    next_request_id: AtomicU64,
    recording_cache: Mutex<Option<(u64, Vec<CachedChunk>)>>,
    normalization_cache: Mutex<Option<(u64, NormalizedPrefix)>>,
}

struct NormalizedPrefix {
    rate: u32,
    channels: u16,
    source: Vec<f32>,
    normalized: Vec<f32>,
}

pub type AudioSnapshot = Box<dyn Fn() -> Option<CapturedAudio> + Send>;

/// Stops speculative work and joins before final recognition or session teardown.
pub struct RecordingRecognition {
    stop: mpsc::Sender<()>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for RecordingRecognition {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct CachedChunk {
    range: AudioRange,
    samples: Vec<f32>,
    result: WorkerAttemptResult,
}

struct WorkerProcess {
    child: Child,
    stdin: ChildStdin,
    receiver: Receiver<Result<WorkerMessage, String>>,
    model_load_ms: Option<u32>,
}

#[derive(Serialize)]
struct WorkerRequest<'a> {
    cmd: &'a str,
    id: u64,
    wav_path: &'a str,
}

#[derive(Serialize)]
struct ShutdownRequest<'a> {
    cmd: &'a str,
}

#[derive(Serialize)]
struct PingRequest<'a> {
    cmd: &'a str,
    id: u64,
}

#[derive(Deserialize)]
struct WorkerMessage {
    #[serde(rename = "type")]
    message_type: String,
    ok: bool,
    id: Option<u64>,
    transcript: Option<String>,
    #[serde(default)]
    tokens: Vec<String>,
    #[serde(default)]
    timestamps: Vec<f64>,
    error: Option<String>,
    model_load_ms: Option<f64>,
    timings: Option<HashMap<String, f64>>,
}

struct WorkerAttemptResult {
    transcript: String,
    tokens: Vec<String>,
    timestamps: Vec<f64>,
    diagnostics: AsrDiagnostics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WarmupAttempt {
    elapsed_ms: u32,
    succeeded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChunkSource {
    WholeClip,
    ProtectedHead,
    VadRegion,
    HardCut,
}

impl ChunkSource {
    fn label(self) -> &'static str {
        match self {
            Self::WholeClip => "whole_clip",
            Self::ProtectedHead => "protected_head",
            Self::VadRegion => "vad_region",
            Self::HardCut => "hard_cut",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlannedChunk {
    range: AudioRange,
    source: ChunkSource,
}

#[derive(Clone, Debug, PartialEq)]
struct SegmentationPlan {
    vad_enabled: bool,
    vad_threshold: f32,
    short_trim: ShortTrimDiagnostics,
    chunks: Vec<PlannedChunk>,
    hard_cut_count: usize,
    dropped_region_count: usize,
    dropped_region_total_samples: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct ShortTrimDiagnostics {
    enabled: bool,
    input_samples: usize,
    output_samples: usize,
    removed_leading_samples: usize,
    removed_trailing_samples: usize,
    threshold: f32,
    fallback_reason: Option<&'static str>,
}

impl ShortTrimDiagnostics {
    fn fallback(input_samples: usize, threshold: f32, reason: &'static str) -> Self {
        Self {
            enabled: false,
            input_samples,
            output_samples: input_samples,
            removed_leading_samples: 0,
            removed_trailing_samples: 0,
            threshold,
            fallback_reason: Some(reason),
        }
    }
}

#[derive(Default)]
struct VoicedRegionDetection {
    regions: Vec<AudioRange>,
    dropped_region_count: usize,
    dropped_region_total_samples: usize,
}

impl LocalAsrWorkerTranscriber {
    pub fn new(config: LocalAsrWorkerConfig) -> Self {
        Self {
            config,
            process: Mutex::new(None),
            next_request_id: AtomicU64::new(1),
            recording_cache: Mutex::new(None),
            normalization_cache: Mutex::new(None),
        }
    }

    fn transcribe_via_worker(
        &self,
        request: &EngineRequest,
    ) -> Result<TranscriptionOutput, String> {
        if request.captured_audio.sample_count == 0 {
            return Ok(TranscriptionOutput {
                transcript: request.transcript_hint.clone().unwrap_or_else(|| {
                    format!(
                        "stub transcript from {} ms of captured audio",
                        request.captured_audio.duration_ms
                    )
                }),
                diagnostics: None,
            });
        }

        let host_total_started_at = Instant::now();
        let audio_prepare_started_at = Instant::now();
        let prefix = self
            .normalization_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.take())
            .filter(|(session, _)| *session == request.session_id)
            .map(|(_, prefix)| prefix);
        let (normalized, reused_samples) = normalize_with_prefix(&request.captured_audio, prefix);
        eprintln!(
            "[voiceflow-asr] normalization_reused_ms={}",
            samples_to_ms(reused_samples)
        );
        if normalized.is_empty() {
            return Err("captured audio contained no samples after normalization".to_string());
        }
        let plan = plan_audio_chunks(&normalized);
        let host_audio_prepare_ms = elapsed_u32(audio_prepare_started_at);
        log_segmentation_plan(&normalized, &plan);
        if plan.chunks.is_empty() {
            return Ok(TranscriptionOutput {
                transcript: String::new(),
                diagnostics: None,
            });
        }

        let debug_transcripts = debug_transcript_logging_enabled();
        let mut cached = self
            .recording_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.take())
            .filter(|(session, _)| *session == request.session_id)
            .map(|(_, chunks)| chunks)
            .unwrap_or_default();
        let mut reused_chunks = 0_u32;
        let mut prefetched_worker_ms = 0_u32;
        let mut host_wav_write_ms = 0_u32;
        let mut host_worker_roundtrip_ms = 0_u32;
        let mut host_temp_cleanup_ms = 0_u32;
        let attempts =
            transcribe_chunks(&normalized, &plan.chunks, |index, chunk, chunk_samples| {
                if let Some(position) = cached
                    .iter()
                    .position(|entry| entry.range == chunk.range && entry.samples == chunk_samples)
                {
                    let mut result = cached.swap_remove(position).result;
                    reused_chunks += 1;
                    // Foreground ASR timings must not count work done during recording.
                    for metric in &mut result.diagnostics.metrics {
                        if metric.name == "total_ms" {
                            prefetched_worker_ms =
                                prefetched_worker_ms.saturating_add(metric.value_ms);
                        }
                        metric.value_ms = 0;
                    }
                    return Ok(result);
                }
                let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
                let wav_write_started_at = Instant::now();
                let wav_path = write_temp_wav_samples(
                    chunk_samples,
                    request.session_id,
                    request_id,
                    &self.config,
                )?;
                host_wav_write_ms =
                    host_wav_write_ms.saturating_add(elapsed_u32(wav_write_started_at));
                let wav_path_string = wav_path.to_string_lossy().to_string();
                let started_at = Instant::now();
                let result = self.transcribe_with_restarts(request_id, &wav_path_string);
                let asr_time_ms = started_at.elapsed().as_millis();
                host_worker_roundtrip_ms =
                    host_worker_roundtrip_ms.saturating_add(elapsed_u32(started_at));
                let cleanup_started_at = Instant::now();
                let _ = fs::remove_file(&wav_path);
                host_temp_cleanup_ms =
                    host_temp_cleanup_ms.saturating_add(elapsed_u32(cleanup_started_at));

                match &result {
                    Ok(attempt) => log_chunk_result(
                        index,
                        plan.chunks.len(),
                        chunk,
                        asr_time_ms,
                        Some(&attempt.transcript),
                        debug_transcripts,
                    ),
                    Err(_) => {
                        log_chunk_result(index, plan.chunks.len(), chunk, asr_time_ms, None, false)
                    }
                }

                result
            })?;

        let transcript = merge_chunk_transcripts(&plan.chunks, &attempts)?;
        let mut diagnostics = merge_chunk_diagnostics(&attempts);
        if let Some(diagnostics) = &mut diagnostics {
            append_latency_metric(diagnostics, "prefetched_worker_ms", prefetched_worker_ms);
            append_latency_metric(diagnostics, "host_audio_prepare_ms", host_audio_prepare_ms);
            append_latency_metric(diagnostics, "host_wav_write_ms", host_wav_write_ms);
            append_latency_metric(
                diagnostics,
                "host_worker_roundtrip_ms",
                host_worker_roundtrip_ms,
            );
            append_latency_metric(diagnostics, "host_temp_cleanup_ms", host_temp_cleanup_ms);
            append_latency_metric(
                diagnostics,
                "host_total_ms",
                elapsed_u32(host_total_started_at),
            );
        }

        eprintln!(
            "[voiceflow-asr] prefetched_chunk_count={} final_chunk_count={} prefetched_worker_ms={}",
            reused_chunks,
            plan.chunks.len(),
            prefetched_worker_ms
        );

        Ok(TranscriptionOutput {
            transcript,
            diagnostics,
        })
    }

    fn transcribe_with_restarts(
        &self,
        request_id: u64,
        wav_path: &str,
    ) -> Result<WorkerAttemptResult, String> {
        let attempts = self.config.max_restart_attempts.saturating_add(1);
        let mut last_error = None;

        for attempt_index in 0..attempts {
            let restart_expected = attempt_index > 0;
            match self.transcribe_once(request_id, wav_path, restart_expected) {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "worker transcription failed".to_string()))
    }

    fn transcribe_once(
        &self,
        request_id: u64,
        wav_path: &str,
        restart_expected: bool,
    ) -> Result<WorkerAttemptResult, String> {
        let mut process_slot = self
            .process
            .lock()
            .map_err(|_| "failed to lock local ASR worker state".to_string())?;

        if restart_expected {
            shutdown_worker(process_slot.take());
        }

        let process = ensure_worker(&self.config, &mut process_slot)?;

        if let Some(status) = process
            .child
            .try_wait()
            .map_err(|error| format!("failed to poll worker status: {error}"))?
        {
            shutdown_worker(process_slot.take());
            return Err(format!("worker exited unexpectedly with status {status}"));
        }

        let message = WorkerRequest {
            cmd: "transcribe",
            id: request_id,
            wav_path,
        };
        send_request(&mut process.stdin, &message)?;
        let response = read_message(&process.receiver, self.config.request_timeout)?;

        if response.message_type != "transcript" {
            shutdown_worker(process_slot.take());
            return Err(format!(
                "unexpected worker response type `{}`",
                response.message_type
            ));
        }

        if response.id != Some(request_id) {
            shutdown_worker(process_slot.take());
            return Err(format!(
                "worker response id mismatch: expected {}, got {:?}",
                request_id, response.id
            ));
        }

        if !response.ok {
            let error = response
                .error
                .unwrap_or_else(|| "worker returned unknown transcription error".to_string());
            shutdown_worker(process_slot.take());
            return Err(error);
        }

        let transcript = response
            .transcript
            .ok_or_else(|| "worker returned no transcript".to_string())?;

        Ok(WorkerAttemptResult {
            transcript,
            tokens: response.tokens,
            timestamps: response.timestamps,
            diagnostics: AsrDiagnostics {
                backend: "sensevoice-worker".to_string(),
                worker_request_id: Some(request_id),
                worker_model_load_ms: process.model_load_ms,
                worker_restarted: restart_expected,
                metrics: parse_latency_metrics(response.timings),
            },
        })
    }

    fn prepare_worker(&self) -> Result<TranscriberPreparation, String> {
        let ping_request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let mut process_slot = self
            .process
            .lock()
            .map_err(|_| "failed to lock local ASR worker state".to_string())?;

        let mut cold_start = process_slot.is_none();
        if let Some(process) = process_slot.as_mut() {
            if process
                .child
                .try_wait()
                .map_err(|error| format!("failed to poll worker status: {error}"))?
                .is_some()
            {
                shutdown_worker(process_slot.take());
                cold_start = true;
            }
        }

        let process = ensure_worker(&self.config, &mut process_slot)?;
        send_request(
            &mut process.stdin,
            &PingRequest {
                cmd: "ping",
                id: ping_request_id,
            },
        )?;
        let response = read_message(&process.receiver, self.config.request_timeout)?;

        if response.message_type != "pong" {
            shutdown_worker(process_slot.take());
            return Err(format!(
                "unexpected worker ping response type `{}`",
                response.message_type
            ));
        }

        if response.id != Some(ping_request_id) {
            shutdown_worker(process_slot.take());
            return Err(format!(
                "worker ping response id mismatch: expected {}, got {:?}",
                ping_request_id, response.id
            ));
        }

        if !response.ok {
            let error = response
                .error
                .unwrap_or_else(|| "worker returned unknown ping error".to_string());
            shutdown_worker(process_slot.take());
            return Err(error);
        }

        let worker_model_load_ms = process.model_load_ms;
        let warmup_request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let warmup = cold_start
            .then(|| attempt_warmup(|| warm_up_worker(process, &self.config, warmup_request_id)));
        if matches!(
            warmup,
            Some(WarmupAttempt {
                succeeded: false,
                ..
            })
        ) {
            shutdown_worker(process_slot.take());
        }

        Ok(TranscriberPreparation {
            backend: "sensevoice-worker".to_string(),
            cold_start,
            worker_model_load_ms,
            worker_model_warmup_ms: warmup.map(|attempt| attempt.elapsed_ms),
            worker_model_warmup_succeeded: warmup.map(|attempt| attempt.succeeded),
        })
    }
}

impl Transcriber for LocalAsrWorkerTranscriber {
    fn start_recording(
        self: Arc<Self>,
        session_id: u64,
        snapshot: AudioSnapshot,
    ) -> Option<RecordingRecognition> {
        *self.recording_cache.lock().ok()? = Some((session_id, Vec::new()));
        *self.normalization_cache.lock().ok()? = None;
        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("asr-prefetch".into())
            .spawn(move || {
                while matches!(
                    receiver.recv_timeout(Duration::from_secs(1)),
                    Err(RecvTimeoutError::Timeout)
                ) {
                    let Some(audio) = snapshot() else { continue };
                    if audio.duration_ms <= SEGMENTATION_THRESHOLD_MS as u64 {
                        continue;
                    }
                    let prefix = self
                        .normalization_cache
                        .lock()
                        .ok()
                        .and_then(|mut cache| cache.take())
                        .filter(|(session, _)| *session == session_id)
                        .map(|(_, prefix)| prefix);
                    let (normalized, _) = normalize_with_prefix(&audio, prefix);
                    if let Ok(mut cache) = self.normalization_cache.lock() {
                        *cache = Some((
                            session_id,
                            NormalizedPrefix {
                                rate: audio.sample_rate_hz,
                                channels: audio.channels,
                                source: audio.samples,
                                normalized: normalized.clone(),
                            },
                        ));
                    }
                    let plan = plan_audio_chunks(&normalized);
                    // Only a naturally closed tail is eligible; no new cuts are introduced.
                    // Final recognition still validates exact ranges AND samples.
                    for chunk in prefetch_candidates(&normalized, &plan) {
                        if receiver.try_recv() != Err(mpsc::TryRecvError::Empty) {
                            return;
                        }
                        let samples = &normalized[chunk.range.start..chunk.range.end];
                        let needed = self.recording_cache.lock().ok().is_some_and(|cache| {
                            cache.as_ref().is_some_and(|(id, entries)| {
                                *id == session_id
                                    && entries.len() < 128
                                    && !entries.iter().any(|entry| {
                                        entry.range == chunk.range && entry.samples == samples
                                    })
                            })
                        });
                        if !needed {
                            continue;
                        }
                        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
                        let Ok(path) =
                            write_temp_wav_samples(samples, session_id, id, &self.config)
                        else {
                            return;
                        };
                        let result = self.transcribe_once(id, &path.to_string_lossy(), false);
                        let _ = fs::remove_file(path);
                        // Failure leaves the final, existing recognition path intact.
                        let Ok(result) = result else {
                            return;
                        };
                        if let Ok(mut cache) = self.recording_cache.lock() {
                            if let Some((active_id, entries)) = cache.as_mut() {
                                if *active_id == session_id {
                                    entries.push(CachedChunk {
                                        range: chunk.range,
                                        samples: samples.to_vec(),
                                        result,
                                    });
                                }
                            }
                        }
                    }
                }
            })
            .ok()?;
        Some(RecordingRecognition {
            stop,
            thread: Some(thread),
        })
    }

    fn prepare(&self) -> Result<Option<TranscriberPreparation>, String> {
        self.prepare_worker().map(Some)
    }

    fn transcribe(&self, request: &EngineRequest) -> Result<TranscriptionOutput, String> {
        self.transcribe_via_worker(request)
    }
}

impl Drop for LocalAsrWorkerTranscriber {
    fn drop(&mut self) {
        if let Ok(mut process_slot) = self.process.lock() {
            shutdown_worker(process_slot.take());
        }
    }
}

fn ensure_worker<'a>(
    config: &LocalAsrWorkerConfig,
    process_slot: &'a mut Option<WorkerProcess>,
) -> Result<&'a mut WorkerProcess, String> {
    if process_slot.is_none() {
        *process_slot = Some(spawn_worker(config)?);
    }

    process_slot
        .as_mut()
        .ok_or_else(|| "worker process unexpectedly missing".to_string())
}

fn shutdown_worker(worker: Option<WorkerProcess>) {
    if let Some(mut process) = worker {
        let _ = send_request(&mut process.stdin, &ShutdownRequest { cmd: "shutdown" });
        let _ = read_message(&process.receiver, Duration::from_secs(2));
        let _ = process.child.kill();
        let _ = process.child.wait();
    }
}

fn attempt_warmup(attempt: impl FnOnce() -> Result<(), String>) -> WarmupAttempt {
    let started_at = Instant::now();
    let succeeded = attempt().is_ok();
    WarmupAttempt {
        elapsed_ms: elapsed_u32(started_at),
        succeeded,
    }
}

fn warm_up_worker(
    process: &mut WorkerProcess,
    config: &LocalAsrWorkerConfig,
    request_id: u64,
) -> Result<(), String> {
    let samples = synthetic_warmup_samples();
    let wav_path = write_temp_wav_samples(&samples, 0, request_id, config)?;
    let wav_path_string = wav_path.to_string_lossy().to_string();
    let result = (|| {
        send_request(
            &mut process.stdin,
            &WorkerRequest {
                cmd: "transcribe",
                id: request_id,
                wav_path: &wav_path_string,
            },
        )?;
        let response = read_message(&process.receiver, config.request_timeout)?;
        if response.message_type != "transcript" {
            return Err(format!(
                "unexpected worker warm-up response type `{}`",
                response.message_type
            ));
        }
        if response.id != Some(request_id) {
            return Err("worker warm-up response id mismatch".to_string());
        }
        if !response.ok {
            return Err("worker warm-up inference failed".to_string());
        }
        Ok(())
    })();
    let _ = fs::remove_file(wav_path);
    result
}

fn synthetic_warmup_samples() -> Vec<f32> {
    vec![0.0_f32; samples_for_ms(WARMUP_AUDIO_MS)]
}

fn spawn_worker(config: &LocalAsrWorkerConfig) -> Result<WorkerProcess, String> {
    if !config.python_executable.exists() {
        return Err(format!(
            "python executable not found at {}",
            config.python_executable.display()
        ));
    }
    if !config.worker_script.exists() {
        return Err(format!(
            "worker script not found at {}",
            config.worker_script.display()
        ));
    }

    let mut worker_command = Command::new(&config.python_executable);
    worker_command
        .arg("-B")
        .arg(&config.worker_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_worker_command(&mut worker_command);

    let mut child = worker_command.spawn().map_err(|error| {
        format!(
            "failed to spawn local ASR worker at {} via {}: {}",
            config.worker_script.display(),
            config.python_executable.display(),
            error
        )
    })?;

    if let Some(stderr) = child.stderr.take() {
        spawn_stderr_pump(stderr);
    }

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "worker stdin was not available".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "worker stdout was not available".to_string())?;
    let receiver = spawn_stdout_pump(stdout);
    let ready = read_ready_or_stop(&mut child, &receiver, config)?;

    Ok(WorkerProcess {
        child,
        stdin,
        receiver,
        model_load_ms: round_metric_value(ready.model_load_ms),
    })
}

fn read_ready_or_stop(
    child: &mut Child,
    receiver: &Receiver<Result<WorkerMessage, String>>,
    config: &LocalAsrWorkerConfig,
) -> Result<WorkerMessage, String> {
    let result = read_message(receiver, config.startup_timeout).and_then(|ready| {
        if ready.message_type != "ready" {
            return Err(format!("worker sent unexpected first message type `{}`", ready.message_type));
        }
        if !ready.ok {
            return Err(ready.error.unwrap_or_else(|| format!(
                "worker failed to initialize; run {} manually for debugging", config.debug_script.display()
            )));
        }
        Ok(ready)
    });
    if result.is_err() {
        // Initialization has not installed this child in the shared process slot.
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

#[cfg(windows)]
fn configure_worker_command(command: &mut Command) {
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_worker_command(_command: &mut Command) {}

fn spawn_stdout_pump(
    stdout: impl Read + Send + 'static,
) -> Receiver<Result<WorkerMessage, String>> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let result = match line {
                Ok(message) => {
                    serde_json::from_str::<WorkerMessage>(message.trim()).map_err(|error| {
                        format!(
                            "failed to parse worker response `{}`: {}",
                            message.trim(),
                            error
                        )
                    })
                }
                Err(error) => Err(format!("failed to read worker response: {error}")),
            };

            if sender.send(result).is_err() {
                return;
            }
        }
    });

    receiver
}

fn spawn_stderr_pump(stderr: impl Read + Send + 'static) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(message) if !message.trim().is_empty() => {
                    eprintln!("[sensevoice-worker] {message}");
                }
                Ok(_) => {}
                Err(error) => {
                    eprintln!("[sensevoice-worker] stderr read error: {error}");
                    break;
                }
            }
        }
    });
}

fn send_request<T: Serialize>(stdin: &mut ChildStdin, message: &T) -> Result<(), String> {
    let encoded = serde_json::to_string(message)
        .map_err(|error| format!("failed to encode worker request: {error}"))?;
    stdin
        .write_all(encoded.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|error| format!("failed to write worker request: {error}"))
}

fn read_message(
    receiver: &Receiver<Result<WorkerMessage, String>>,
    timeout: Duration,
) -> Result<WorkerMessage, String> {
    match receiver.recv_timeout(timeout) {
        Ok(Ok(message)) => Ok(message),
        Ok(Err(error)) => Err(error),
        Err(RecvTimeoutError::Timeout) => Err(format!(
            "worker response timed out after {} ms",
            timeout.as_millis()
        )),
        Err(RecvTimeoutError::Disconnected) => Err("worker closed stdout unexpectedly".to_string()),
    }
}

fn parse_latency_metrics(timings: Option<HashMap<String, f64>>) -> Vec<AsrLatencyMetric> {
    let mut metrics = timings
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, value)| {
            round_metric_value(Some(value)).map(|rounded| AsrLatencyMetric {
                name,
                value_ms: rounded,
            })
        })
        .collect::<Vec<_>>();
    metrics.sort_by(|left, right| left.name.cmp(&right.name));
    metrics
}

fn round_metric_value(value: Option<f64>) -> Option<u32> {
    value.and_then(|raw| {
        if raw.is_finite() && raw >= 0.0 {
            Some(raw.round() as u32)
        } else {
            None
        }
    })
}

fn merge_chunk_transcripts(chunks: &[PlannedChunk], attempts: &[WorkerAttemptResult]) -> Result<String, String> {
    let mut parts = Vec::new();
    for (index, (chunk, attempt)) in chunks.iter().zip(attempts).enumerate() {
        let left_overlap = index.checked_sub(1).map_or(0, |i| chunks[i].range.end.saturating_sub(chunk.range.start));
        let right_overlap = chunks.get(index + 1).map_or(0, |next| chunk.range.end.saturating_sub(next.range.start));
        if (left_overlap == 0 && right_overlap == 0) || attempt.transcript.trim().is_empty() {
            parts.push(attempt.transcript.trim().to_string());
            continue;
        }
        // Assign each half of the overlap to one chunk by time, never by matching
        // text: actual repeated words at different times must remain in the output.
        if attempt.tokens.is_empty() || attempt.tokens.len() != attempt.timestamps.len()
            || attempt.timestamps.iter().any(|time| !time.is_finite() || *time < 0.0)
            || attempt.timestamps.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err("local ASR worker must provide valid token timestamps for overlapping chunks; update the bundled worker".into());
        }
        let start = left_overlap as f64 / 2.0 / TARGET_SAMPLE_RATE_HZ as f64;
        let end = (chunk.range.end - chunk.range.start) as f64 / TARGET_SAMPLE_RATE_HZ as f64
            - right_overlap as f64 / 2.0 / TARGET_SAMPLE_RATE_HZ as f64;
        let retained = attempt.tokens.iter().zip(&attempt.timestamps)
            .filter(|(_, time)| **time >= start && **time < end)
            .map(|(token, _)| token.as_str()).collect::<Vec<_>>();
        if retained.len() == attempt.tokens.len() {
            parts.push(attempt.transcript.trim().to_string());
        } else {
            parts.push(retained.concat().replace('▁', " ").trim().to_string());
        }
    }
    Ok(parts.into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" "))
}

fn plan_audio_chunks(samples: &[f32]) -> SegmentationPlan {
    let vad_threshold = adaptive_vad_threshold(samples);
    if samples.is_empty() {
        return SegmentationPlan {
            vad_enabled: false,
            vad_threshold,
            short_trim: ShortTrimDiagnostics::fallback(samples.len(), vad_threshold, "empty_audio"),
            chunks: Vec::new(),
            hard_cut_count: 0,
            dropped_region_count: 0,
            dropped_region_total_samples: 0,
        };
    }

    if samples.len() <= samples_for_ms(SEGMENTATION_THRESHOLD_MS) {
        let (range, short_trim) = plan_short_whole_clip_trim(samples, vad_threshold);
        return SegmentationPlan {
            vad_enabled: false,
            vad_threshold,
            short_trim,
            chunks: vec![PlannedChunk {
                range,
                source: ChunkSource::WholeClip,
            }],
            hard_cut_count: 0,
            dropped_region_count: 0,
            dropped_region_total_samples: 0,
        };
    }

    let protected_head_end = samples.len().min(samples_for_ms(PROTECTED_HEAD_MS));
    let mut chunks = Vec::new();
    let head_detection = raw_voiced_regions(samples, 0, protected_head_end, vad_threshold);

    if let Some(head) = protected_head_range(&head_detection.regions, protected_head_end) {
        chunks.push(PlannedChunk {
            range: head,
            source: ChunkSource::ProtectedHead,
        });
    }

    let tail_detection =
        raw_voiced_regions(samples, protected_head_end, samples.len(), vad_threshold);
    let tail_regions = voiced_regions(
        tail_detection.regions.clone(),
        protected_head_end,
        samples.len(),
    );
    let mut hard_cut_count = 0;
    for region in tail_regions {
        let (region_chunks, region_hard_cuts) = cap_region(samples, region);
        chunks.extend(region_chunks);
        hard_cut_count += region_hard_cuts;
    }

    SegmentationPlan {
        vad_enabled: true,
        vad_threshold,
        short_trim: ShortTrimDiagnostics::fallback(samples.len(), vad_threshold, "long_vad_path"),
        chunks,
        hard_cut_count,
        dropped_region_count: head_detection.dropped_region_count
            + tail_detection.dropped_region_count,
        dropped_region_total_samples: head_detection.dropped_region_total_samples
            + tail_detection.dropped_region_total_samples,
    }
}

fn prefetch_candidates<'a>(
    samples: &[f32],
    plan: &'a SegmentationPlan,
) -> impl Iterator<Item = &'a PlannedChunk> {
    let closed_tail = plan.chunks.last().is_some_and(|chunk| {
        chunk.source != ChunkSource::HardCut
            && samples.len().saturating_sub(chunk.range.end) >= samples_for_ms(MERGE_GAP_MS)
            // Use the lowest VAD threshold and reject even brief activity, rather
            // than treating quiet words or dropped blips as a confident pause.
            && samples[chunk.range.end..].chunks(samples_for_ms(VAD_FRAME_MS))
                .all(|frame| frame_rms(frame) < VAD_MIN_RMS)
    });
    let count = if plan.vad_enabled {
        plan.chunks.len().saturating_sub(usize::from(!closed_tail))
    } else {
        0
    };
    plan.chunks.iter().take(count)
}

fn plan_short_whole_clip_trim(
    samples: &[f32],
    threshold: f32,
) -> (AudioRange, ShortTrimDiagnostics) {
    let original_range = AudioRange {
        start: 0,
        end: samples.len(),
    };
    let detection = raw_voiced_regions(samples, 0, samples.len(), threshold);
    let Some(first) = detection.regions.first() else {
        return (
            original_range,
            ShortTrimDiagnostics::fallback(samples.len(), threshold, "no_confident_speech_region"),
        );
    };
    let last = detection
        .regions
        .last()
        .expect("first voiced region guarantees a last region");
    let voiced_samples = detection
        .regions
        .iter()
        .map(|region| region.end.saturating_sub(region.start))
        .sum::<usize>();
    if voiced_samples < samples_for_ms(SHORT_TRIM_MIN_SPEECH_MS) {
        return (
            original_range,
            ShortTrimDiagnostics::fallback(samples.len(), threshold, "detected_speech_too_short"),
        );
    }

    let range = AudioRange {
        start: first
            .start
            .saturating_sub(samples_for_ms(SHORT_TRIM_PRE_PADDING_MS)),
        end: last
            .end
            .saturating_add(samples_for_ms(SHORT_TRIM_POST_PADDING_MS))
            .min(samples.len()),
    };
    if range == original_range {
        return (
            original_range,
            ShortTrimDiagnostics::fallback(samples.len(), threshold, "no_trimmable_edge_silence"),
        );
    }

    (
        range,
        ShortTrimDiagnostics {
            enabled: true,
            input_samples: samples.len(),
            output_samples: range.end.saturating_sub(range.start),
            removed_leading_samples: range.start,
            removed_trailing_samples: samples.len().saturating_sub(range.end),
            threshold,
            fallback_reason: None,
        },
    )
}

fn protected_head_range(regions: &[AudioRange], head_end: usize) -> Option<AudioRange> {
    let first = regions.first()?;
    let last = regions.last()?;

    Some(AudioRange {
        start: first.start.saturating_sub(samples_for_ms(PRE_PADDING_MS)),
        end: last
            .end
            .saturating_add(samples_for_ms(POST_PADDING_MS))
            .min(head_end),
    })
}

fn voiced_regions(regions: Vec<AudioRange>, start: usize, end: usize) -> Vec<AudioRange> {
    let merged = merge_nearby_regions(regions, samples_for_ms(MERGE_GAP_MS));
    let padded = merged
        .into_iter()
        .map(|region| AudioRange {
            start: region
                .start
                .saturating_sub(samples_for_ms(PRE_PADDING_MS))
                .max(start),
            end: region
                .end
                .saturating_add(samples_for_ms(POST_PADDING_MS))
                .min(end),
        })
        .collect::<Vec<_>>();

    merge_nearby_regions(padded, 0)
}

fn raw_voiced_regions(
    samples: &[f32],
    start: usize,
    end: usize,
    vad_threshold: f32,
) -> VoicedRegionDetection {
    if start >= end || start >= samples.len() {
        return VoicedRegionDetection::default();
    }

    let frame_samples = samples_for_ms(VAD_FRAME_MS);
    let min_voiced_samples = samples_for_ms(MIN_VOICED_RUN_MS);
    let bounded_end = end.min(samples.len());
    let mut detection = VoicedRegionDetection::default();
    let mut voiced_start = None;
    let mut cursor = start;

    while cursor < bounded_end {
        let frame_end = cursor.saturating_add(frame_samples).min(bounded_end);
        let voiced = frame_rms(&samples[cursor..frame_end]) >= vad_threshold;

        match (voiced_start, voiced) {
            (None, true) => voiced_start = Some(cursor),
            (Some(region_start), false) => {
                record_voiced_region(
                    &mut detection,
                    AudioRange {
                        start: region_start,
                        end: cursor,
                    },
                    min_voiced_samples,
                );
                voiced_start = None;
            }
            _ => {}
        }

        cursor = frame_end;
    }

    if let Some(region_start) = voiced_start {
        record_voiced_region(
            &mut detection,
            AudioRange {
                start: region_start,
                end: bounded_end,
            },
            min_voiced_samples,
        );
    }

    detection
}

fn record_voiced_region(
    detection: &mut VoicedRegionDetection,
    region: AudioRange,
    min_voiced_samples: usize,
) {
    let duration_samples = region.end.saturating_sub(region.start);
    if duration_samples >= min_voiced_samples {
        detection.regions.push(region);
    } else {
        detection.dropped_region_count += 1;
        detection.dropped_region_total_samples = detection
            .dropped_region_total_samples
            .saturating_add(duration_samples);
    }
}

fn merge_nearby_regions(regions: Vec<AudioRange>, max_gap_samples: usize) -> Vec<AudioRange> {
    let mut merged: Vec<AudioRange> = Vec::new();

    for region in regions {
        if let Some(previous) = merged.last_mut()
            && region.start.saturating_sub(previous.end) <= max_gap_samples
        {
            previous.end = previous.end.max(region.end);
            continue;
        }
        merged.push(region);
    }

    merged
}

fn cap_region(samples: &[f32], region: AudioRange) -> (Vec<PlannedChunk>, usize) {
    let max_samples = samples_for_ms(MAX_CHUNK_MS);
    if region.end.saturating_sub(region.start) <= max_samples {
        return (
            vec![PlannedChunk {
                range: region,
                source: ChunkSource::VadRegion,
            }],
            0,
        );
    }

    let overlap_samples = samples_for_ms(HARD_CUT_OVERLAP_MS);
    let mut chunks = Vec::new();
    let mut chunk_start = region.start;
    let mut hard_cut_count = 0;

    while region.end.saturating_sub(chunk_start) > max_samples {
        let cut = choose_low_energy_cut(samples, chunk_start, region.end);
        chunks.push(PlannedChunk {
            range: AudioRange {
                start: chunk_start,
                end: cut.saturating_add(overlap_samples).min(region.end),
            },
            source: ChunkSource::HardCut,
        });
        chunk_start = cut;
        hard_cut_count += 1;
    }

    if chunk_start < region.end {
        chunks.push(PlannedChunk {
            range: AudioRange {
                start: chunk_start,
                end: region.end,
            },
            source: ChunkSource::HardCut,
        });
    }

    (chunks, hard_cut_count)
}

fn choose_low_energy_cut(samples: &[f32], chunk_start: usize, region_end: usize) -> usize {
    let preferred = chunk_start.saturating_add(samples_for_ms(PREFERRED_CHUNK_MS));
    let latest = chunk_start
        .saturating_add(samples_for_ms(MAX_CHUNK_MS - HARD_CUT_OVERLAP_MS))
        .min(region_end);
    let frame_samples = samples_for_ms(VAD_FRAME_MS);
    let search_start = preferred.min(latest);
    let mut best_cut = search_start;
    let mut best_rms = f32::INFINITY;
    let mut cursor = search_start;

    while cursor < latest {
        let frame_end = cursor.saturating_add(frame_samples).min(latest);
        let rms = frame_rms(&samples[cursor..frame_end]);
        if rms < best_rms {
            best_rms = rms;
            best_cut = cursor;
        }
        cursor = frame_end;
    }

    best_cut.max(chunk_start.saturating_add(1)).min(latest)
}

fn transcribe_chunks<T, F>(
    samples: &[f32],
    chunks: &[PlannedChunk],
    mut transcribe_chunk: F,
) -> Result<Vec<T>, String>
where
    F: FnMut(usize, &PlannedChunk, &[f32]) -> Result<T, String>,
{
    let mut outputs = Vec::with_capacity(chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        let output = transcribe_chunk(index, chunk, &samples[chunk.range.start..chunk.range.end])
            .map_err(|error| {
            format!(
                "local ASR chunk {}/{} failed: {}",
                index + 1,
                chunks.len(),
                error
            )
        })?;
        outputs.push(output);
    }
    Ok(outputs)
}

fn adaptive_vad_threshold(samples: &[f32]) -> f32 {
    (frame_rms(samples) * VAD_AUDIO_RMS_MULTIPLIER).clamp(VAD_MIN_RMS, VAD_MAX_RMS)
}

fn format_short_trim_diagnostic(short_trim: &ShortTrimDiagnostics) -> String {
    format!(
        "short_trim_enabled={} short_trim_input_ms={} short_trim_output_ms={} short_trim_removed_leading_ms={} short_trim_removed_trailing_ms={} short_trim_threshold={:.6} short_trim_fallback_reason={}",
        short_trim.enabled,
        samples_to_ms(short_trim.input_samples),
        samples_to_ms(short_trim.output_samples),
        samples_to_ms(short_trim.removed_leading_samples),
        samples_to_ms(short_trim.removed_trailing_samples),
        short_trim.threshold,
        short_trim.fallback_reason.unwrap_or("none"),
    )
}

fn log_segmentation_plan(samples: &[f32], plan: &SegmentationPlan) {
    eprintln!(
        "[voiceflow-asr] {}",
        format_short_trim_diagnostic(&plan.short_trim)
    );
    eprintln!(
        "[voiceflow-asr] vad_enabled={} vad_threshold={:.6} chunk_count={} hard_cut_count={} dropped_region_count={} dropped_region_total_ms={}",
        plan.vad_enabled,
        plan.vad_threshold,
        plan.chunks.len(),
        plan.hard_cut_count,
        plan.dropped_region_count,
        samples_to_ms(plan.dropped_region_total_samples),
    );

    for (index, chunk) in plan.chunks.iter().enumerate() {
        let chunk_samples = &samples[chunk.range.start..chunk.range.end];
        let (peak, rms) = peak_and_rms(chunk_samples);
        eprintln!(
            "[voiceflow-asr] chunk={}/{} source={} start_ms={} end_ms={} duration_ms={} peak={:.6} rms={:.6}",
            index + 1,
            plan.chunks.len(),
            chunk.source.label(),
            samples_to_ms(chunk.range.start),
            samples_to_ms(chunk.range.end),
            samples_to_ms(chunk.range.end.saturating_sub(chunk.range.start)),
            peak,
            rms,
        );
    }
}

fn log_chunk_result(
    index: usize,
    chunk_count: usize,
    chunk: &PlannedChunk,
    asr_time_ms: u128,
    transcript: Option<&str>,
    debug_transcripts: bool,
) {
    let transcript_char_count = transcript.map(|text| text.chars().count()).unwrap_or(0);
    eprintln!(
        "[voiceflow-asr] chunk={}/{} source={} start_ms={} end_ms={} duration_ms={} asr_time_ms={} transcript_char_count={} success={}",
        index + 1,
        chunk_count,
        chunk.source.label(),
        samples_to_ms(chunk.range.start),
        samples_to_ms(chunk.range.end),
        samples_to_ms(chunk.range.end.saturating_sub(chunk.range.start)),
        asr_time_ms,
        transcript_char_count,
        transcript.is_some(),
    );

    if debug_transcripts {
        if let Some(transcript) = transcript {
            eprintln!(
                "[voiceflow-asr] chunk={}/{} transcript={:?}",
                index + 1,
                chunk_count,
                transcript
            );
        }
    }
}

fn debug_transcript_logging_enabled() -> bool {
    env::var(ASR_DEBUG_TRANSCRIPTS_ENV)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn peak_and_rms(samples: &[f32]) -> (f32, f32) {
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    (peak, frame_rms(samples))
}

fn samples_to_ms(samples: usize) -> usize {
    samples.saturating_mul(1000) / TARGET_SAMPLE_RATE_HZ
}

fn merge_chunk_diagnostics(attempts: &[WorkerAttemptResult]) -> Option<AsrDiagnostics> {
    let first = attempts.first()?;
    let mut metrics = BTreeMap::<String, u32>::new();
    for attempt in attempts {
        for metric in &attempt.diagnostics.metrics {
            let total = metrics.entry(metric.name.clone()).or_default();
            *total = total.saturating_add(metric.value_ms);
        }
    }

    Some(AsrDiagnostics {
        backend: first.diagnostics.backend.clone(),
        worker_request_id: attempts
            .last()
            .and_then(|attempt| attempt.diagnostics.worker_request_id),
        worker_model_load_ms: attempts
            .iter()
            .find_map(|attempt| attempt.diagnostics.worker_model_load_ms),
        worker_restarted: attempts
            .iter()
            .any(|attempt| attempt.diagnostics.worker_restarted),
        metrics: metrics
            .into_iter()
            .map(|(name, value_ms)| AsrLatencyMetric { name, value_ms })
            .collect(),
    })
}

fn append_latency_metric(diagnostics: &mut AsrDiagnostics, name: &str, value_ms: u32) {
    diagnostics.metrics.push(AsrLatencyMetric {
        name: name.to_string(),
        value_ms,
    });
}

fn elapsed_u32(started_at: Instant) -> u32 {
    started_at.elapsed().as_millis().min(u32::MAX as u128) as u32
}

fn samples_for_ms(milliseconds: usize) -> usize {
    TARGET_SAMPLE_RATE_HZ.saturating_mul(milliseconds) / 1000
}

fn frame_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let mean_square = samples
        .iter()
        .map(|sample| {
            let sample = f64::from(*sample);
            sample * sample
        })
        .sum::<f64>()
        / samples.len() as f64;
    mean_square.sqrt() as f32
}

fn write_temp_wav_samples(
    samples: &[f32],
    session_id: u64,
    request_id: u64,
    config: &LocalAsrWorkerConfig,
) -> Result<PathBuf, String> {
    if samples.is_empty() {
        return Err("ASR chunk contained no samples".to_string());
    }

    let temp_dir = env::temp_dir().join("voiceflow-speech-input");
    fs::create_dir_all(&temp_dir).map_err(|error| {
        format!(
            "failed to create temp audio directory {}: {}",
            temp_dir.display(),
            error
        )
    })?;

    let wav_path = temp_dir.join(format!(
        "session-{session_id}-request-{request_id}-sensevoice.wav"
    ));

    write_pcm16_wav_file(&wav_path, samples).map_err(|error| {
        format!(
            "failed to write worker WAV {} for {}: {}",
            wav_path.display(),
            config.worker_script.display(),
            error
        )
    })?;

    Ok(wav_path)
}

fn normalize_to_mono_16khz(captured_audio: &CapturedAudio) -> Vec<f32> {
    normalize_audio_suffix(captured_audio, 0)
}

fn normalize_with_prefix(
    audio: &CapturedAudio,
    prefix: Option<NormalizedPrefix>,
) -> (Vec<f32>, usize) {
    let Some(mut prefix) = prefix.filter(|prefix| {
        prefix.rate == audio.sample_rate_hz
            && prefix.channels == audio.channels
            && audio.sample_rate_hz != 0
            && audio.samples.starts_with(&prefix.source)
    }) else {
        return (normalize_to_mono_16khz(audio), 0);
    };
    // The previous last mono frame may be partial, and its interpolation neighbour
    // may not have arrived yet. Recompute from before that boundary, at global
    // output indices, so chunked processing is byte-identical to whole-clip input.
    let stable_frames = (prefix.source.len() / audio.channels.max(1) as usize).saturating_sub(1);
    let reused = ((stable_frames as f64 * TARGET_SAMPLE_RATE_HZ as f64
        / audio.sample_rate_hz as f64)
        .floor() as usize)
        .min(prefix.normalized.len());
    prefix.normalized.truncate(reused);
    prefix
        .normalized
        .extend(normalize_audio_suffix(audio, reused));
    (prefix.normalized, reused)
}

fn normalize_audio_suffix(captured_audio: &CapturedAudio, output_start: usize) -> Vec<f32> {
    let channels = captured_audio.channels.max(1) as usize;
    if captured_audio.sample_rate_hz == 0 {
        return Vec::new();
    }
    let source_start = (output_start as f64 * captured_audio.sample_rate_hz as f64
        / TARGET_SAMPLE_RATE_HZ as f64)
        .floor() as usize;
    let source =
        &captured_audio.samples[(source_start * channels).min(captured_audio.samples.len())..];
    let mono: Vec<f32> = if channels == 1 {
        source.to_vec()
    } else {
        source
            .chunks(channels)
            .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
            .collect()
    };

    if mono.is_empty() {
        return Vec::new();
    }

    if captured_audio.sample_rate_hz == 16_000 {
        return mono;
    }

    if captured_audio.sample_rate_hz == 0 {
        return Vec::new();
    }

    let source_rate = captured_audio.sample_rate_hz as f64;
    let target_rate = 16_000_f64;
    let output_len =
        (((source_start + mono.len()) as f64) * target_rate / source_rate).round() as usize;

    (output_start..output_len)
        .map(|index| {
            let position = index as f64 * source_rate / target_rate;
            let lower = position.floor() as usize - source_start;
            let upper = (lower + 1).min(mono.len().saturating_sub(1));
            let fraction = (position - position.floor()) as f32;
            let lower_sample = mono[lower];
            let upper_sample = mono[upper];
            lower_sample + (upper_sample - lower_sample) * fraction
        })
        .collect()
}

fn write_pcm16_wav_file(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    write_pcm16_wav(File::create(path)?, samples)
}

fn write_pcm16_wav(writer: impl Write, samples: &[f32]) -> std::io::Result<()> {
    let mut file = BufWriter::with_capacity(64 * 1024, writer);
    let sample_rate = 16_000_u32;
    let channels = 1_u16;
    let bits_per_sample = 16_u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_chunk_size = (samples.len() * 2) as u32;
    let riff_chunk_size = 36 + data_chunk_size;

    file.write_all(b"RIFF")?;
    file.write_all(&riff_chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_chunk_size.to_le_bytes())?;

    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32).round() as i16;
        file.write_all(&pcm.to_le_bytes())?;
    }

    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_uses_timestamps_and_preserves_real_repeated_words() {
        let chunks = vec![
            planned_chunk(AudioRange { start: 0, end: samples_for_ms(20_400) }),
            planned_chunk(AudioRange { start: samples_for_ms(20_000), end: samples_for_ms(25_000) }),
        ];
        let result = |text: &str, tokens: &[&str], timestamps: Vec<f64>| WorkerAttemptResult {
            transcript: text.into(), tokens: tokens.iter().map(|s| s.to_string()).collect(), timestamps,
            diagnostics: AsrDiagnostics { backend: "test".into(), worker_request_id: None,
                worker_model_load_ms: None, worker_restarted: false, metrics: vec![] },
        };
        let attempts = vec![
            result("go go", &["▁go", "▁go"], vec![19.8, 20.3]),
            result("go now", &["▁go", "▁now"], vec![0.3, 1.0]),
        ];
        assert_eq!(merge_chunk_transcripts(&chunks, &attempts).unwrap(), "go go now");
        let attempts = vec![
            result("明天发布", &["明", "天", "发", "布"], vec![18.0, 18.5, 20.25, 20.35]),
            result("发布新版", &["发", "布", "新", "版"], vec![0.25, 0.35, 1.0, 1.5]),
        ];
        assert_eq!(merge_chunk_transcripts(&chunks, &attempts).unwrap(), "明天 发布新版");
        let attempts = vec![result("old worker", &[], vec![]), result("text", &[], vec![])];
        assert!(merge_chunk_transcripts(&chunks, &attempts).is_err());
    }

    #[test]
    #[ignore = "requires VOICEFLOW_TEST_PYTHON; starts only a local fake worker"]
    fn failed_ready_reaps_the_child() {
        let python = env::var_os("VOICEFLOW_TEST_PYTHON").expect("set VOICEFLOW_TEST_PYTHON");
        for first_message in ["", "not-json", r#"{"type":"other","ok":true}"#, r#"{"type":"ready","ok":false}"#] {
            let mut command = Command::new(&python);
            command.arg("-c").arg("import sys,time; print(sys.argv[1], flush=True) if sys.argv[1] else None; time.sleep(60)")
                .arg(first_message).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
            configure_worker_command(&mut command);
            let mut child = command.spawn().unwrap();
            let receiver = spawn_stdout_pump(child.stdout.take().unwrap());
            let config = LocalAsrWorkerConfig { startup_timeout: Duration::from_millis(500), ..Default::default() };
            assert!(read_ready_or_stop(&mut child, &receiver, &config).is_err());
            assert!(child.try_wait().unwrap().is_some(), "failed startup must reap its child");
        }
    }

    #[test]
    fn only_naturally_closed_tail_is_prefetched_without_changing_plan() {
        let mut audio = vec![0.1; samples_for_ms(30_000)];
        let growing = plan_audio_chunks(&audio);
        assert_eq!(
            prefetch_candidates(&audio, &growing).count(),
            growing.chunks.len() - 1
        );
        audio[samples_for_ms(28_000)..].fill(0.0);
        let ended = plan_audio_chunks(&audio);
        assert_eq!(
            prefetch_candidates(&audio, &ended).count(),
            ended.chunks.len()
        );
        assert_eq!(ended.hard_cut_count, 0);
        // No splitting of the existing tail, only early computation of the same range.
        assert_eq!(ended.chunks.len(), growing.chunks.len());
        let mut brief_pause = vec![0.1; samples_for_ms(30_000)];
        brief_pause[samples_for_ms(29_100)..].fill(0.0);
        let brief = plan_audio_chunks(&brief_pause);
        assert_eq!(
            prefetch_candidates(&brief_pause, &brief).count(),
            brief.chunks.len() - 1
        );
        // A small voiced blip is insufficient for VAD, but must still block speculation.
        audio[samples_for_ms(29_600)..samples_for_ms(29_620)].fill(0.1);
        let blip = plan_audio_chunks(&audio);
        assert_eq!(
            prefetch_candidates(&audio, &blip).count(),
            blip.chunks.len() - 1
        );
    }

    #[test]
    fn short_silence_and_hard_cut_tails_are_not_new_prefetch_candidates() {
        let short = vec![0.0; samples_for_ms(20_000)];
        assert_eq!(
            prefetch_candidates(&short, &plan_audio_chunks(&short)).count(),
            0
        );
        let silence = vec![0.0; samples_for_ms(30_000)];
        assert_eq!(
            prefetch_candidates(&silence, &plan_audio_chunks(&silence)).count(),
            0
        );
        let mut long = vec![0.1; samples_for_ms(70_000)];
        long[samples_for_ms(68_000)..].fill(0.0);
        let plan = plan_audio_chunks(&long);
        assert_eq!(plan.chunks.last().unwrap().source, ChunkSource::HardCut);
        assert_eq!(
            prefetch_candidates(&long, &plan).count(),
            plan.chunks.len() - 1
        );
    }

    fn reference_normalize(audio: &CapturedAudio) -> Vec<f32> {
        let mono: Vec<f32> = audio
            .samples
            .chunks(audio.channels.max(1) as usize)
            .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
            .collect();
        if mono.is_empty() || audio.sample_rate_hz == 0 {
            return Vec::new();
        }
        if audio.sample_rate_hz == 16_000 {
            return mono;
        }
        let len = (mono.len() as f64 * 16_000.0 / audio.sample_rate_hz as f64).round() as usize;
        (0..len)
            .map(|index| {
                let position = index as f64 * audio.sample_rate_hz as f64 / 16_000.0;
                let lower = position.floor() as usize;
                let upper = (lower + 1).min(mono.len() - 1);
                mono[lower] + (mono[upper] - mono[lower]) * (position - lower as f64) as f32
            })
            .collect()
    }

    #[test]
    fn incremental_normalization_matches_original_at_partial_frame_boundaries() {
        let source: Vec<f32> = (0..5001)
            .map(|i| ((i * 73 % 997) as f32 - 498.0) / 997.0)
            .collect();
        for rate in [8000, 16000, 22050, 44100, 48000] {
            for channels in [1, 2, 3] {
                let mut prefix = None;
                for len in [0, 1, 2, 3, 7, 113, 1000, 2001, 4000, 5001] {
                    let audio = CapturedAudio::from_samples(rate, channels, source[..len].to_vec());
                    let (normalized, reused) = normalize_with_prefix(&audio, prefix);
                    assert_eq!(
                        normalized,
                        reference_normalize(&audio),
                        "rate={rate} channels={channels} len={len}"
                    );
                    if len >= 1000 {
                        assert!(reused > 0);
                    }
                    prefix = Some(NormalizedPrefix {
                        rate,
                        channels,
                        source: audio.samples,
                        normalized,
                    });
                }
            }
        }
    }

    #[test]
    fn changed_or_truncated_audio_resets_normalization_reuse() {
        for change in 0..4 {
            let original = CapturedAudio::from_samples(48000, 2, vec![0.1; 1000]);
            let prefix = NormalizedPrefix {
                rate: 48000,
                channels: 2,
                source: original.samples.clone(),
                normalized: reference_normalize(&original),
            };
            let mut audio = original;
            match change {
                0 => audio.samples[0] = 0.2,
                1 => audio.samples.truncate(500),
                2 => audio.sample_rate_hz = 44100,
                _ => audio.channels = 1,
            }
            let (normalized, reused) = normalize_with_prefix(&audio, Some(prefix));
            assert_eq!(reused, 0);
            assert_eq!(normalized, reference_normalize(&audio));
        }
    }

    #[test]
    fn wav_buffer_preserves_header_samples_and_flushes() {
        let mut bytes = Vec::new();
        write_pcm16_wav(&mut bytes, &[-2.0, -1.0, 0.0, 1.0, 2.0]).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 46);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 10);
        let pcm: Vec<_> = bytes[44..]
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert_eq!(pcm, [-32767, -32767, 0, 32767, 32767]);
    }

    #[test]
    #[ignore = "local filesystem benchmark; no microphone or network"]
    fn benchmark_85_second_wav_write() {
        let samples = vec![0.1_f32; 85 * 16_000];
        let path = env::temp_dir().join(format!("voiceflow-wav-bench-{}.wav", std::process::id()));
        let started = Instant::now();
        write_pcm16_wav_file(&path, &samples).unwrap();
        let buffered = started.elapsed();
        let expected = fs::read(&path).unwrap();
        let started = Instant::now();
        {
            let mut file = File::create(&path).unwrap();
            file.write_all(&expected[..44]).unwrap();
            for sample in &samples {
                let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                file.write_all(&pcm.to_le_bytes()).unwrap();
            }
        }
        let unbuffered = started.elapsed();
        assert_eq!(fs::read(&path).unwrap(), expected);
        fs::remove_file(path).unwrap();
        eprintln!("85s WAV: buffered={buffered:?} per_sample_file_write={unbuffered:?}");
    }

    #[test]
    fn long_wav_uses_batched_writes_and_propagates_flush_failure() {
        #[derive(Default)]
        struct Counter {
            writes: usize,
            bytes: usize,
        }
        impl Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.writes += 1;
                self.bytes += bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut counter = Counter::default();
        write_pcm16_wav(&mut counter, &vec![0.1; 85 * 16_000]).unwrap();
        assert_eq!(counter.bytes, 44 + 85 * 16_000 * 2);
        assert!(counter.writes < 50, "{} writes", counter.writes);
        struct FailedWriter;
        impl Write for FailedWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk write failed"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(write_pcm16_wav(FailedWriter, &[0.1]).is_err());
    }

    fn cached_request() -> EngineRequest {
        EngineRequest {
            session_id: 123,
            requested_kind: shared_protocol::SessionKind::Dictation,
            captured_audio: CapturedAudio::from_samples(16_000, 1, vec![0.1; 30 * 16_000]),
            transcript_hint: None,
            refinement_profile: shared_protocol::builtin_refinement_model_profile_for_quality(
                &shared_protocol::RefinementQuality::BestQuality,
            ),
            provider_settings: Default::default(),
            provider_runtime_config: None,
            dictation_routing: None,
        }
    }

    fn seeded_transcriber(request: &EngineRequest) -> LocalAsrWorkerTranscriber {
        let worker = LocalAsrWorkerTranscriber::new(LocalAsrWorkerConfig {
            python_executable: PathBuf::from("nonexistent-prefetch-test-python"),
            max_restart_attempts: 0,
            ..Default::default()
        });
        let samples = normalize_to_mono_16khz(&request.captured_audio);
        let entries = plan_audio_chunks(&samples)
            .chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| CachedChunk {
                range: chunk.range,
                samples: samples[chunk.range.start..chunk.range.end].to_vec(),
                result: WorkerAttemptResult {
                    transcript: format!("part{i}"),
                    tokens: vec![],
                    timestamps: vec![],
                    diagnostics: AsrDiagnostics {
                        backend: "test".into(),
                        worker_request_id: Some(i as u64),
                        worker_model_load_ms: None,
                        worker_restarted: false,
                        metrics: vec![AsrLatencyMetric {
                            name: "total_ms".into(),
                            value_ms: 100,
                        }],
                    },
                },
            })
            .collect();
        *worker.recording_cache.lock().unwrap() = Some((request.session_id, entries));
        worker
    }

    #[test]
    fn exact_prefetch_reuses_in_final_order_without_worker_or_double_counting() {
        let request = cached_request();
        let worker = seeded_transcriber(&request);
        let output = worker.transcribe(&request).unwrap();
        assert_eq!(output.transcript, "part0 part1");
        let metrics = output.diagnostics.unwrap().metrics;
        assert_eq!(
            metrics
                .iter()
                .find(|m| m.name == "total_ms")
                .unwrap()
                .value_ms,
            0
        );
        assert_eq!(
            metrics
                .iter()
                .find(|m| m.name == "prefetched_worker_ms")
                .unwrap()
                .value_ms,
            200
        );
        assert!(worker.recording_cache.lock().unwrap().is_none());
    }

    #[test]
    fn changed_samples_ranges_and_sessions_never_reuse_stale_prefetch() {
        for change in 0..3 {
            let mut request = cached_request();
            let worker = seeded_transcriber(&request);
            match change {
                0 => request.session_id += 1,
                1 => request.captured_audio.samples[100] = 0.2,
                _ => {
                    worker.recording_cache.lock().unwrap().as_mut().unwrap().1[0]
                        .range
                        .end -= 1
                }
            }
            // A cache miss reaches the normal worker (deliberately absent here),
            // rather than returning a successful but incorrect cached transcript.
            assert!(worker.transcribe(&request).is_err());
        }
    }

    #[test]
    fn recording_guard_cancels_and_joins_without_waiting_for_poll_interval() {
        let worker = Arc::new(LocalAsrWorkerTranscriber::default());
        let guard = Arc::clone(&worker)
            .start_recording(1, Box::new(|| None))
            .unwrap();
        drop(guard);
        assert_eq!(Arc::strong_count(&worker), 1);
    }

    #[test]
    #[ignore = "requires VOICEFLOW_TEST_PYTHON; uses a local fake worker, no model or network"]
    fn background_prefetch_then_final_tail_uses_same_worker() {
        let python = env::var_os("VOICEFLOW_TEST_PYTHON").expect("set VOICEFLOW_TEST_PYTHON");
        let script = env::temp_dir().join(format!(
            "voiceflow-prefetch-worker-{}.py",
            std::process::id()
        ));
        fs::write(&script, r#"import json, sys, wave
print(json.dumps({'type':'ready','ok':True}), flush=True)
for line in sys.stdin:
    req = json.loads(line)
    if req['cmd'] == 'shutdown': break
    with wave.open(req['wav_path'], 'rb') as audio:
        assert audio.getframerate() == 16000
        assert audio.getnframes() > 0
    print(json.dumps({'type':'transcript','ok':True,'id':req['id'], 'transcript':'part'+str(req['id']), 'timings':{'total_ms':100}}), flush=True)
"#).unwrap();
        let worker = Arc::new(LocalAsrWorkerTranscriber::new(LocalAsrWorkerConfig {
            python_executable: python.into(),
            worker_script: script.clone(),
            startup_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(5),
            max_restart_attempts: 0,
            ..Default::default()
        }));
        let request = cached_request();
        let audio = request.captured_audio.clone();
        let guard = Arc::clone(&worker)
            .start_recording(request.session_id, Box::new(move || Some(audio.clone())))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while worker
            .recording_cache
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .1
            .is_empty()
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(25));
        }
        drop(guard);
        assert_eq!(
            worker
                .recording_cache
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .1
                .len(),
            1
        );
        let output = worker.transcribe(&request).unwrap();
        assert_eq!(output.transcript, "part1 part2");
        assert_eq!(worker.next_request_id.load(Ordering::Relaxed), 3);
        let metrics = output.diagnostics.unwrap().metrics;
        assert_eq!(
            metrics
                .iter()
                .find(|m| m.name == "prefetched_worker_ms")
                .unwrap()
                .value_ms,
            100
        );
        // A natural trailing pause permits prefetching the final chunk too.
        let mut request = request;
        request.session_id += 1;
        request.captured_audio.samples[samples_for_ms(28_000)..].fill(0.0);
        let audio = request.captured_audio.clone();
        let guard = Arc::clone(&worker)
            .start_recording(request.session_id, Box::new(move || Some(audio.clone())))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while worker
            .recording_cache
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .1
            .len()
            < 2
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(25));
        }
        drop(guard);
        assert_eq!(
            worker
                .recording_cache
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .1
                .len(),
            2
        );
        let output = worker.transcribe(&request).unwrap();
        assert_eq!(output.transcript, "part3 part4");
        assert_eq!(worker.next_request_id.load(Ordering::Relaxed), 5);
        assert!(worker.normalization_cache.lock().unwrap().is_none());
        drop(worker);
        fs::remove_file(script).unwrap();
    }

    #[test]
    fn warmup_audio_is_short_and_silent() {
        let samples = synthetic_warmup_samples();

        assert_eq!(samples.len(), samples_for_ms(WARMUP_AUDIO_MS));
        assert!(samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn failed_warmup_is_reported_without_retaining_error_details() {
        let outcome = attempt_warmup(|| Err("sensitive worker failure detail".to_string()));

        assert!(!outcome.succeeded);
        assert!(outcome.elapsed_ms < u32::MAX);
    }

    fn silent_audio(duration_ms: usize) -> Vec<f32> {
        vec![0.0; samples_for_ms(duration_ms)]
    }

    fn add_tone(samples: &mut [f32], start_ms: usize, end_ms: usize) {
        let start = samples_for_ms(start_ms);
        let end = samples_for_ms(end_ms).min(samples.len());
        samples[start..end].fill(0.1);
    }

    fn duration_ms(range: AudioRange) -> usize {
        range.end.saturating_sub(range.start) * 1000 / TARGET_SAMPLE_RATE_HZ
    }

    fn planned_chunk(range: AudioRange) -> PlannedChunk {
        PlannedChunk {
            range,
            source: ChunkSource::VadRegion,
        }
    }

    #[test]
    fn audio_at_or_below_25_seconds_is_not_segmented() {
        let samples = silent_audio(25_000);
        let plan = plan_audio_chunks(&samples);

        assert!(!plan.vad_enabled);
        assert!(!plan.short_trim.enabled);
        assert_eq!(
            plan.short_trim.fallback_reason,
            Some("no_confident_speech_region")
        );
        assert_eq!(
            plan.chunks,
            vec![PlannedChunk {
                range: AudioRange {
                    start: 0,
                    end: samples.len()
                },
                source: ChunkSource::WholeClip,
            }]
        );
    }

    #[test]
    fn adaptive_threshold_is_half_audio_rms_with_safe_clamps() {
        let observed_quiet_audio = vec![0.006; samples_for_ms(30_000)];
        let silent = silent_audio(30_000);
        let loud = vec![0.1; samples_for_ms(30_000)];

        assert!((adaptive_vad_threshold(&observed_quiet_audio) - 0.003).abs() < 0.000001);
        assert_eq!(adaptive_vad_threshold(&silent), VAD_MIN_RMS);
        assert_eq!(adaptive_vad_threshold(&loud), VAD_MAX_RMS);
    }

    #[test]
    fn audio_over_25_seconds_uses_segmentation() {
        let mut samples = silent_audio(30_000);
        add_tone(&mut samples, 2_000, 3_000);
        add_tone(&mut samples, 20_000, 21_000);

        let plan = plan_audio_chunks(&samples);

        assert!(plan.vad_enabled);
        assert_eq!(plan.chunks.len(), 2);
        assert!(
            plan.chunks
                .iter()
                .all(|chunk| chunk.range.end <= samples.len())
        );
        assert_eq!(plan.chunks[0].source, ChunkSource::ProtectedHead);
        assert_eq!(plan.chunks[1].source, ChunkSource::VadRegion);
        assert!(!plan.short_trim.enabled);
        assert_eq!(plan.short_trim.fallback_reason, Some("long_vad_path"));
    }

    #[test]
    fn short_whole_clip_trims_leading_and_trailing_silence_with_padding() {
        let mut samples = silent_audio(2_000);
        add_tone(&mut samples, 600, 1_200);

        let plan = plan_audio_chunks(&samples);
        let chunk = plan.chunks[0];

        assert!(!plan.vad_enabled);
        assert_eq!(chunk.source, ChunkSource::WholeClip);
        assert!(plan.short_trim.enabled);
        assert_eq!(samples_to_ms(chunk.range.start), 300);
        assert_eq!(samples_to_ms(chunk.range.end), 1_700);
        assert_eq!(samples_to_ms(plan.short_trim.removed_leading_samples), 300);
        assert_eq!(samples_to_ms(plan.short_trim.removed_trailing_samples), 300);
        assert_eq!(samples_to_ms(plan.short_trim.input_samples), 2_000);
        assert_eq!(samples_to_ms(plan.short_trim.output_samples), 1_400);
        assert_eq!(plan.short_trim.fallback_reason, None);

        let diagnostic = format_short_trim_diagnostic(&plan.short_trim);
        for field in [
            "short_trim_enabled=true",
            "short_trim_input_ms=2000",
            "short_trim_output_ms=1400",
            "short_trim_removed_leading_ms=300",
            "short_trim_removed_trailing_ms=300",
            "short_trim_threshold=",
            "short_trim_fallback_reason=none",
        ] {
            assert!(
                diagnostic.contains(field),
                "missing diagnostic field {field}"
            );
        }
    }

    #[test]
    fn short_whole_clip_preserves_internal_pauses() {
        let mut samples = silent_audio(3_000);
        add_tone(&mut samples, 600, 900);
        add_tone(&mut samples, 2_000, 2_300);

        let plan = plan_audio_chunks(&samples);
        let chunk = plan.chunks[0];

        assert!(plan.short_trim.enabled);
        assert!(chunk.range.start <= samples_for_ms(600));
        assert!(chunk.range.end >= samples_for_ms(2_300));
        assert!(chunk.range.start < samples_for_ms(900) && chunk.range.end > samples_for_ms(2_000));
    }

    #[test]
    fn short_whole_clip_uncertain_or_silent_audio_falls_back_safely() {
        let silent = silent_audio(2_000);
        let uncertain = vec![0.001; samples_for_ms(2_000)];

        for samples in [silent, uncertain] {
            let plan = plan_audio_chunks(&samples);

            assert!(!plan.short_trim.enabled);
            assert_eq!(
                plan.short_trim.fallback_reason,
                Some("no_confident_speech_region")
            );
            assert_eq!(
                plan.chunks[0].range,
                AudioRange {
                    start: 0,
                    end: samples.len()
                }
            );
        }
    }

    #[test]
    fn short_whole_clip_too_short_detection_falls_back_safely() {
        let mut samples = silent_audio(2_000);
        add_tone(&mut samples, 900, 1_000);

        let plan = plan_audio_chunks(&samples);

        assert!(!plan.short_trim.enabled);
        assert_eq!(
            plan.short_trim.fallback_reason,
            Some("detected_speech_too_short")
        );
        assert_eq!(
            plan.chunks[0].range,
            AudioRange {
                start: 0,
                end: samples.len()
            }
        );
    }

    #[test]
    fn protected_first_15_seconds_remain_one_segment() {
        let mut samples = silent_audio(30_000);
        add_tone(&mut samples, 1_000, 2_000);
        add_tone(&mut samples, 10_000, 11_000);

        let plan = plan_audio_chunks(&samples);
        let chunk = plan.chunks[0];

        assert_eq!(plan.chunks.len(), 1);
        assert_eq!(chunk.source, ChunkSource::ProtectedHead);
        assert!(chunk.range.start <= samples_for_ms(1_000));
        assert!(chunk.range.end >= samples_for_ms(11_000));
        assert!(chunk.range.end <= samples_for_ms(PROTECTED_HEAD_MS));
    }

    #[test]
    fn separated_speech_regions_stay_in_time_order() {
        let mut samples = silent_audio(35_000);
        add_tone(&mut samples, 16_000, 17_000);
        add_tone(&mut samples, 22_000, 23_000);
        add_tone(&mut samples, 30_000, 31_000);

        let plan = plan_audio_chunks(&samples);

        assert_eq!(plan.chunks.len(), 3);
        assert!(
            plan.chunks
                .windows(2)
                .all(|pair| pair[0].range.end < pair[1].range.start)
        );
    }

    #[test]
    fn nearby_speech_regions_within_900ms_are_merged() {
        let mut samples = silent_audio(30_000);
        add_tone(&mut samples, 16_000, 17_000);
        add_tone(&mut samples, 17_700, 18_000);

        let plan = plan_audio_chunks(&samples);
        let chunk = plan.chunks[0];

        assert_eq!(plan.chunks.len(), 1);
        assert!(chunk.range.start <= samples_for_ms(16_000));
        assert!(chunk.range.end >= samples_for_ms(18_000));
    }

    #[test]
    fn quiet_speech_uses_300ms_pre_padding_and_500ms_post_padding() {
        let mut samples = silent_audio(30_000);
        let start = samples_for_ms(20_000);
        let end = samples_for_ms(21_000);
        samples[start..end].fill(0.003);

        let plan = plan_audio_chunks(&samples);

        assert_eq!(plan.vad_threshold, VAD_MIN_RMS);
        assert_eq!(plan.chunks.len(), 1);
        assert_eq!(samples_to_ms(plan.chunks[0].range.start), 19_700);
        assert_eq!(samples_to_ms(plan.chunks[0].range.end), 21_500);
    }

    #[test]
    fn long_regions_are_capped_at_25_seconds() {
        let mut samples = silent_audio(70_000);
        add_tone(&mut samples, 15_000, 69_000);

        let plan = plan_audio_chunks(&samples);

        assert!(plan.chunks.len() >= 3);
        assert!(plan.hard_cut_count >= 2);
        assert!(
            plan.chunks
                .iter()
                .all(|chunk| duration_ms(chunk.range) <= MAX_CHUNK_MS)
        );
        assert!(
            plan.chunks
                .iter()
                .all(|chunk| chunk.source == ChunkSource::HardCut)
        );
    }

    #[test]
    fn continuous_speech_hard_cuts_preserve_overlap() {
        let mut samples = silent_audio(60_000);
        add_tone(&mut samples, 15_000, 60_000);

        let plan = plan_audio_chunks(&samples);

        assert_eq!(plan.chunks.len(), 2);
        assert_eq!(plan.hard_cut_count, 1);
        assert_eq!(
            duration_ms(plan.chunks[0].range),
            PREFERRED_CHUNK_MS + HARD_CUT_OVERLAP_MS
        );
        assert_eq!(
            samples_to_ms(
                plan.chunks[0]
                    .range
                    .end
                    .saturating_sub(plan.chunks[1].range.start)
            ),
            HARD_CUT_OVERLAP_MS
        );
    }

    #[test]
    fn silence_only_long_audio_creates_no_chunks() {
        let samples = silent_audio(30_000);

        assert!(plan_audio_chunks(&samples).chunks.is_empty());
    }

    #[test]
    fn short_voiced_blips_are_counted_as_dropped_regions() {
        let mut samples = silent_audio(30_000);
        add_tone(&mut samples, 20_000, 20_020);

        let plan = plan_audio_chunks(&samples);

        assert_eq!(plan.dropped_region_count, 1);
        assert_eq!(samples_to_ms(plan.dropped_region_total_samples), 20);
        assert!(plan.chunks.is_empty());
    }

    #[test]
    fn any_chunk_failure_fails_the_full_transcription() {
        let samples = vec![0.1; 100];
        let chunks = vec![
            planned_chunk(AudioRange { start: 0, end: 50 }),
            planned_chunk(AudioRange {
                start: 50,
                end: 100,
            }),
        ];
        let mut calls = 0;

        let result = transcribe_chunks(&samples, &chunks, |index, _, _| {
            calls += 1;
            if index == 1 {
                Err("worker timed out".to_string())
            } else {
                Ok("first chunk")
            }
        });

        assert_eq!(calls, 2);
        assert_eq!(
            result.expect_err("a chunk failure must fail the whole transcription"),
            "local ASR chunk 2/2 failed: worker timed out"
        );
    }

    #[test]
    fn chunk_outputs_can_be_joined_in_order_while_skipping_empty_text() {
        let samples = vec![0.1; 120];
        let chunks = vec![
            planned_chunk(AudioRange { start: 0, end: 40 }),
            planned_chunk(AudioRange { start: 40, end: 80 }),
            planned_chunk(AudioRange {
                start: 80,
                end: 120,
            }),
        ];

        let outputs = transcribe_chunks(&samples, &chunks, |index, _, _| {
            Ok(["first", " ", "third"][index].to_string())
        })
        .expect("all chunks should succeed");
        let merged = outputs
            .iter()
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        assert_eq!(merged, "first third");
    }
}
