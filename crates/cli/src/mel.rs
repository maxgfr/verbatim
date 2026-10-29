//! NeMo's log-mel front end and the pause finder, as llm-connect computes
//! them for the site (`mel.js`, `speechSpans` in `nemo.js`), so Canary hears
//! the same features and the same phrases in the CLI as in the page.
//!
//! Features: pre-emphasis 0.97, a 512-point STFT every 10 ms over a 25 ms
//! symmetric Hann window, Slaney mel bands, log with a 2^-24 guard, then each
//! band normalized over the audio.

const N_FFT: usize = 512;
const WIN_LENGTH: usize = 400;
const HOP: usize = 160;
const PREEMPH: f32 = 0.97;
const LOG_GUARD: f64 = 5.960_464_477_539_063e-8; // 2^-24
const BINS: usize = N_FFT / 2 + 1;

/// Slaney-style mel filters, `bins × mels`, as torchaudio's `melscale_fbanks(norm="slaney")`.
fn mel_filters(mels: usize, rate: f64) -> Vec<f64> {
    let to_mel = |f: f64| {
        if f < 1000.0 {
            3.0 * f / 200.0
        } else {
            15.0 + 27.0 * (f / 1000.0 + 1.192_092_9e-7).ln() / 6.4f64.ln()
        }
    };
    let to_hz = |m: f64| {
        if m < 15.0 {
            200.0 * m / 3.0
        } else {
            1000.0 * 6.4f64.powf((m - 15.0) / 27.0)
        }
    };
    let top = to_mel(rate / 2.0);
    let points: Vec<f64> = (0..mels + 2)
        .map(|i| to_hz(top * i as f64 / (mels + 1) as f64))
        .collect();
    let mut out = vec![0.0; BINS * mels];
    for b in 0..BINS {
        let f = rate / 2.0 * b as f64 / (BINS - 1) as f64;
        for m in 0..mels {
            let (lo, mid, hi) = (points[m], points[m + 1], points[m + 2]);
            let up = (f - lo) / (mid - lo);
            let down = (hi - f) / (hi - mid);
            out[b * mels + m] = up.min(down).max(0.0) * (2.0 / (hi - lo));
        }
    }
    out
}

/// In-place radix-2 FFT of `re + i·im` (length a power of two).
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (angle.cos(), angle.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (i + k, i + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = next;
            }
        }
        len <<= 1;
    }
}

/// Normalized log-mel features of mono samples, `mels × frames` band after band
/// (the layout of a `[1, mels, frames]` tensor), and the number of frames.
pub fn features(samples: &[f32], rate: u32, mels: usize) -> (Vec<f32>, usize) {
    let filters = mel_filters(mels, f64::from(rate));
    let n = samples.len();
    // NeMo counts every frame the centred STFT gives as audio.
    let frames = n / HOP + 1;
    let mut padded = vec![0.0f32; n + N_FFT];
    for i in 0..n {
        padded[i + N_FFT / 2] = samples[i] - if i > 0 { PREEMPH * samples[i - 1] } else { 0.0 };
    }
    let mut window = vec![0.0f64; N_FFT];
    let offset = (N_FFT - WIN_LENGTH) / 2;
    for i in 0..WIN_LENGTH {
        window[offset + i] =
            0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (WIN_LENGTH - 1) as f64).cos();
    }
    let mut data = vec![0.0f32; mels * frames];
    let (mut re, mut im, mut power) =
        (vec![0.0f64; N_FFT], vec![0.0f64; N_FFT], vec![0.0f64; BINS]);
    for t in 0..frames {
        let start = t * HOP;
        for i in 0..N_FFT {
            re[i] = f64::from(padded[start + i]) * window[i];
            im[i] = 0.0;
        }
        fft(&mut re, &mut im);
        for b in 0..BINS {
            power[b] = re[b] * re[b] + im[b] * im[b];
        }
        for m in 0..mels {
            let sum: f64 = (0..BINS).map(|b| power[b] * filters[b * mels + m]).sum();
            data[m * frames + t] = (sum + LOG_GUARD).ln() as f32;
        }
    }
    // Each band to zero mean and unit variance.
    for m in 0..mels {
        let row = &mut data[m * frames..(m + 1) * frames];
        let mean = row.iter().map(|&v| f64::from(v)).sum::<f64>() / frames.max(1) as f64;
        let variance = row
            .iter()
            .map(|&v| (f64::from(v) - mean).powi(2))
            .sum::<f64>()
            / (frames.max(2) - 1) as f64;
        let scale = 1.0 / (variance.sqrt() + 1e-5);
        for v in row.iter_mut() {
            *v = ((f64::from(*v) - mean) * scale) as f32;
        }
    }
    (data, frames)
}

