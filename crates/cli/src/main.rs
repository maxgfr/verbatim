//! verbatim: transcribe and edit video and audio on this machine.
//!
//! info        what a media file holds (JSON)
//! transcribe  speech to text with Parakeet or Canary, every export format
//! segments    the transcript as numbered, timed segments (to choose cuts from)
//! plan        the cuts as a cut list (JSON), without touching the media
//! cut         write the media without the cut parts
//! export      a transcript (the CLI's or the site's JSON) to another format
//!
//! The CLI reads no natural language: an agent (see `verbatim skill`) turns
//! what someone asks, in any language, into these options.

mod asr;
mod media;
mod mel;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::path::{Path, PathBuf};
use verbatim_edl::{
    CutList, Edit, ExportDoc, Format, Piece, Range, Span, Transcript, format_clock, parse_range,
    parse_seconds, resolve,
};

const SKILL: &str = include_str!("../../../skills/verbatim/SKILL.md");

#[derive(Parser)]
#[command(name = "verbatim", version, about = "Transcribe and edit video and audio on your machine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// No progress or notes on stderr.
    #[arg(long, short, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Command {
    /// What a media file holds: duration, streams, codecs, size (JSON).
    Info { media: PathBuf },
    /// Transcribe a video or audio file (Parakeet, or Canary with --lang en|fr|de|es).
    Transcribe {
        media: PathBuf,
        /// txt, txt-timed, srt, vtt, md, json, csv; comma-separated or repeated. Default: from -o's extension, else txt.
        #[arg(long, short)]
        format: Vec<String>,
        /// A file, or a directory for several formats. Default: stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Transcribe again even when a cached transcript exists.
        #[arg(long)]
        no_cache: bool,
        #[command(flatten)]
        speech: SpeechArgs,
    },
    /// The transcript as numbered, timed segments: the numbers --segments takes.
    Segments {
        media: PathBuf,
        /// Only segments containing this text (case aside). Repeatable: any of them.
        #[arg(long, short)]
        grep: Vec<String>,
        /// Only segments starting at or after this time.
        #[arg(long)]
        from: Option<String>,
        /// Only segments starting before this time.
        #[arg(long)]
        to: Option<String>,
        /// Words with their times instead of segments.
        #[arg(long)]
        words: bool,
        /// JSON lines: {"n", "start", "end", "text"}.
        #[arg(long)]
        json: bool,
        /// Use this transcript instead of the cached one or a new one.
        #[arg(long)]
        transcript: Option<PathBuf>,
        #[command(flatten)]
        speech: SpeechArgs,
    },
    /// Turn a transcript JSON (from `verbatim transcribe -f json` or the site) into another format.
    Export {
        transcript: PathBuf,
        #[arg(long, short, default_value = "srt")]
        format: String,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Work out the cuts and print them as a cut list (JSON), without writing any media.
    Plan {
        media: PathBuf,
        #[command(flatten)]
        edit: EditArgs,
        /// Write the cut list here instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Write the media without the cut parts. The source is never modified.
    #[command(visible_alias = "edit")]
    Cut {
        media: PathBuf,
        #[command(flatten)]
        edit: EditArgs,
        /// The edited media; its extension picks the container (.mp4, .mov, .mkv, .m4a, .mp3, .wav…).
        #[arg(long, short)]
        output: PathBuf,
        /// Copy the streams instead of re-encoding: much faster, each cut snaps to the keyframe before it.
        #[arg(long)]
        fast: bool,
        #[command(flatten)]
        encode: EncodeArgs,
        /// Also write the result's transcript as subtitles next to it (.srt).
        #[arg(long)]
        subtitles: bool,
        /// Show the cuts and stop.
        #[arg(long)]
        dry_run: bool,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// The speech models: status, download, path, remove.
    Model {
        #[arg(value_parser = ["status", "download", "path", "remove"], default_value = "status")]
        action: String,
        /// Which model download, path and remove act on.
        #[arg(long, value_enum, default_value = "parakeet")]
        engine: ModelChoice,
    },
    /// Check ffmpeg and the speech models.
    Doctor,
    /// Print the agent skill (SKILL.md) that matches this version.
    Skill,
}

/// What to cut. Every option is repeatable or combinable; the cuts add up.
#[derive(Args, Clone, Default)]
struct EditArgs {
    /// A range to delete: START-END in times (15:39-15:43, 939-943, 1:02:03.5-1:05:00, start-0:30, 12:00-end).
    #[arg(long, short, value_name = "RANGE")]
    delete: Vec<String>,
    /// A range to keep; everything outside the kept ranges is deleted.
    #[arg(long, short, value_name = "RANGE")]
    keep: Vec<String>,
    /// Delete the first DURATION (30, 0:30, 90s, 5min).
    #[arg(long, value_name = "DURATION")]
    delete_first: Option<String>,
    /// Delete the last DURATION (5:42 = the last five minutes forty-two).
    #[arg(long, value_name = "DURATION")]
    delete_last: Option<String>,
    /// Keep only the first DURATION.
    #[arg(long, value_name = "DURATION")]
    keep_first: Option<String>,
    /// Keep only the last DURATION.
    #[arg(long, value_name = "DURATION")]
    keep_last: Option<String>,
    /// Delete transcript segments by number, as `verbatim segments` shows them: 12,15-18.
    #[arg(long, value_name = "LIST")]
    segments: Vec<String>,
    /// Keep only these transcript segments.
    #[arg(long, value_name = "LIST")]
    keep_segments: Vec<String>,
    /// Delete every place these words are said (whole words, case and punctuation aside).
    #[arg(long, value_name = "TEXT")]
    phrase: Vec<String>,
    /// Delete hesitations: the built-in list (euh, um, uh, äh, ehm…) or --filler-words.
    #[arg(long)]
    fillers: bool,
    /// The hesitations of the spoken language, comma-separated (replaces the built-in list; implies --fillers).
    #[arg(long, value_name = "WORDS")]
    filler_words: Option<String>,
    /// Delete every pause of at least SECONDS.
    #[arg(long, value_name = "SECONDS")]
    silences: Option<f64>,
    /// Seconds of each pause kept on both sides.
    #[arg(long, default_value_t = 0.15, value_name = "SECONDS")]
    pad: f64,
    /// A cut list from `verbatim plan` or the site. Repeatable.
    #[arg(long, value_name = "FILE")]
    cuts: Vec<PathBuf>,
    /// Grow every cut by SECONDS on both sides (negative shrinks it).
    #[arg(
        long,
        default_value_t = 0.0,
        allow_hyphen_values = true,
        value_name = "SECONDS"
    )]
    expand: f64,
    /// Join cuts separated by less than SECONDS.
    #[arg(long, default_value_t = 0.0, value_name = "SECONDS")]
    merge_gap: f64,
    /// Drop cuts shorter than SECONDS.
    #[arg(long, default_value_t = 0.0, value_name = "SECONDS")]
    min_cut: f64,
    /// Use this transcript instead of the cached one or a new one.
    #[arg(long, value_name = "FILE")]
    transcript: Option<PathBuf>,
    #[command(flatten)]
    speech: SpeechArgs,
}

