//! Visualizer data. mpv doesn't hand out the audio it plays, so a second,
//! lightweight decoder reads the same file and keeps a little of it decoded
//! around the playback position. `Spectrum` turns that into the same numbers
//! the Electron app got from a Web Audio `AnalyserNode` (fftSize 2048,
//! smoothing 0.8, -100..-30 dB), so the visualizers draw the same way.

use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use parking_lot::{Condvar, Mutex};
use realfft::{RealFftPlanner, RealToComplex};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Time, TimeBase};

pub const FFT_SIZE: usize = 2048;
pub const BINS: usize = FFT_SIZE / 2;

/// How far ahead of the playback position to keep decoded.
const LOOKAHEAD: f64 = 0.35;
/// How much already-played audio to hold on to.
const KEEP_BEHIND: f64 = 0.5;
/// A jump further than this from the decoded window means the user seeked.
const SEEK_SLACK: f64 = 1.5;

#[derive(Default)]
struct Window {
    rate: u32,
    /// Mono samples starting at `start` seconds.
    samples: VecDeque<f32>,
    start: f64,
    eof: bool,
}

impl Window {
    fn end(&self) -> f64 {
        if self.rate == 0 {
            self.start
        } else {
            self.start + self.samples.len() as f64 / self.rate as f64
        }
    }
}

struct Request {
    path: Option<PathBuf>,
    generation: u64,
    quit: bool,
}

struct Shared {
    request: Mutex<Request>,
    window: Mutex<Window>,
    wake: Condvar,
    /// The playback position the UI is drawing, as f64 bits.
    position: AtomicU64,
}

