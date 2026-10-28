//! verbatim: transcribe and edit video and audio on this machine.
//!
//! transcribe  speech to text with Parakeet, every export format
//! plan        an edit as a cut list (JSON), without touching the media
//! cut / edit  write the media without the cut parts
//! export      a transcript (the CLI's or the site's JSON) to another format

mod ai;
mod asr;
mod media;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use verbatim_edl::{
    CutList, Edit, ExportDoc, Format, Range, Transcript, format_clock, parse_instruction, resolve,
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
    /// Transcribe a video or audio file with Parakeet (25 languages, timed to the word).
    Transcribe {
        media: PathBuf,
        /// txt, txt-timed, srt, vtt, md, json, csv; repeat for several. Default: from -o's extension, else txt.
        #[arg(long, short)]
        format: Vec<String>,
        /// A file, or a directory for several formats. Default: the first format on stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Transcribe again even when a cached transcript exists.
        #[arg(long)]
        no_cache: bool,
    },
    /// Turn a transcript JSON (from `verbatim transcribe -f json` or the site) into another format.
    Export {
        transcript: PathBuf,
        #[arg(long, short, default_value = "srt")]
        format: String,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Work out the cuts and print them as a cut list, without writing any media.
    Plan {
        media: PathBuf,
        #[command(flatten)]
        edit: EditArgs,
        /// Write the cut list here instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Write the media without the cut parts.
    #[command(visible_alias = "edit")]
    Cut {
        media: PathBuf,
        #[command(flatten)]
        edit: EditArgs,
        /// The edited media; its extension picks the container (.mp4, .mov, .mkv, .m4a, .mp3, .wav…).
        #[arg(long, short)]
        output: PathBuf,
        /// Copy the streams instead of re-encoding: much faster, cuts snap to keyframes.
        #[arg(long)]
        fast: bool,
        /// Also write the transcript of the result as subtitles next to it (.srt).
        #[arg(long)]
        subtitles: bool,
        /// Apply a plan that came from a model without asking.
        #[arg(long, short)]
        yes: bool,
        /// Show the plan and stop.
        #[arg(long)]
        dry_run: bool,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// The Parakeet model: status, download, path, remove.
    Model {
        #[arg(value_parser = ["status", "download", "path", "remove"], default_value = "status")]
        action: String,
    },
    /// Check ffmpeg, the model and the AI settings.
    Doctor,
    /// Print the agent skill (SKILL.md) that matches this version.
    Skill,
}

#[derive(Args, Clone, Default)]
struct EditArgs {
    /// The edit in plain French or English: "supprime de 15:39 à 15:43 et les 5:42 dernières minutes".
    instruction: Option<String>,
    /// A range to delete: 15:39-15:43, "last 5:42", "first 30", 12:00-end, start-0:30. Repeatable.
    #[arg(long, short)]
    delete: Vec<String>,
    /// A range to keep; everything outside the kept ranges is deleted. Repeatable.
    #[arg(long, short)]
    keep: Vec<String>,
    /// A cut list from `verbatim plan` or the site.
    #[arg(long)]
    cuts: Option<PathBuf>,
    /// Delete hesitations (euh, um, uh…). Needs a transcript (made if missing).
    #[arg(long)]
    fillers: bool,
    /// Delete every pause of at least this many seconds.
    #[arg(long, value_name = "SECONDS")]
    silences: Option<f64>,
    /// Seconds of each pause kept on both sides.
    #[arg(long, default_value_t = 0.15)]
    pad: f64,
    /// Delete every place these words are said. Repeatable.
    #[arg(long)]
    phrase: Vec<String>,
    /// Use this transcript instead of the cached one or a new one.
    #[arg(long)]
    transcript: Option<PathBuf>,
    /// The model for instructions the phrase reader cannot follow. Default: VERBATIM_AI, else the API key found.
    #[arg(long, value_enum)]
    ai: Option<ai::Provider>,
    /// The model id; default VERBATIM_AI_MODEL, or claude-sonnet-5-5 for Anthropic.
    #[arg(long)]
    ai_model: Option<String>,
    /// Never call a model: fail when the instruction is not understood.
    #[arg(long)]
    no_ai: bool,
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
    quiet: bool,
) -> Result<Transcript> {
    if let Some(p) = given {
        return Ok(read_transcript(p)?.transcript);
    }
    let cache = asr::cache_path(media)?;
    if !no_cache && let Ok(text) = std::fs::read_to_string(&cache) {
        return Ok(serde_json::from_str(&text)?);
    }
    note(
        quiet,
        format!("Transcribing {} with Parakeet…", media.display()),
    );
    let mut t = asr::transcribe(media::decode(media)?, quiet)?;
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
        engine: "Parakeet TDT 0.6B v3".into(),
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

fn spec_edits(specs: &[String], keep: bool) -> Result<Vec<Edit>> {
    let mut out = Vec::new();
    for s in specs {
        let p = parse_instruction(s);
        if !p.is_complete() {
            bail!(
                "cannot read the range {s:?}: write 15:39-15:43, \"last 5:42\", \"first 30\", 12:00-end or start-0:30"
            );
        }
        out.extend(
            p.edits
                .into_iter()
                .map(|e| if keep { Edit::keep(e.span) } else { e }),
        );
    }
    Ok(out)
}

struct Planned {
    cuts: CutList,
    /// "phrase", "ai" or "flags": where the edits came from.
    source: &'static str,
    note: Option<String>,
    transcript: Option<Transcript>,
}

fn plan(media: &Path, a: &EditArgs, duration: f64, quiet: bool) -> Result<Planned> {
    let mut edits = spec_edits(&a.delete, false)?;
    edits.extend(spec_edits(&a.keep, true)?);
    let mut ranges: Vec<Range> = Vec::new();
    if let Some(p) = &a.cuts {
        let text = std::fs::read_to_string(p)?;
        let list: CutList = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a cut list", p.display()))?;
        ranges.extend(list.delete);
    }
    let mut source = "flags";
    let mut note_text = None;
    let (mut fillers, mut silences, mut phrases) = (a.fillers, a.silences, a.phrase.clone());
    let mut transcript: Option<Transcript> = None;
    if let Some(instr) = a.instruction.as_deref().filter(|s| !s.trim().is_empty()) {
        let parsed = parse_instruction(instr);
        if parsed.is_complete() {
            source = "phrase";
            edits.extend(parsed.edits);
        } else {
            if a.no_ai {
                bail!(
                    "not understood without a model: {}. Use times (\"de 15:39 à 15:43\"), or drop --no-ai",
                    parsed.unparsed.join(", ")
                );
            }
            let model = ai::Ai::from_env(a.ai, a.ai_model.clone())?;
            let t = transcript_for(media, a.transcript.as_deref(), false, quiet)?;
            note(
                quiet,
                format!("Asking {} to read the instruction…", model.model),
            );
            let p = ai::plan(&model, instr, duration, Some(&t))?;
            transcript = Some(t);
            source = "ai";
            edits.extend(p.edits);
            fillers |= p.fillers;
            silences = silences.or(p.silences);
            phrases.extend(p.phrases);
            note_text = p.note;
        }
    }
    if fillers
        || silences.is_some()
        || !phrases.is_empty()
        || transcript.is_none() && a.transcript.is_some()
    {
        let t = match transcript.take() {
            Some(t) => t,
            None => transcript_for(media, a.transcript.as_deref(), false, quiet)?,
        };
        if fillers {
            ranges.extend(verbatim_edl::fillers(&t.words));
        }
        if let Some(gap) = silences {
            ranges.extend(verbatim_edl::silences(&t.words, duration, gap, a.pad));
        }
        for p in &phrases {
            let found = verbatim_edl::find_text(&t.words, p);
            if found.is_empty() {
                note(quiet, format!("\"{p}\" is not in the transcript"));
            }
            ranges.extend(found);
        }
        transcript = Some(t);
    }
    ranges.extend(resolve(&edits, duration));
    let mut cuts = CutList::new(duration, ranges);
    cuts.source = media.file_name().map(|n| n.to_string_lossy().into_owned());
    Ok(Planned {
        cuts,
        source,
        note: note_text,
        transcript,
    })
}

fn describe(p: &Planned) -> String {
    let mut s = String::new();
    if let Some(n) = &p.note {
        s.push_str(&format!("{n}\n"));
    }
    if p.cuts.delete.is_empty() {
        s.push_str("Nothing to cut.\n");
        return s;
    }
    let shown = 40;
    for r in p.cuts.delete.iter().take(shown) {
        s.push_str(&format!(
            "  cut {} → {}  ({:.1} s)\n",
            format_clock(r.start, true),
            format_clock(r.end, true),
            r.len()
        ));
    }
    if p.cuts.delete.len() > shown {
        s.push_str(&format!("  … and {} more\n", p.cuts.delete.len() - shown));
    }
    s.push_str(&format!(
        "{} cut(s), {} removed: {} → {}\n",
        p.cuts.delete.len(),
        format_clock(verbatim_edl::total(&p.cuts.delete), true),
        format_clock(p.cuts.duration, false),
        format_clock(p.cuts.output_duration(), false)
    ));
    s
}

fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("this plan came from a model: check it with --dry-run, then pass --yes to apply it");
    }
    eprint!("{question} [y/N] ");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(matches!(
        line.trim().to_lowercase().as_str(),
        "y" | "yes" | "o" | "oui"
    ))
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
        Command::Transcribe {
            media,
            format,
            output,
            no_cache,
        } => {
            let formats = formats(&format, output.as_deref())?;
            let t = transcript_for(&media, None, no_cache, quiet)?;
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
            note(quiet, describe(&p).trim_end());
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
            subtitles,
            yes,
            dry_run,
            json,
        } => {
            let probe = media::probe(&media)?;
            let p = plan(&media, &edit, probe.duration, quiet)?;
            let summary = describe(&p);
            if !json || dry_run {
                note(quiet && !dry_run, summary.trim_end());
            }
            let result = |written: bool| json!({ "source": p.source, "note": p.note, "cuts": p.cuts, "keeps": p.cuts.keeps(), "outputDuration": p.cuts.output_duration(), "output": written.then(|| output.display().to_string()), "command": format!("verbatim cut {} {} -o {}", media.display(), p.cuts.cli_args().join(" "), output.display()) });
            if dry_run {
                if json {
                    println!("{}", serde_json::to_string_pretty(&result(false))?);
                }
                return Ok(());
            }
            if p.cuts.delete.is_empty() {
                bail!("nothing to cut");
            }
            if p.source == "ai" && !yes && !confirm("Apply these cuts?")? {
                bail!("cancelled");
            }
            if std::fs::canonicalize(&media).ok() == std::fs::canonicalize(&output).ok() {
                bail!("the output would overwrite the source");
            }
            media::render(&media, &output, &p.cuts.keeps(), &probe, fast, quiet)?;
            if subtitles {
                let t = match p.transcript {
                    Some(ref t) => t.clone(),
                    None => transcript_for(&media, edit.transcript.as_deref(), false, quiet)?,
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
        Command::Model { action } => {
            match action.as_str() {
                "download" => {
                    asr::ensure_model(quiet)?;
                    println!("{}", asr::model_dir().display());
                }
                "path" => println!("{}", asr::model_dir().display()),
                "remove" => {
                    asr::remove_model()?;
                    note(quiet, "Removed.");
                }
                _ => println!(
                    "{} {} ({} MB) at {}",
                    asr::MODEL_NAME,
                    if asr::model_present() {
                        "ready"
                    } else {
                        "not downloaded"
                    },
                    asr::model_size() / 1_000_000,
                    asr::model_dir().display()
                ),
            }
            Ok(())
        }
        Command::Doctor => {
            let ok = |b: bool| if b { "ok" } else { "missing" };
            println!("ffmpeg   {}", ok(media::which("ffmpeg").is_some()));
            println!("ffprobe  {}", ok(media::which("ffprobe").is_some()));
            println!(
                "model    {} ({})",
                if asr::model_present() {
                    "ready"
                } else {
                    "not downloaded: `verbatim model download`"
                },
                asr::model_dir().display()
            );
            let ai = ai::Ai::from_env(None, None);
            println!(
                "ai       {}",
                ai.map_or_else(
                    |e| format!("none ({e})"),
                    |a| format!("{:?} {}", a.provider, a.model)
                )
            );
            Ok(())
        }
        Command::Skill => {
            print!("{SKILL}");
            Ok(())
        }
    }
}