/// How a missing transcript is made.
#[derive(Args, Clone, Default)]
struct SpeechArgs {
    /// The spoken language (en, fr, de, es…). Parakeet guesses it, and can take accented speech for
    /// English; a language Canary knows (en, fr, de, es) is transcribed by Canary, told the language.
    #[arg(long, short = 'l', env = "VERBATIM_LANG")]
    lang: Option<String>,
    /// auto: Parakeet, or Canary when --lang names one of its languages.
    #[arg(long, value_enum, default_value = "auto")]
    engine: asr::EngineChoice,
}

/// How an exact cut is encoded (ignored with --fast).
#[derive(Args, Clone)]
struct EncodeArgs {
    #[arg(long, value_enum, default_value = "h264")]
    codec: media::VideoCodec,
    /// Constant quality, 0–51: lower is better and bigger.
    #[arg(long, default_value_t = 18, value_parser = clap::value_parser!(u8).range(0..=51))]
    crf: u8,
    /// Encoder speed: ultrafast, superfast, veryfast, faster, fast, medium, slow, slower, veryslow.
    #[arg(long, default_value = "medium")]
    preset: String,
    #[arg(long, default_value = "192k")]
    audio_bitrate: String,
    /// Leave the video out of the result.
    #[arg(long)]
    no_video: bool,
    /// Leave the audio out of the result.
    #[arg(long)]
    no_audio: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ModelChoice {
    Parakeet,
    Canary,
}

fn note(quiet: bool, msg: impl AsRef<str>) {
    if !quiet {
        eprintln!("{}", msg.as_ref());
    }
}

fn read_transcript(path: &Path) -> Result<ExportDoc> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    // The site's and `-f json`'s documents, or a bare transcript.
    if let Ok(doc) = serde_json::from_str::<ExportDoc>(&text) {
        return Ok(doc);
    }
    let transcript: Transcript = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a transcript", path.display()))?;
    Ok(ExportDoc {
        name: path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned()),
        engine: String::new(),
        transcript,
        summary: None,
        chapters: vec![],
    })
}

