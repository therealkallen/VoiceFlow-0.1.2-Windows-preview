use std::{sync::Mutex, thread};

use shared_protocol::SessionKind;
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT, SND_SYNC};

use crate::host::AudioFeedbackAdapter;

#[derive(Debug, Default)]
pub struct WindowsAudioFeedbackAdapter;
static PLAYBACK: Mutex<()> = Mutex::new(());
const RATE: u32 = 24_000;

impl AudioFeedbackAdapter for WindowsAudioFeedbackAdapter {
    fn session_armed(&self, _session_id: u64, session_kind: &SessionKind) {
        match session_kind {
            SessionKind::SelectedTextEdit => play_sequence_async(vec![(440.0, 75), (554.4, 85)]),
            _ => play_sequence_async(vec![(523.3, 110)]),
        }
    }

    fn recording_stopped(&self, _session_id: u64, session_kind: &SessionKind) {
        match session_kind {
            _ => play_sequence_async(vec![(392.0, 120)]),
        }
    }

    fn session_failed(&self, _session_id: u64, _session_kind: &SessionKind) {
        play_sequence_async(vec![(330.0, 100), (293.7, 120)]);
    }
}

fn play_sequence_async(tones: Vec<(f32, u32)>) {
    thread::spawn(move || {
        let wav = cue_wav(&tones);
        let Ok(_guard) = PLAYBACK.lock() else { return };
        // Keep the buffer alive during synchronous playback on this background
        // thread. Device failures stay silent instead of playing a system beep.
        unsafe {
            PlaySoundW(
                wav.as_ptr().cast(),
                std::ptr::null_mut(),
                SND_MEMORY | SND_NODEFAULT | SND_SYNC,
            );
        }
    });
}

fn cue_wav(notes: &[(f32, u32)]) -> Vec<u8> {
    let mut pcm = Vec::new();
    for (index, &(hz, ms)) in notes.iter().enumerate() {
        if index > 0 {
            pcm.extend(std::iter::repeat_n(0_i16, (RATE / 50) as usize));
        }
        let count = (RATE * ms / 1000) as usize;
        for i in 0..count {
            let attack = (i as f32 / (RATE as f32 * 0.02)).min(1.0);
            let release = ((count - 1 - i) as f32 / (RATE as f32 * 0.06)).min(1.0);
            let envelope = (attack * std::f32::consts::FRAC_PI_2).sin().powi(2)
                * (release * std::f32::consts::FRAC_PI_2).sin().powi(2);
            let phase = std::f32::consts::TAU * hz * i as f32 / RATE as f32;
            let tone = phase.sin() + 0.08 * (phase * 2.0).sin();
            pcm.push((tone * envelope * 0.075 * i16::MAX as f32).round() as i16);
        }
    }
    let len = (pcm.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + len as usize);
    wav.extend(b"RIFF");
    wav.extend((36 + len).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16_u32.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(RATE.to_le_bytes());
    wav.extend((RATE * 2).to_le_bytes());
    wav.extend(2_u16.to_le_bytes());
    wav.extend(16_u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(len.to_le_bytes());
    for sample in pcm {
        wav.extend(sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cues_have_quiet_endpoints_low_peak_and_no_click_steps() {
        for notes in [
            vec![(523.3, 110)],
            vec![(392.0, 120)],
            vec![(440.0, 75), (554.4, 85)],
            vec![(330.0, 100), (293.7, 120)],
        ] {
            let wav = cue_wav(&notes);
            assert_eq!(&wav[..4], b"RIFF");
            assert_eq!(
                wav.len() - 44,
                u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize
            );
            let samples: Vec<i16> = wav[44..]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect();
            assert_eq!(samples.first(), Some(&0));
            assert_eq!(samples.last(), Some(&0));
            assert!(samples.iter().all(|s| s.abs() < 3000));
            assert!(
                samples
                    .windows(2)
                    .all(|s| (s[1] as i32 - s[0] as i32).abs() < 450)
            );
            assert!(samples.iter().any(|s| s.abs() > 100));
        }
    }
    #[test]
    #[ignore = "exports preview WAV files without playing them"]
    fn export_cue_previews() {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/cue-preview");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, notes) in [
            ("start", vec![(523.3, 110)]),
            ("stop", vec![(392.0, 120)]),
            ("edit", vec![(440.0, 75), (554.4, 85)]),
            ("error", vec![(330.0, 100), (293.7, 120)]),
        ] {
            std::fs::write(dir.join(format!("{name}.wav")), cue_wav(&notes)).unwrap();
        }
    }
}
