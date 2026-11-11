//! Balance volume: how loud a song is on average (EBU R128 integrated
//! loudness, the measure Spotify and Apple Music normalise by), and the one
//! fixed gain that brings it to a common level. A fixed gain per song keeps
//! its dynamics as they are, unlike a live compressor.

use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};

use ebur128::{EbuR128, Mode};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::mpv::{EndReason, Event, Mpv};

/// Every song is brought to this level, in LUFS.
pub const TARGET: f64 = -16.0;
/// Never turn a song up or down by more than this many dB.
const MAX_DB: f64 = 12.0;
/// Turning up stops before the loudest sample would pass −1 dBFS.
const PEAK_CEILING: f64 = 0.89;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    /// Integrated loudness, LUFS.
    pub lufs: f64,
    /// Sample peak, linear (1.0 = full scale).
    pub peak: f64,
}

/// Decode the whole file and measure it. None if it's silent or can't be
/// decoded at all.
pub fn measure(path: &Path) -> Option<Loudness> {
    measure_decoded(path).or_else(|| measure_with_mpv(path))
}

/// Symphonia and the `ebur128` crate: quick, and no second mpv.
fn measure_decoded(path: &Path) -> Option<Loudness> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut reader = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .ok()?;
    let track = reader.default_track(TrackType::Audio)?;
    let track_id = track.id;
    let params = track.codec_params.as_ref()?.audio()?;
    let mut decoder = symphonia::default::get_codecs().make_audio_decoder(params, &AudioDecoderOptions::default()).ok()?;

    let mut meter: Option<(EbuR128, usize)> = None;
    let mut samples: Vec<f32> = Vec::new();
    loop {
        let packet = match reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(SymError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => break,
        };
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(b) => b,
            Err(SymError::DecodeError(_)) => continue,
            Err(_) => break,
        };
        let channels = buf.spec().channels().count().max(1);
        let (m, ch) = match &mut meter {
            Some(m) => m,
            None => meter.insert((EbuR128::new(channels as u32, buf.spec().rate(), Mode::I | Mode::SAMPLE_PEAK).ok()?, channels)),
        };
        if *ch != channels {
            break; // the layout changed mid-file; what we have is plenty
        }
        samples.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut samples[..]);
        m.add_frames_f32(&samples).ok()?;
    }

    let (m, channels) = meter?;
    let lufs = m.loudness_global().ok()?;
    let peak = (0..channels as u32).filter_map(|c| m.sample_peak(c).ok()).fold(0.0, f64::max);
    (lufs.is_finite() && lufs > -70.0).then_some(Loudness { lufs, peak })
}

/// For what symphonia can't decode (Opus, and the odd MP3 it rejects): have
/// a silent second mpv decode it to a temporary WAV as fast as it can, then
/// measure that. (Reading FFmpeg's ebur128 filter results through
/// `af-metadata` instead crashes libmpv 0.37, which Ubuntu ships.)
fn measure_with_mpv(path: &Path) -> Option<Loudness> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.subsec_nanos();
    let wav = std::env::temp_dir().join(format!("lumen-loudness-{}-{nanos}.wav", std::process::id()));
    let decoded = decode_with_mpv(path, &wav);
    let result = decoded.then(|| measure_decoded(&wav)).flatten();
    let _ = std::fs::remove_file(&wav);
    result
}

fn decode_with_mpv(path: &Path, wav: &Path) -> bool {
    let Some(out) = wav.to_str() else { return false };
    let Ok(mpv) = Mpv::new(&[
        ("vo", "null"),
        ("vid", "no"),
        ("ao", "pcm"),
        ("ao-pcm-file", out),
        ("ao-pcm-waveheader", "yes"),
        ("audio-format", "floatp"),
        ("audio-display", "no"),
        ("ytdl", "no"),
        ("load-scripts", "no"),
        ("terminal", "no"),
    ]) else {
        return false;
    };
    let Some(input) = path.to_str() else { return false };
    if mpv.command(&["loadfile", input]).is_err() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_secs(120);
    let finished = loop {
        if Instant::now() > deadline {
            break false;
        }
        if let Some(Event::EndFile { reason, .. }) = mpv.wait_event(1.0) {
            break reason == EndReason::Eof;
        }
    };
    // The WAV header gets its final sizes when mpv closes the file.
    drop(mpv);
    finished
}

/// The volume multiplier that brings a song to [`TARGET`]: within ±12 dB,
/// and never turned up far enough to clip.
pub fn gain(lufs: f64, peak: f64) -> f64 {
    let db = (TARGET - lufs).clamp(-MAX_DB, MAX_DB);
    let mut g = 10f64.powf(db / 20.0);
    if g > 1.0 && peak > 0.0 {
        g = g.min((PEAK_CEILING / peak).max(1.0));
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(path: &Path, amplitude: f32, secs: f32) {
        let rate = 44_100u32;
        let n = (rate as f32 * secs) as u32;
        let mut data = Vec::with_capacity(n as usize * 4);
        for i in 0..n {
            let v = (amplitude * (i as f32 * 1000.0 * std::f32::consts::TAU / rate as f32).sin() * 32767.0) as i16;
            data.extend_from_slice(&v.to_le_bytes());
            data.extend_from_slice(&v.to_le_bytes());
        }
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&2u16.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * 4).to_le_bytes());
        f.extend_from_slice(&4u16.to_le_bytes());
        f.extend_from_slice(&16u16.to_le_bytes());
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(data.len() as u32).to_le_bytes());
        f.extend_from_slice(&data);
        std::fs::write(path, f).unwrap();
    }

    #[test]
    fn measures_a_sine() {
        let dir = std::env::temp_dir().join(format!("lumen-loudness-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tone.wav");
        // A stereo 1 kHz sine at −20 dBFS peak measures about −20 LUFS
        // (−23 per channel, +3 for two channels, ~0 from K-weighting at 1 kHz).
        write_wav(&path, 0.1, 5.0);
        let l = measure(&path).unwrap();
        assert!((l.lufs - -20.0).abs() < 0.7, "{l:?}");
        assert!((l.peak - 0.1).abs() < 0.01, "{l:?}");
        let m = measure_with_mpv(&path).unwrap();
        assert!((m.lufs - l.lufs).abs() < 0.3 && (m.peak - l.peak).abs() < 0.01, "{m:?} vs {l:?}");
        write_wav(&path, 0.0, 2.0);
        assert_eq!(measure(&path), None, "silence has no loudness");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gain_turns_loud_down_and_quiet_up_without_clipping() {
        let db = |g: f64| 20.0 * g.log10();
        assert!((db(gain(-10.0, 1.0)) - -6.0).abs() < 1e-9); // a loud master comes down 6 dB
        assert!((db(gain(-22.0, 0.2)) - 6.0).abs() < 1e-9); // a quiet one goes up 6 dB
        assert!((db(gain(-40.0, 0.01)) - 12.0).abs() < 1e-9); // but never more than 12
        assert!((gain(-22.0, 0.6) - 0.89 / 0.6).abs() < 1e-9); // and not into clipping
        assert_eq!(gain(-22.0, 1.0), 1.0); // already peaking: leave it
    }
}