/// The transcript of `media`: given, cached, or made now (then cached).
fn transcript_for(
    media: &Path,
    given: Option<&Path>,
    no_cache: bool,
    speech: &SpeechArgs,
    quiet: bool,
) -> Result<Transcript> {
    if let Some(p) = given {
        return Ok(read_transcript(p)?.transcript);
    }
    let engine = asr::Engine::resolve(speech.engine, speech.lang.as_deref())?;
    let cache = asr::cache_path(media, &engine)?;
    if !no_cache && let Ok(text) = std::fs::read_to_string(&cache) {
        return Ok(serde_json::from_str(&text)?);
    }
    note(
        quiet,
        format!(
            "Transcribing {} with {}…",
            media.display(),
            engine.spec().label
        ),
    );
    let mut t = asr::transcribe(media::decode(media)?, &engine, quiet)?;
    // The container's length, not the audio's: a video may run past its sound.
    if let Ok(p) = media::probe(media) {
        t.duration = Some(t.duration.unwrap_or(0.0).max(p.duration));
    }
    std::fs::create_dir_all(cache.parent().expect("cache has a parent"))?;
    std::fs::write(&cache, serde_json::to_string(&t)?)?;
    Ok(t)
}

fn doc_for(media: &Path, transcript: Transcript) -> ExportDoc {
    ExportDoc {
        name: media
            .file_name()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned()),
        // Canary reports the language it was told; Parakeet reports none.
        engine: if transcript.language.is_some() {
            asr::CANARY.label
        } else {
            asr::PARAKEET.label
        }
        .into(),
        transcript,
        summary: None,
        chapters: vec![],
    }
}

fn formats(given: &[String], output: Option<&Path>) -> Result<Vec<Format>> {
    let mut out = Vec::new();
    for f in given.iter().flat_map(|f| f.split(',')) {
        out.push(Format::parse(f.trim()).with_context(|| {
            format!("unknown format {f}: txt, txt-timed, srt, vtt, md, json, csv")
        })?);
    }
    if out.is_empty() {
        out.push(
            output
                .and_then(|o| o.extension())
                .and_then(|e| Format::parse(&e.to_string_lossy()))
                .unwrap_or(Format::Txt),
        );
    }
    Ok(out)
}

fn write_exports(
    doc: &ExportDoc,
    formats: &[Format],
    output: Option<&Path>,
    stem: &str,
    quiet: bool,
) -> Result<()> {
    match output {
        None => {
            for f in formats {
                print!("{}", verbatim_edl::render(doc, *f));
            }
        }
        Some(o) if formats.len() == 1 && !o.is_dir() => {
            std::fs::write(o, verbatim_edl::render(doc, formats[0]))?;
            note(quiet, format!("Wrote {}", o.display()));
        }
        Some(dir) => {
            std::fs::create_dir_all(dir)?;
            for f in formats {
                let name = if *f == Format::TxtTimed {
                    format!("{stem}.timed.txt")
                } else {
                    format!("{stem}.{}", f.ext())
                };
                let path = dir.join(name);
                std::fs::write(&path, verbatim_edl::render(doc, *f))?;
                note(quiet, format!("Wrote {}", path.display()));
            }
        }
    }
    Ok(())
}

/// `12,15-18` → the 1-based numbers it lists.
fn segment_numbers(lists: &[String]) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for item in lists
        .iter()
        .flat_map(|l| l.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let bad = || format!("{item:?} is not a segment number or range: write 12, 15-18");
        match item.split_once('-') {
            Some((a, b)) => {
                let (a, b): (usize, usize) = (
                    a.trim().parse().with_context(bad)?,
                    b.trim().parse().with_context(bad)?,
                );
                if a == 0 || b < a {
                    bail!(bad());
                }
                out.extend(a..=b);
            }
            None => {
                let n: usize = item.parse().with_context(bad)?;
                if n == 0 {
                    bail!(bad());
                }
                out.push(n);
            }
        }
    }
    Ok(out)
}

