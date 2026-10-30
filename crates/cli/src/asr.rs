//! Speech to text through ONNX Runtime, with the two NVIDIA models the site
//! runs (istupakov's int8 exports), pinned by revision and SHA-256 and
//! downloaded once into the cache:
//!
//! - Parakeet TDT 0.6B v3: 25 European languages, found by the model (it
//!   cannot be told one), timed to the word.
//! - Canary 180M Flash: English, French, German and Spanish, told the
//!   language; phrases timed between pauses, words spread inside them. Its
//!   features and decoding follow llm-connect (`nemo.js`) so the CLI and the
//!   page hear the same thing.

use crate::media::SAMPLE_RATE;
use crate::mel;
use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use parakeet_rs::ort::session::Session;
use parakeet_rs::ort::value::{DynValue, Tensor};
use parakeet_rs::{ExecutionConfig, ParakeetTDT, TimestampMode, Transcriber};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use verbatim_edl::{Piece, Transcript, join_words, segments_from_words, words_from_segments};

/// A model's files, pinned.
pub struct Spec {
    pub id: &'static str,
    pub label: &'static str,
    repo: &'static str,
    revision: &'static str,
    /// (name, bytes, sha256)
    files: &'static [(&'static str, u64, &'static str)],
}

pub const PARAKEET: Spec = Spec {
    id: "parakeet-tdt-0.6b-v3-int8",
    label: "Parakeet TDT 0.6B v3",
    repo: "istupakov/parakeet-tdt-0.6b-v3-onnx",
    revision: "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce",
    files: &[
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
    ],
};

pub const CANARY: Spec = Spec {
    id: "canary-180m-flash-int8",
    label: "Canary 180M Flash",
    repo: "istupakov/canary-180m-flash-onnx",
    revision: "92c2231a4e2b2524277fea759be967d2e6edfc49",
    files: &[
        (
            "encoder-model.int8.onnx",
            133_710_896,
            "996d1c89e6cbc891a7c88bf410884c178ffa474f7b13084522ac74a5e144cc81",
        ),
        (
            "decoder-model.int8.onnx",
            79_520_211,
            "9dd9c447872088c912e916d73751f9621a54085d5bc46788454fe904db51a914",
        ),
        (
            "vocab.txt",
            53_555,
            "2dae6fc7815f9640645e0c765522b278ee0cef49b482d91f6913e334628d3e77",
        ),
    ],
};