/// Decodes the playing file in the background, just around where playback is.
pub struct Analyzer {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl Analyzer {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            request: Mutex::new(Request { path: None, generation: 0, quit: false }),
            window: Mutex::new(Window::default()),
            wake: Condvar::new(),
            position: AtomicU64::new(0f64.to_bits()),
        });
        let worker = {
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("lumen-analyzer".into())
                .spawn(move || worker(shared))
                .ok()
        };
        Analyzer { shared, worker }
    }

    /// Start following a new file (or nothing).
    pub fn load(&self, path: Option<&Path>) {
        let mut req = self.shared.request.lock();
        req.path = path.map(Path::to_path_buf);
        req.generation += 1;
        self.shared.position.store(0f64.to_bits(), Ordering::Relaxed);
        *self.shared.window.lock() = Window::default();
        self.shared.wake.notify_one();
    }

    /// Copy the `out.len()` mono samples that end at `position` seconds.
    /// Returns false (and fills silence) when that stretch isn't decoded.
    pub fn read(&self, position: f64, out: &mut [f32]) -> bool {
        let old = f64::from_bits(self.shared.position.swap(position.to_bits(), Ordering::Relaxed));
        if (position - old).abs() > SEEK_SLACK {
            self.shared.wake.notify_one();
        }
        let w = self.shared.window.lock();
        out.fill(0.0);
        if w.rate == 0 || w.samples.is_empty() {
            return false;
        }
        let end = ((position - w.start) * w.rate as f64).round() as isize;
        if end <= 0 || end as usize > w.samples.len() + w.rate as usize / 20 {
            return false;
        }
        let end = (end as usize).min(w.samples.len());
        let n = out.len().min(end);
        let (dst_from, src_from) = (out.len() - n, end - n);
        for (d, s) in out[dst_from..].iter_mut().zip(w.samples.range(src_from..end)) {
            *d = *s;
        }
        true
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Analyzer {
    fn drop(&mut self) {
        self.shared.request.lock().quit = true;
        self.shared.wake.notify_one();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

struct Source {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    time_base: Option<TimeBase>,
    rate: u32,
    scratch: Vec<f32>,
}

impl Source {
    fn open(path: &Path) -> Option<Source> {
        let file = File::open(path).ok()?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let reader = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .ok()?;
        let track = reader.default_track(TrackType::Audio)?;
        let params = track.codec_params.as_ref()?.audio()?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .ok()?;
        let rate = params.sample_rate.unwrap_or(44_100);
        let (track_id, time_base) = (track.id, track.time_base);
        Some(Source { reader, decoder, track_id, time_base, rate, scratch: Vec::new() })
    }

    fn seek(&mut self, seconds: f64) -> Option<f64> {
        let time = Time::try_from_secs_f64(seconds.max(0.0))?;
        let to = SeekTo::Time { time, track_id: Some(self.track_id) };
        let seeked = self.reader.seek(SeekMode::Accurate, to).ok()?;
        self.decoder.reset();
        let tb = self.time_base?;
        Some(tb.calc_time(seeked.actual_ts)?.as_secs_f64())
    }

    /// Decode the next packet of our track into mono samples. Returns the
    /// packet's start time, or None at the end of the file.
    fn next(&mut self, mono: &mut Vec<f32>) -> Option<f64> {
        loop {
            let packet = match self.reader.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return None,
                Err(SymError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(_) => return None,
            };
            if packet.track_id != self.track_id {
                continue;
            }
            let at = self
                .time_base
                .and_then(|tb| tb.calc_time(packet.pts))
                .map(|t| t.as_secs_f64())
                .unwrap_or(f64::NAN);
            let buf = match self.decoder.decode(&packet) {
                Ok(b) => b,
                Err(SymError::DecodeError(_)) => continue,
                Err(_) => return None,
            };
            let channels = buf.spec().channels().count().max(1);
            self.rate = buf.spec().rate();
            self.scratch.resize(buf.samples_interleaved(), 0.0);
            buf.copy_to_slice_interleaved(&mut self.scratch[..]);
            mono.clear();
            // Web Audio's analyser downmixes to mono by averaging the channels.
            let scale = 1.0 / channels as f32;
            mono.extend(self.scratch.chunks_exact(channels).map(|f| f.iter().sum::<f32>() * scale));
            return Some(at);
        }
    }
}

fn worker(shared: Arc<Shared>) {
    let mut generation = 0;
    let mut source: Option<Source> = None;
    let mut mono = Vec::new();
    loop {
        let (path, gen_now) = {
            let req = shared.request.lock();
            if req.quit {
                return;
            }
            (req.path.clone(), req.generation)
        };
        if gen_now != generation {
            generation = gen_now;
            source = path.as_deref().and_then(Source::open);
            let mut w = shared.window.lock();
            *w = Window::default();
            if let Some(s) = &source {
                w.rate = s.rate;
            }
        }
        let Some(src) = source.as_mut() else {
            idle(&shared, Duration::from_millis(250));
            continue;
        };

        let want = f64::from_bits(shared.position.load(Ordering::Relaxed));
        let (start, end, eof) = {
            let w = shared.window.lock();
            (w.start, w.end(), w.eof)
        };

        // A seek: jump the decoder instead of decoding our way there.
        if want < start - 0.05 || want > end + SEEK_SLACK {
            let at = src.seek(want - 0.1).unwrap_or(want.max(0.0));
            let mut w = shared.window.lock();
            w.samples.clear();
            w.start = at;
            w.eof = false;
            continue;
        }

        if eof || end >= want + LOOKAHEAD {
            idle(&shared, Duration::from_millis(15));
            continue;
        }

        match src.next(&mut mono) {
            Some(at) => {
                // `load` takes request then window, so never hold window while
                // taking request.
                if shared.request.lock().generation != generation {
                    continue;
                }
                let mut w = shared.window.lock();
                w.rate = src.rate;
                // After a seek the first packets can start before the target,
                // or there can be a gap; line the window up with the packet.
                if at.is_finite() && (w.samples.is_empty() || (at - w.end()).abs() > 0.05) {
                    w.samples.clear();
                    w.start = at;
                }
                w.samples.extend(mono.iter().copied());
                let keep_from = want - KEEP_BEHIND;
                if keep_from > w.start {
                    let drop = (((keep_from - w.start) * w.rate as f64) as usize).min(w.samples.len());
                    w.samples.drain(..drop);
                    w.start += drop as f64 / w.rate as f64;
                }
            }
            None => shared.window.lock().eof = true,
        }
    }
}

fn idle(shared: &Shared, timeout: Duration) {
    let mut req = shared.request.lock();
    if !req.quit {
        shared.wake.wait_for(&mut req, timeout);
    }
}

/// One frame of visualizer input, shaped like the Electron app's `AudioFrame`.
#[derive(Clone)]
pub struct AudioFrame {
    /// Frequency magnitudes, 0..255.
    pub freq: [u8; BINS],
    /// Waveform, 0..255 with 128 as silence.
    pub wave: [u8; BINS],
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub level: f32,
}

impl Default for AudioFrame {
    fn default() -> Self {
        AudioFrame {
            freq: [0; BINS],
            wave: [128; BINS],
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            level: 0.0,
        }
    }
}

/// An `AnalyserNode` in software.
pub struct Spectrum {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    output: Vec<realfft::num_complex::Complex<f32>>,
    smoothed: Vec<f32>,
    pub samples: Vec<f32>,
    pub frame: AudioFrame,
}

const MIN_DB: f32 = -100.0;
const MAX_DB: f32 = -30.0;
const SMOOTHING: f32 = 0.8;

impl Spectrum {
    pub fn new() -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let n = FFT_SIZE as f32;
        // Blackman window, as the Web Audio spec defines it (alpha = 0.16).
        let window = (0..FFT_SIZE)
            .map(|i| {
                let x = i as f32 / n;
                0.42 - 0.5 * (std::f32::consts::TAU * x).cos()
                    + 0.08 * (2.0 * std::f32::consts::TAU * x).cos()
            })
            .collect();
        Spectrum {
            output: fft.make_output_vec(),
            input: vec![0.0; FFT_SIZE],
            fft,
            window,
            smoothed: vec![0.0; BINS],
            samples: vec![0.0; FFT_SIZE],
            frame: AudioFrame::default(),
        }
    }

    /// Analyse `self.samples` (the last FFT_SIZE samples up to now). `dt` is the
    /// time since the previous frame; smoothing is scaled to it so the decay
    /// looks the same at 30, 60 or 120 fps as it did at the browser's 60.
    pub fn analyse(&mut self, dt: f32) {
        for ((d, s), w) in self.input.iter_mut().zip(&self.samples).zip(&self.window) {
            *d = s * w;
        }
        if self.fft.process(&mut self.input, &mut self.output).is_err() {
            return;
        }
        let tau = SMOOTHING.powf((dt * 60.0).clamp(0.25, 4.0));
        let norm = 1.0 / FFT_SIZE as f32;
        let scale = 255.0 / (MAX_DB - MIN_DB);
        for k in 0..BINS {
            let mag = self.output[k].norm() * norm;
            let s = tau * self.smoothed[k] + (1.0 - tau) * mag;
            self.smoothed[k] = if s.is_finite() { s } else { 0.0 };
            let db = 20.0 * self.smoothed[k].max(1e-12).log10();
            self.frame.freq[k] = (scale * (db - MIN_DB)).clamp(0.0, 255.0) as u8;
        }
        let mut sum_sq = 0.0;
        for (k, s) in self.samples[..BINS].iter().enumerate() {
            self.frame.wave[k] = (128.0 * (1.0 + s)).clamp(0.0, 255.0) as u8;
            let v = (self.frame.wave[k] as f32 - 128.0) / 128.0;
            sum_sq += v * v;
        }
        self.frame.level = (sum_sq / BINS as f32).sqrt();
        let f = &self.frame.freq;
        let (b, m) = ((BINS as f32 * 0.08) as usize, (BINS as f32 * 0.4) as usize);
        self.frame.bass = band_avg(f, 0, b);
        self.frame.mid = band_avg(f, b, m);
        self.frame.treble = band_avg(f, m, BINS);
    }
}

impl Default for Spectrum {
    fn default() -> Self {
        Self::new()
    }
}

fn band_avg(data: &[u8], from: usize, to: usize) -> f32 {
    let to = to.min(data.len());
    if to <= from {
        return 0.0;
    }
    data[from..to].iter().map(|&v| v as f32).sum::<f32>() / (to - from) as f32 / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_lands_in_the_right_bin() {
        let mut s = Spectrum::new();
        let (rate, hz) = (48_000.0, 1_000.0);
        for (i, v) in s.samples.iter_mut().enumerate() {
            *v = (std::f32::consts::TAU * hz * i as f32 / rate).sin() * 0.5;
        }
        for _ in 0..30 {
            s.analyse(1.0 / 60.0);
        }
        let peak = (0..BINS).max_by_key(|&k| s.frame.freq[k]).unwrap();
        let expected = (hz / (rate / FFT_SIZE as f32)).round() as usize;
        assert!(peak.abs_diff(expected) <= 1, "peak at bin {peak}, expected ~{expected}");
        assert!(s.frame.freq[peak] > 200);
        assert!(s.frame.level > 0.3);
    }

    #[test]
    fn silence_reads_as_zero() {
        let mut s = Spectrum::new();
        s.analyse(1.0 / 60.0);
        assert!(s.frame.freq.iter().all(|&v| v == 0));
        assert!(s.frame.wave.iter().all(|&v| v == 128));
        assert_eq!(s.frame.level, 0.0);
    }
}