fn segment_ranges(t: &Transcript, numbers: &[usize]) -> Result<Vec<Range>> {
    numbers
        .iter()
        .map(|&n| {
            t.segments
                .get(n - 1)
                .map(|s| Range::new(s.start, s.end))
                .with_context(|| {
                    format!(
                        "there is no segment {n}: the transcript has {}",
                        t.segments.len()
                    )
                })
        })
        .collect()
}

struct Planned {
    cuts: CutList,
    transcript: Option<Transcript>,
}

fn plan(media: &Path, a: &EditArgs, duration: f64, quiet: bool) -> Result<Planned> {
    let mut edits: Vec<Edit> = Vec::new();
    for r in &a.delete {
        edits.push(Edit::delete(parse_range(r).map_err(anyhow::Error::msg)?));
    }
    for r in &a.keep {
        edits.push(Edit::keep(parse_range(r).map_err(anyhow::Error::msg)?));
    }
    let secs = |s: &Option<String>| {
        s.as_deref()
            .map(parse_seconds)
            .transpose()
            .map_err(anyhow::Error::msg)
    };
    if let Some(s) = secs(&a.delete_first)? {
        edits.push(Edit::delete(Span::First { seconds: s }));
    }
    if let Some(s) = secs(&a.delete_last)? {
        edits.push(Edit::delete(Span::Last { seconds: s }));
    }
    if let Some(s) = secs(&a.keep_first)? {
        edits.push(Edit::keep(Span::First { seconds: s }));
    }
    if let Some(s) = secs(&a.keep_last)? {
        edits.push(Edit::keep(Span::Last { seconds: s }));
    }
    let delete_segments = segment_numbers(&a.segments)?;
    let keep_segments = segment_numbers(&a.keep_segments)?;
    let mut ranges: Vec<Range> = Vec::new();
    for p in &a.cuts {
        let text =
            std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
        let list: CutList = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a cut list", p.display()))?;
        ranges.extend(list.delete);
    }
    let filler_words: Option<Vec<String>> = a.filler_words.as_deref().map(|w| {
        w.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });
    let needs_transcript = a.fillers
        || filler_words.is_some()
        || a.silences.is_some()
        || !a.phrase.is_empty()
        || !delete_segments.is_empty()
        || !keep_segments.is_empty();
    let mut transcript = None;
    if needs_transcript {
        let t = transcript_for(media, a.transcript.as_deref(), false, &a.speech, quiet)?;
        if let Some(list) = &filler_words {
            ranges.extend(verbatim_edl::fillers_in(&t.words, list));
        } else if a.fillers {
            ranges.extend(verbatim_edl::fillers(&t.words));
        }
        if let Some(gap) = a.silences {
            ranges.extend(verbatim_edl::silences(&t.words, duration, gap, a.pad));
        }
        for p in &a.phrase {
            let found = verbatim_edl::find_text(&t.words, p);
            if found.is_empty() {
                note(quiet, format!("\"{p}\" is not in the transcript"));
            }
            ranges.extend(found);
        }
        ranges.extend(segment_ranges(&t, &delete_segments)?);
        if !keep_segments.is_empty() {
            let kept = verbatim_edl::merge(segment_ranges(&t, &keep_segments)?);
            ranges.extend(verbatim_edl::complement(&kept, duration));
        }
        transcript = Some(t);
    }
    ranges.extend(resolve(&edits, duration));
    // Shape the cuts: grow or shrink them, join close ones, drop tiny ones.
    if a.expand != 0.0 {
        ranges = ranges
            .into_iter()
            .map(|r| Range::new(r.start - a.expand, r.end + a.expand))
            .filter(|r| r.end > r.start)
            .collect();
    }
    let mut cuts = CutList::new(duration, ranges);
    if a.merge_gap > 0.0 {
        let grown: Vec<Range> = cuts
            .delete
            .iter()
            .map(|r| Range::new(r.start, r.end + a.merge_gap))
            .collect();
        let joined = verbatim_edl::merge(grown)
            .into_iter()
            .map(|r| Range::new(r.start, (r.end - a.merge_gap).max(r.start)))
            .collect();
        cuts = CutList::new(duration, joined);
    }
    if a.min_cut > 0.0 {
        cuts = CutList::new(
            duration,
            cuts.delete
                .into_iter()
                .filter(|r| r.len() >= a.min_cut)
                .collect(),
        );
    }
    cuts.source = media.file_name().map(|n| n.to_string_lossy().into_owned());
    Ok(Planned { cuts, transcript })
}

