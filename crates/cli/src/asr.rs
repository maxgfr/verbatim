//! Speech to text with NVIDIA Parakeet TDT 0.6B v3 (25 European languages,
//! found by the model, timed to the word) through ONNX Runtime. The same
//! int8 export the site runs (istupakov's), pinned by revision and SHA-256,
//! downloaded once into the cache.

use crate::media::SAMPLE_RATE;
use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use parakeet_rs::{ExecutionConfig, ParakeetTDT, TimestampMode, Transcriber};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use verbatim_edl::{Piece, Transcript, join_words, segments_from_words};

pub const MODEL_NAME: &str = "parakeet-tdt-0.6b-v3-int8";
const REPO: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";
const REVISION: &str = "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";
/// (name, bytes, sha256)
const FILES: &[(&str, u64, &str)] = &[
    (
        "encoder-model.int8.onnx",
        652_183_999,
        "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
    ),
    (
        "decoder_joint-model.int8.onnx",
        18_202_004,
        "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
    ),
    (
        "vocab.txt",
        93_939,
        "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    ),
];

/// Where models and cached transcripts live: `$VERBATIM_HOME`, else the OS cache directory.
pub fn home() -> PathBuf {
    if let Some(h) = std::env::var_os("VERBATIM_HOME") {
        return PathBuf::from(h);
    }
    let user = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if cfg!(target_os = "macos") {
        user.join("Library/Caches/verbatim")
    } else if let Some(x) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(x).join("verbatim")
    } else {
        user.join(".cache/verbatim")
    }
}

pub fn model_dir() -> PathBuf {
    home().join("models").join(MODEL_NAME)
}

pub fn model_size() -> u64 {
    FILES.iter().map(|f| f.1).sum()
}

pub fn model_present() -> bool {
    let dir = model_dir();
    FILES
        .iter()
        .all(|(name, size, _)| std::fs::metadata(dir.join(name)).is_ok_and(|m| m.len() == *size))
}

/// Download whatever is missing, checking every file's SHA-256.
pub fn ensure_model(quiet: bool) -> Result<PathBuf> {
    let dir = model_dir();
    if model_present() {
        return Ok(dir);
    }
    std::fs::create_dir_all(&dir)?;
    let client = reqwest::blocking::Client::builder().timeout(None).build()?;
    if !quiet {
        eprintln!(
            "Downloading Parakeet TDT 0.6B v3 ({} MB, once) into {}",
            model_size() / 1_000_000,
            dir.display()
        );
    }
    for (name, size, sha) in FILES {
        let path = dir.join(name);
        if std::fs::metadata(&path).is_ok_and(|m| m.len() == *size) {
            continue;
        }
        let url = format!("https://huggingface.co/{REPO}/resolve/{REVISION}/{name}");
        let mut res = client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .with_context(|| format!("downloading {url}"))?;
        let bar = if quiet {
            ProgressBar::hidden()
        } else {
            ProgressBar::new(*size)
        };
        bar.set_style(
            ProgressStyle::with_template("{msg:28} {bar:30} {bytes}/{total_bytes} {eta}")
                .expect("valid template"),
        );
        bar.set_message(name.to_string());
        let part = path.with_extension("part");
        let mut file = std::fs::File::create(&part)?;
        let mut hash = Sha256::new();
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = res.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n])?;
            bar.inc(n as u64);
        }
        bar.finish_and_clear();
        let got = hex::encode(hash.finalize());
        if got != *sha {
            let _ = std::fs::remove_file(&part);
            bail!("{name}: SHA-256 {got} does not match the pinned {sha}");
        }
        std::fs::rename(&part, &path)?;
    }
    Ok(dir)
}