const LEVEL_SECONDS: f64 = 0.02;

/// A stretch of audio in seconds, with the part of it that holds sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: f64,
    pub end: f64,
    pub speech_start: f64,
    pub speech_end: f64,
}

/// The audio cut into spans of at most `max_seconds`, at every pause of at
/// least `min_pause` seconds and, when a span is still too long, at its
/// quietest moment. Spans without any sound are left out.
pub fn speech_spans(samples: &[f32], rate: u32, max_seconds: f64, min_pause: f64) -> Vec<Span> {
    let size = ((f64::from(rate) * LEVEL_SECONDS).round() as usize).max(1);
    let lv: Vec<f32> = samples
        .chunks(size)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let total = lv.len();
    if total == 0 {
        return vec![];
    }
    let percentile = |p: f64| {
        let mut sorted = lv.clone();
        sorted.sort_by(f32::total_cmp);
        sorted[((p * sorted.len() as f64) as usize).min(sorted.len() - 1)]
    };
    // Quiet: near the noise floor, or far below the speech.
    let (floor, loud) = (percentile(0.1), percentile(0.95));
    let quiet = (floor * 3.0).min(loud * 0.3).max(loud * 0.05).max(1e-4);
    let is_quiet = |f: usize| lv[f] < quiet;
    let mut cuts = vec![0];
    let need = ((min_pause / LEVEL_SECONDS).round() as usize).max(1);
    let mut run = 0;
    for f in 0..=total {
        if f < total && is_quiet(f) {
            run += 1;
            continue;
        }
        if min_pause > 0.0 && run >= need && f > run && f < total {
            cuts.push(f - run.div_ceil(2));
        }
        run = 0;
    }
    cuts.push(total);
    let max = ((max_seconds / LEVEL_SECONDS) as usize).max(1);
    let mut bounds = vec![];
    for w in cuts.windows(2) {
        let (mut a, b) = (w[0], w[1]);
        while b - a > max {
            let from = a + max / 2;
            let mut best = from;
            for f in from..a + max {
                if lv[f] < lv[best] {
                    best = f;
                }
            }
            bounds.push((a, best));
            a = best;
        }
        bounds.push((a, b));
    }
    let duration = samples.len() as f64 / f64::from(rate);
    let seconds = |f: usize| (f as f64 * LEVEL_SECONDS).min(duration);
    bounds
        .into_iter()
        .filter_map(|(a, b)| {
            let mut s = a;
            while s < b && is_quiet(s) {
                s += 1;
            }
            if s == b {
                return None;
            }
            let mut e = b;
            while e > s && is_quiet(e - 1) {
                e -= 1;
            }
            Some(Span {
                start: seconds(a),
                end: seconds(b),
                speech_start: seconds(s),
                speech_end: seconds(e),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_have_nemo_shape_and_are_normalized() {
        let samples: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 16_000.0).sin() * 0.5)
            .collect();
        let (data, frames) = features(&samples, 16_000, 128);
        assert_eq!(frames, 16_000 / 160 + 1);
        assert_eq!(data.len(), 128 * frames);
        let row = &data[40 * frames..41 * frames];
        let mean: f32 = row.iter().sum::<f32>() / frames as f32;
        assert!(mean.abs() < 1e-3, "{mean}");
        assert!(data.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn spans_cut_at_pauses_and_skip_silence() {
        let rate = 16_000;
        let tone = |secs: f64| {
            (0..(secs * f64::from(rate)) as usize)
                .map(|i| (i as f32 * 0.05).sin() * 0.4)
                .collect::<Vec<_>>()
        };
        let mut audio = vec![0.0; rate as usize]; // 1 s silence
        audio.extend(tone(2.0));
        audio.extend(vec![0.0; rate as usize / 2]); // 0.5 s pause
        audio.extend(tone(3.0));
        let spans = speech_spans(&audio, rate, 12.0, 0.25);
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert!((spans[0].speech_start - 1.0).abs() < 0.05);
        assert!((spans[1].speech_end - 6.5).abs() < 0.05);
        // A long span is cut again.
        let long = tone(30.0);
        assert!(
            speech_spans(&long, rate, 12.0, 0.25)
                .iter()
                .all(|s| s.end - s.start <= 12.0 + 1e-9)
        );
    }
}