fn describe(cuts: &CutList) -> String {
    if cuts.delete.is_empty() {
        return "Nothing to cut.".into();
    }
    let mut s = String::new();
    let shown = 40;
    for r in cuts.delete.iter().take(shown) {
        s.push_str(&format!(
            "  cut {} → {}  ({:.1} s)\n",
            format_clock(r.start, true),
            format_clock(r.end, true),
            r.len()
        ));
    }
    if cuts.delete.len() > shown {
        s.push_str(&format!("  … and {} more\n", cuts.delete.len() - shown));
    }
    s.push_str(&format!(
        "{} cut(s), {} removed: {} → {}",
        cuts.delete.len(),
        format_clock(verbatim_edl::total(&cuts.delete), true),
        format_clock(cuts.duration, false),
        format_clock(cuts.output_duration(), false)
    ));
    s
}

fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "._/-:".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn print_pieces(pieces: &[(usize, &Piece)], json: bool) -> Result<()> {
    for (n, p) in pieces {
        if json {
            println!(
                "{}",
                serde_json::to_string(
                    &json!({ "n": n, "start": p.start, "end": p.end, "text": p.text })
                )?
            );
        } else {
            println!(
                "{n:>4}  {} – {}  {}",
                format_clock(p.start, true),
                format_clock(p.end, true),
                p.text.trim()
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("verbatim: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let quiet = cli.quiet;
    match cli.command {
        Command::Info { media } => {
            let p = media::probe(&media)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({ "file": media.display().to_string(), "duration": p.duration, "clock": format_clock(p.duration, true), "video": p.has_video, "audio": p.has_audio, "ffprobe": p.raw })
                )?
            );
            Ok(())
        }
        Command::Transcribe {
            media,
            format,
            output,
            no_cache,
            speech,
        } => {
            let formats = formats(&format, output.as_deref())?;
            let t = transcript_for(&media, None, no_cache, &speech, quiet)?;
            let stem = media
                .file_stem()
                .map_or_else(|| "transcript".into(), |s| s.to_string_lossy().into_owned());
            write_exports(
                &doc_for(&media, t),
                &formats,
                output.as_deref(),
                &stem,
                quiet,
            )
        }
        Command::Segments {
            media,
            grep,
            from,
            to,
            words,
            json,
            transcript,
            speech,
        } => {
            let t = transcript_for(&media, transcript.as_deref(), false, &speech, quiet)?;
            let bound = |s: &Option<String>| {
                s.as_deref()
                    .map(parse_seconds)
                    .transpose()
                    .map_err(anyhow::Error::msg)
            };
            let (from, to) = (
                bound(&from)?.unwrap_or(0.0),
                bound(&to)?.unwrap_or(f64::INFINITY),
            );
            let pieces = if words { &t.words } else { &t.segments };
            let shown: Vec<(usize, &Piece)> = pieces
                .iter()
                .enumerate()
                .map(|(i, p)| (i + 1, p))
                .filter(|(_, p)| p.start >= from && p.start < to)
                .filter(|(_, p)| {
                    grep.is_empty()
                        || grep
                            .iter()
                            .any(|g| p.text.to_lowercase().contains(&g.to_lowercase()))
                })
                .collect();
            print_pieces(&shown, json)
        }
        Command::Export {
            transcript,
            format,
            output,
        } => {
            let doc = read_transcript(&transcript)?;
            let formats = formats(&[format], output.as_deref())?;
            let stem = transcript
                .file_stem()
                .map_or_else(|| "transcript".into(), |s| s.to_string_lossy().into_owned());
            write_exports(&doc, &formats, output.as_deref(), &stem, quiet)
        }
        Command::Plan {
            media,
            edit,
            output,
        } => {
            let probe = media::probe(&media)?;
            let p = plan(&media, &edit, probe.duration, quiet)?;
            note(quiet, describe(&p.cuts));
            let body = format!("{}\n", serde_json::to_string_pretty(&p.cuts)?);
            match output {
                Some(o) => {
                    std::fs::write(&o, body).with_context(|| format!("writing {}", o.display()))
                }
                None => {
                    print!("{body}");
                    Ok(())
                }
            }
        }
        Command::Cut {
            media,
            edit,
            output,
            fast,
            encode,
            subtitles,
            dry_run,
            json,
        } => {
            let probe = media::probe(&media)?;
            let p = plan(&media, &edit, probe.duration, quiet)?;
            if !json || dry_run {
                note(quiet && !dry_run, describe(&p.cuts));
            }
            let command = format!(
                "verbatim cut {} {} -o {}",
                shell_quote(&media.display().to_string()),
                p.cuts
                    .cli_args()
                    .iter()
                    .map(|a| shell_quote(a))
                    .collect::<Vec<_>>()
                    .join(" "),
                shell_quote(&output.display().to_string())
            );
            let result = |written: bool| json!({ "cuts": p.cuts, "keeps": p.cuts.keeps(), "outputDuration": p.cuts.output_duration(), "output": written.then(|| output.display().to_string()), "command": command });
            if dry_run {
                if json {
                    println!("{}", serde_json::to_string_pretty(&result(false))?);
                }
                return Ok(());
            }
            if p.cuts.delete.is_empty() {
                bail!("nothing to cut");
            }
            if std::fs::canonicalize(&media).ok() == std::fs::canonicalize(&output).ok() {
                bail!("the output would overwrite the source");
            }
            let enc = media::Encode {
                codec: encode.codec,
                crf: encode.crf,
                preset: encode.preset.clone(),
                audio_bitrate: encode.audio_bitrate.clone(),
                no_video: encode.no_video,
                no_audio: encode.no_audio,
            };
            media::render(&media, &output, &p.cuts.keeps(), &probe, fast, &enc, quiet)?;
            if subtitles {
                let t = match p.transcript {
                    Some(ref t) => t.clone(),
                    None => transcript_for(
                        &media,
                        edit.transcript.as_deref(),
                        false,
                        &edit.speech,
                        quiet,
                    )?,
                };
                let doc = doc_for(&output, verbatim_edl::remap(&t, &p.cuts.delete));
                let srt = output.with_extension("srt");
                std::fs::write(&srt, verbatim_edl::render(&doc, Format::Srt))?;
                note(quiet, format!("Wrote {}", srt.display()));
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&result(true))?);
            } else {
                note(quiet, format!("Wrote {}", output.display()));
            }
            Ok(())
        }
        Command::Model { action, engine } => {
            let spec = match engine {
                ModelChoice::Parakeet => &asr::PARAKEET,
                ModelChoice::Canary => &asr::CANARY,
            };
            match action.as_str() {
                "download" => println!("{}", spec.ensure(quiet)?.display()),
                "path" => println!("{}", spec.dir().display()),
                "remove" => {
                    spec.remove()?;
                    note(quiet, "Removed.");
                }
                _ => {
                    for m in [&asr::PARAKEET, &asr::CANARY] {
                        println!(
                            "{:<22} {:<15} {:>4} MB  {}",
                            m.label,
                            if m.present() {
                                "ready"
                            } else {
                                "not downloaded"
                            },
                            m.size() / 1_000_000,
                            m.dir().display()
                        );
                    }
                }
            }
            Ok(())
        }
        Command::Doctor => {
            let ok = |b: bool| if b { "ok" } else { "missing" };
            println!("ffmpeg   {}", ok(media::which("ffmpeg").is_some()));
            println!("ffprobe  {}", ok(media::which("ffprobe").is_some()));
            for (name, m, hint) in [
                ("parakeet", &asr::PARAKEET, "verbatim model download"),
                (
                    "canary",
                    &asr::CANARY,
                    "verbatim model download --engine canary (for --lang en/fr/de/es)",
                ),
            ] {
                println!(
                    "{name:<8} {} ({})",
                    if m.present() {
                        "ready".to_string()
                    } else {
                        format!("not downloaded: `{hint}`")
                    },
                    m.dir().display()
                );
            }
            Ok(())
        }
        Command::Skill => {
            print!("{SKILL}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_lists() {
        assert_eq!(
            segment_numbers(&["12,15-17".into(), " 3 ".into()]).unwrap(),
            vec![12, 15, 16, 17, 3]
        );
        assert!(segment_numbers(&["0".into()]).is_err());
        assert!(segment_numbers(&["5-2".into()]).is_err());
        assert!(segment_numbers(&["douze".into()]).is_err());
    }

    #[test]
    fn quoting() {
        assert_eq!(shell_quote("15:39-15:43"), "15:39-15:43");
        assert_eq!(shell_quote("my talk.mp4"), "'my talk.mp4'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