pub fn remove_model() -> Result<()> {
    let dir = model_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

/// Where to cut long audio: chunks of at most `max` seconds, each ending at the
/// quietest 100 ms between `min` and `max` seconds in.
pub fn chunks(samples: &[f32], min: f64, max: f64) -> Vec<(usize, usize)> {
    let rate = SAMPLE_RATE as usize;
    let (lo, hi) = ((min * rate as f64) as usize, (max * rate as f64) as usize);
    let frame = rate / 10;
    let mut out = Vec::new();
    let mut at = 0;
    while samples.len() - at > hi {
        let mut best = (f32::INFINITY, at + hi);
        let mut f = at + lo;
        while f + frame <= at + hi {
            let energy: f32 = samples[f..f + frame].iter().map(|s| s * s).sum();
            if energy < best.0 {
                best = (energy, f + frame / 2);
            }
            f += frame;
        }
        out.push((at, best.1));
        at = best.1;
    }
    if at < samples.len() {
        out.push((at, samples.len()));
    }
    out
}

pub fn transcribe(samples: Vec<f32>, quiet: bool) -> Result<Transcript> {
    let dir = ensure_model(quiet)?;
    let env = parakeet_rs::ort::init()
        .with_name("verbatim")
        .build()
        .context("starting ONNX Runtime")?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let config = ExecutionConfig::new()
        .with_environment(env)
        .with_intra_threads(threads);
    let mut model = ParakeetTDT::from_pretrained(&dir, Some(config)).context("loading Parakeet")?;
    let duration = samples.len() as f64 / f64::from(SAMPLE_RATE);
    let spans = chunks(&samples, 60.0, 90.0);
    let bar = if quiet {
        ProgressBar::hidden()
    } else {
        ProgressBar::new((duration * 1000.0) as u64)
    };
    bar.set_style(
        ProgressStyle::with_template("Transcribing {bar:30} {percent:>3}% {elapsed} (eta {eta})")
            .expect("valid template"),
    );
    let mut words: Vec<Piece> = Vec::new();
    for (a, b) in spans {
        let offset = a as f64 / f64::from(SAMPLE_RATE);
        // A chunk that is all silence has nothing to say.
        if samples[a..b].iter().all(|s| s.abs() < 1e-4) {
            bar.inc(((b - a) as f64 / 16.0) as u64);
            continue;
        }
        let r = model
            .transcribe_samples(
                samples[a..b].to_vec(),
                SAMPLE_RATE,
                1,
                Some(TimestampMode::Words),
            )
            .context("Parakeet failed")?;
        words.extend(
            r.tokens
                .into_iter()
                .filter(|t| !t.text.trim().is_empty())
                .map(|t| Piece {
                    text: t.text.trim().to_string(),
                    start: offset + f64::from(t.start),
                    end: offset + f64::from(t.end),
                }),
        );
        bar.inc(((b - a) as f64 / 16.0) as u64);
    }
    bar.finish_and_clear();
    let segments = segments_from_words(&words, 1.0, 30);
    Ok(Transcript {
        text: join_words(segments.iter().map(|s| s.text.as_str())),
        language: None,
        duration: Some(duration),
        segments,
        words,
        word_timings: "exact".into(),
    })
}

/// A cached transcript for this file, keyed by path, size and modification time.
pub fn cache_path(media: &Path) -> Result<PathBuf> {
    let abs = std::fs::canonicalize(media)
        .with_context(|| format!("{} does not exist", media.display()))?;
    let meta = std::fs::metadata(&abs)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let key = hex::encode(Sha256::digest(format!(
        "{}\n{}\n{mtime}\n{MODEL_NAME}",
        abs.display(),
        meta.len()
    )));
    Ok(home()
        .join("transcripts")
        .join(format!("{}.json", &key[..24])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_cut_at_the_quietest_moment() {
        let rate = SAMPLE_RATE as usize;
        let mut s = vec![0.5f32; rate * 200];
        // A quiet moment at 75 s.
        for x in &mut s[rate * 75..rate * 75 + rate / 5] {
            *x = 0.0;
        }
        let c = chunks(&s, 60.0, 90.0);
        assert_eq!(c.len(), 3);
        assert!((c[0].1 as f64 / rate as f64 - 75.05).abs() < 0.2, "{c:?}");
        assert_eq!(c.last().unwrap().1, s.len());
        assert!(c.windows(2).all(|w| w[0].1 == w[1].0));
        assert_eq!(chunks(&s[..rate * 10], 60.0, 90.0), vec![(0, rate * 10)]);
    }
}