/// The languages Canary is told.
pub const CANARY_LANGUAGES: [&str; 4] = ["en", "fr", "de", "es"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum EngineChoice {
    /// Parakeet, or Canary when `--lang` names one of its languages.
    #[default]
    Auto,
    Parakeet,
    Canary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Engine {
    Parakeet,
    Canary { language: String },
}

impl Engine {
    /// Parakeet guesses the language and takes accented speech for English (a
    /// French talk came out as English): a language the user names goes to
    /// Canary when Canary knows it, as the site does.
    pub fn resolve(choice: EngineChoice, lang: Option<&str>) -> Result<Engine> {
        let lang = lang
            .map(|l| {
                l.trim()
                    .to_lowercase()
                    .split(['-', '_'])
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .filter(|l| !l.is_empty() && l != "auto");
        let canary_knows = lang
            .as_deref()
            .is_some_and(|l| CANARY_LANGUAGES.contains(&l));
        Ok(match (choice, lang) {
            (EngineChoice::Parakeet, _) => Engine::Parakeet,
            (EngineChoice::Canary, Some(l)) if canary_knows => Engine::Canary { language: l },
            (EngineChoice::Canary, Some(l)) => bail!(
                "Canary transcribes English, French, German and Spanish, not \"{l}\": use --engine parakeet"
            ),
            (EngineChoice::Canary, None) => {
                bail!("Canary needs the spoken language: --lang en, fr, de or es")
            }
            (EngineChoice::Auto, Some(l)) if canary_knows => Engine::Canary { language: l },
            (EngineChoice::Auto, _) => Engine::Parakeet,
        })
    }

    pub fn spec(&self) -> &'static Spec {
        match self {
            Engine::Parakeet => &PARAKEET,
            Engine::Canary { .. } => &CANARY,
        }
    }

    /// Part of the transcript cache key.
    pub fn key(&self) -> String {
        match self {
            Engine::Parakeet => PARAKEET.id.into(),
            Engine::Canary { language } => format!("{}:{language}", CANARY.id),
        }
    }
}

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

impl Spec {
    pub fn dir(&self) -> PathBuf {
        home().join("models").join(self.id)
    }

    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.1).sum()
    }

    pub fn present(&self) -> bool {
        let dir = self.dir();
        self.files.iter().all(|(name, size, _)| {
            std::fs::metadata(dir.join(name)).is_ok_and(|m| m.len() == *size)
        })
    }

    /// Download whatever is missing, checking every file's SHA-256.
    pub fn ensure(&self, quiet: bool) -> Result<PathBuf> {
        let dir = self.dir();
        if self.present() {
            return Ok(dir);
        }
        std::fs::create_dir_all(&dir)?;
        let client = reqwest::blocking::Client::builder().timeout(None).build()?;
        if !quiet {
            eprintln!(
                "Downloading {} ({} MB, once) into {}",
                self.label,
                self.size() / 1_000_000,
                dir.display()
            );
        }
        for (name, size, sha) in self.files {
            let path = dir.join(name);
            if std::fs::metadata(&path).is_ok_and(|m| m.len() == *size) {
                continue;
            }
            let url = format!(
                "https://huggingface.co/{}/resolve/{}/{name}",
                self.repo, self.revision
            );
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

    pub fn remove(&self) -> Result<()> {
        let dir = self.dir();
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}

/// Where to cut long audio for Parakeet: chunks of at most `max` seconds, each
/// ending at the quietest 100 ms between `min` and `max` seconds in.
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

fn runtime() -> Result<ExecutionConfig> {
    let env = parakeet_rs::ort::init()
        .with_name("verbatim")
        .build()
        .context("starting ONNX Runtime")?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    Ok(ExecutionConfig::new()
        .with_environment(env)
        .with_intra_threads(threads))
}

fn progress(quiet: bool, duration: f64) -> ProgressBar {
    let bar = if quiet {
        ProgressBar::hidden()
    } else {
        ProgressBar::new((duration * 1000.0) as u64)
    };
    bar.set_style(
        ProgressStyle::with_template("Transcribing {bar:30} {percent:>3}% {elapsed} (eta {eta})")
            .expect("valid template"),
    );
    bar
}

pub fn transcribe(samples: Vec<f32>, engine: &Engine, quiet: bool) -> Result<Transcript> {
    match engine {
        Engine::Parakeet => parakeet(samples, quiet),
        Engine::Canary { language } => canary(&samples, language, quiet),
    }
}

fn parakeet(samples: Vec<f32>, quiet: bool) -> Result<Transcript> {
    let dir = PARAKEET.ensure(quiet)?;
    let mut model =
        ParakeetTDT::from_pretrained(&dir, Some(runtime()?)).context("loading Parakeet")?;
    let duration = samples.len() as f64 / f64::from(SAMPLE_RATE);
    let bar = progress(quiet, duration);
    let mut words: Vec<Piece> = Vec::new();
    for (a, b) in chunks(&samples, 60.0, 90.0) {
        let offset = a as f64 / f64::from(SAMPLE_RATE);
        // A chunk that is all silence has nothing to say.
        if !samples[a..b].iter().all(|s| s.abs() < 1e-4) {
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
        }
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

/// `vocab.txt`: one `token id` per line, `▁` standing for a space.
pub fn parse_vocab(text: &str) -> Vec<String> {
    let mut vocab: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some(i) = line.rfind(' ') else { continue };
        let Ok(id) = line[i + 1..].parse::<usize>() else {
            continue;
        };
        if vocab.len() <= id {
            vocab.resize(id + 1, String::new());
        }
        vocab[id] = line[..i].replace('▁', " ");
    }
    vocab
}

struct Canary {
    encoder: Session,
    decoder: Session,
    vocab: Vec<String>,
}

impl Canary {
    fn load() -> Result<Self> {
        let dir = CANARY.dir();
        let config = runtime()?;
        let encoder = config
            .build_session(&dir.join("encoder-model.int8.onnx"))
            .context("loading Canary's encoder")?;
        let decoder = config
            .build_session(&dir.join("decoder-model.int8.onnx"))
            .context("loading Canary's decoder")?;
        let vocab = parse_vocab(&std::fs::read_to_string(dir.join("vocab.txt"))?);
        Ok(Self {
            encoder,
            decoder,
            vocab,
        })
    }

    /// The tokenizer joins one vocabulary per language, so a token can be there
    /// several times: the prompt takes the last one, as NeMo's lookup does.
    fn id(&self, token: &str) -> Result<i64> {
        self.vocab
            .iter()
            .rposition(|t| t == token)
            .map(|i| i as i64)
            .with_context(|| format!("Canary has no {token} token"))
    }

    /// Greedy decoding of the transcription task for one phrase, without timestamps.
    fn text(&mut self, samples: &[f32], language: &str) -> Result<String> {
        let (features, frames) = mel::features(samples, SAMPLE_RATE, 128);
        let mut enc = self.encoder.run(parakeet_rs::ort::inputs!(
            "audio_signal" => Tensor::from_array(([1usize, 128, frames], features))?,
            "length" => Tensor::from_array(([1usize], vec![frames as i64]))?
        ))?;
        let embeddings: DynValue = enc
            .remove("encoder_embeddings")
            .context("Canary's encoder gave no embeddings")?;
        let mask: DynValue = enc
            .remove("encoder_mask")
            .context("Canary's encoder gave no mask")?;
        drop(enc);
        let lang = format!("<|{language}|>");
        let mut tokens: Vec<i64> = [
            " ",
            "<|startofcontext|>",
            "<|startoftranscript|>",
            "<|emo:undefined|>",
            &lang,
            &lang,
            "<|pnc|>",
            "<|noitn|>",
            "<|notimestamp|>",
            "<|nodiarize|>",
        ]
        .iter()
        .map(|t| self.id(t))
        .collect::<Result<_>>()?;
        let prompt = tokens.len();
        let end = self.id("<|endoftext|>")?;
        let vocab = self.vocab.len();
        let mut mems: DynValue =
            Tensor::from_array(([6usize, 1, 0, 1024], Vec::<f32>::new()))?.into_dyn();
        let mut first = true;
        // Speech runs at a few tokens a second: a phrase past that budget is a decoding loop.
        let seconds = samples.len() as f64 / f64::from(SAMPLE_RATE);
        let budget = prompt + ((seconds * 12.0) as usize + 16).min(512 - prompt);
        while tokens.len() < budget {
            let ids: Vec<i64> = if first {
                tokens.clone()
            } else {
                vec![*tokens.last().expect("prompt is not empty")]
            };
            let mut out = self.decoder.run(parakeet_rs::ort::inputs!(
                "input_ids" => Tensor::from_array(([1usize, ids.len()], ids))?,
                "encoder_embeddings" => &embeddings,
                "encoder_mask" => &mask,
                "decoder_mems" => &mems
            ))?;
            let (_, logits) = out["logits"].try_extract_tensor::<f32>()?;
            let tail = &logits[logits.len() - vocab..];
            let next = tail
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map_or(end, |(i, _)| i as i64);
            mems = out
                .remove("decoder_hidden_states")
                .context("Canary's decoder gave no state")?;
            first = false;
            if next == end {
                break;
            }
            tokens.push(next);
            if let Some(keep) = repeating_tail(&tokens[prompt..]) {
                tokens.truncate(prompt + keep);
                break;
            }
        }
        Ok(tokens[prompt..]
            .iter()
            .map(|&t| self.vocab.get(t as usize).map_or("", String::as_str))
            .filter(|t| !t.starts_with("<|"))
            .collect::<String>()
            .trim()
            .to_string())
    }
}

/// Greedy decoding can fall into a loop ("euh, euh, euh…" seventy times): when
/// the tail is one short pattern repeated more than `MAX_REPEATS` times, the
/// length to keep, the tokens before the loop and one occurrence of it.
const MAX_REPEATS: usize = 4;
fn repeating_tail(tokens: &[i64]) -> Option<usize> {
    for period in 1..=4 {
        let need = period * (MAX_REPEATS + 1);
        if tokens.len() < need {
            continue;
        }
        let tail = &tokens[tokens.len() - need..];
        if tail.chunks(period).all(|c| c == &tail[..period]) {
            // Walk back to where the loop started.
            let mut start = tokens.len() - need;
            while start >= period && tokens[start - period..start] == tail[..period] {
                start -= period;
            }
            return Some(start + period);
        }
    }
    None
}

fn canary(samples: &[f32], language: &str, quiet: bool) -> Result<Transcript> {
    CANARY.ensure(quiet)?;
    let mut model = Canary::load()?;
    let rate = f64::from(SAMPLE_RATE);
    let duration = samples.len() as f64 / rate;
    let bar = progress(quiet, duration);
    // Phrases between pauses: their time is exact, the words' is spread inside.
    let mut segments = Vec::new();
    for span in mel::speech_spans(samples, SAMPLE_RATE, 12.0, 0.25) {
        let (a, b) = (
            (span.start * rate) as usize,
            ((span.end * rate).ceil() as usize).min(samples.len()),
        );
        let text = model
            .text(&samples[a..b], language)
            .context("Canary failed")?;
        if !text.is_empty() {
            segments.push(Piece {
                text,
                start: span.speech_start,
                end: span.speech_end,
            });
        }
        bar.set_position((span.end * 1000.0) as u64);
    }
    bar.finish_and_clear();
    let words = words_from_segments(&segments);
    Ok(Transcript {
        text: join_words(segments.iter().map(|s| s.text.as_str())),
        language: Some(language.to_string()),
        duration: Some(duration),
        word_timings: if segments.is_empty() {
            "none"
        } else {
            "estimated"
        }
        .into(),
        segments,
        words,
    })
}

/// A cached transcript for this file and engine, keyed by path, size and modification time.
pub fn cache_path(media: &Path, engine: &Engine) -> Result<PathBuf> {
    let abs = std::fs::canonicalize(media)
        .with_context(|| format!("{} does not exist", media.display()))?;
    let meta = std::fs::metadata(&abs)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let key = hex::encode(Sha256::digest(format!(
        "{}\n{}\n{mtime}\n{}",
        abs.display(),
        meta.len(),
        engine.key()
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

    #[test]
    fn a_named_language_goes_to_canary_when_it_knows_it() {
        use EngineChoice::*;
        assert_eq!(Engine::resolve(Auto, None).unwrap(), Engine::Parakeet);
        assert_eq!(
            Engine::resolve(Auto, Some("fr")).unwrap(),
            Engine::Canary {
                language: "fr".into()
            }
        );
        assert_eq!(
            Engine::resolve(Auto, Some("FR-ca")).unwrap(),
            Engine::Canary {
                language: "fr".into()
            }
        );
        assert_eq!(Engine::resolve(Auto, Some("it")).unwrap(), Engine::Parakeet);
        assert_eq!(
            Engine::resolve(Auto, Some("auto")).unwrap(),
            Engine::Parakeet
        );
        assert_eq!(
            Engine::resolve(Parakeet, Some("fr")).unwrap(),
            Engine::Parakeet
        );
        assert!(Engine::resolve(Canary, None).is_err());
        assert!(Engine::resolve(Canary, Some("it")).is_err());
        assert_ne!(
            Engine::resolve(Auto, Some("fr")).unwrap().key(),
            Engine::resolve(Auto, Some("de")).unwrap().key()
        );
    }

    #[test]
    fn decoding_loops_are_cut_to_one_occurrence() {
        // "a b" then "x ," repeated: keep "a b x ,".
        let mut t = vec![1, 2];
        for _ in 0..6 {
            t.extend([7, 8]);
        }
        assert_eq!(repeating_tail(&t), Some(4));
        assert_eq!(repeating_tail(&[5; 5]), Some(1));
        // Four repeats are speech ("non non non non"), not a loop.
        assert_eq!(repeating_tail(&[1, 5, 5, 5, 5]), None);
        assert_eq!(repeating_tail(&[1, 2, 3, 4, 5, 6]), None);
    }

    #[test]
    fn vocab_keeps_spaces_and_ids() {
        let v = parse_vocab("<unk> 0\n▁le 1\n▁ 2\n<|fr|> 3\n");
        assert_eq!(v, vec!["<unk>", " le", " ", "<|fr|>"]);
    }
}
