//! Everything that touches the media goes through ffmpeg and ffprobe: decoding
//! the audio for the model, reading the duration, and cutting.

use anyhow::{Context, Result, bail};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use verbatim_edl::Range;

pub const SAMPLE_RATE: u32 = 16_000;

fn tool(name: &str) -> Result<Command> {
    which(name).with_context(|| {
        format!("{name} is not installed: `brew install ffmpeg` (or your system's package)")
    })?;
    Ok(Command::new(name))
}

pub fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join(name))
            .find(|f| f.is_file())
    })
}

pub struct Probe {
    pub duration: f64,
    pub has_video: bool,
    pub has_audio: bool,
}

pub fn probe(path: &Path) -> Result<Probe> {
    let out = tool("ffprobe")?
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .context("running ffprobe")?;
    if !out.status.success() {
        bail!(
            "ffprobe cannot read {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let duration = v["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse().ok())
        .context("the media has no duration")?;
    let kinds: Vec<&str> = v["streams"]
        .as_array()
        .map(|s| s.iter().filter_map(|s| s["codec_type"].as_str()).collect())
        .unwrap_or_default();
    Ok(Probe {
        duration,
        has_video: kinds.contains(&"video"),
        has_audio: kinds.contains(&"audio"),
    })
}

/// The audio track as mono samples at 16 kHz.
pub fn decode(path: &Path) -> Result<Vec<f32>> {
    let mut child = tool("ffmpeg")?
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            &SAMPLE_RATE.to_string(),
            "-f",
            "f32le",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running ffmpeg")?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .expect("piped")
        .read_to_end(&mut bytes)?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "ffmpeg cannot decode {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect())
}

fn audio_only(out: &Path) -> bool {
    matches!(
        out.extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("mp3" | "m4a" | "aac" | "wav" | "flac" | "ogg" | "opus")
    )
}

/// Write `input` without the cut ranges to `out`.
///
/// Exact (default): every kept range is trimmed and the pieces concatenated,
/// re-encoding (H.264/AAC in MP4, or what the output's extension implies).
/// Fast: streams are copied, so each cut snaps to the keyframe before it.
pub fn render(
    input: &Path,
    out: &Path,
    keeps: &[Range],
    probe: &Probe,
    fast: bool,
    quiet: bool,
) -> Result<()> {
    if keeps.is_empty() {
        bail!("nothing would be left: every part of the media is cut");
    }
    if fast {
        return render_fast(input, out, keeps, quiet);
    }
    let video = probe.has_video && !audio_only(out);
    let audio = probe.has_audio;
    let mut graph = String::new();
    let mut inputs = String::new();
    for (i, k) in keeps.iter().enumerate() {
        if video {
            graph.push_str(&format!(
                "[0:v]trim=start={:.3}:end={:.3},setpts=PTS-STARTPTS[v{i}];",
                k.start, k.end
            ));
            inputs.push_str(&format!("[v{i}]"));
        }
        if audio {
            graph.push_str(&format!(
                "[0:a]atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS[a{i}];",
                k.start, k.end
            ));
            inputs.push_str(&format!("[a{i}]"));
        }
    }
    graph.push_str(&format!(
        "{inputs}concat=n={}:v={}:a={}",
        keeps.len(),
        u8::from(video),
        u8::from(audio)
    ));
    if video {
        graph.push_str("[v]");
    }
    if audio {
        graph.push_str("[a]");
    }
    let mut cmd = tool("ffmpeg")?;
    cmd.args([
        "-y",
        "-nostdin",
        "-v",
        if quiet { "error" } else { "warning" },
        if quiet { "-nostats" } else { "-stats" },
        "-i",
    ])
    .arg(input);
    // Long graphs (hundreds of filler cuts) go through a file (`-/option file`, ffmpeg 7+), not the command line.
    let script = std::env::temp_dir().join(format!("verbatim-{}.filter", std::process::id()));
    if graph.len() > 32_000 {
        std::fs::File::create(&script)?.write_all(graph.as_bytes())?;
        cmd.arg("-/filter_complex").arg(&script);
    } else {
        cmd.arg("-filter_complex").arg(&graph);
    }
    if video {
        cmd.args([
            "-map", "[v]", "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt",
            "yuv420p",
        ]);
    }
    if audio {
        cmd.args(["-map", "[a]"]);
        if !audio_only(out) {
            cmd.args(["-c:a", "aac", "-b:a", "192k"]);
        }
    }
    if !audio_only(out) {
        cmd.args(["-movflags", "+faststart"]);
    }
    let status = cmd.arg(out).status().context("running ffmpeg")?;
    let _ = std::fs::remove_file(&script);
    if !status.success() {
        bail!("ffmpeg failed to write {}", out.display());
    }
    Ok(())
}

fn render_fast(input: &Path, out: &Path, keeps: &[Range], quiet: bool) -> Result<()> {
    let list = std::env::temp_dir().join(format!("verbatim-{}.concat", std::process::id()));
    let abs = std::fs::canonicalize(input)?;
    let quoted = abs.to_string_lossy().replace('\'', "'\\''");
    let mut body = String::from("ffconcat version 1.0\n");
    for k in keeps {
        body.push_str(&format!(
            "file '{quoted}'\ninpoint {:.3}\noutpoint {:.3}\n",
            k.start, k.end
        ));
    }
    std::fs::write(&list, body)?;
    let status = tool("ffmpeg")?
        .args([
            "-y",
            "-nostdin",
            "-v",
            if quiet { "error" } else { "warning" },
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
        ])
        .arg(&list)
        .args(["-c", "copy", "-avoid_negative_ts", "make_zero"])
        .arg(out)
        .status()
        .context("running ffmpeg")?;
    let _ = std::fs::remove_file(&list);
    if !status.success() {
        bail!("ffmpeg failed to write {}", out.display());
    }
    Ok(())
}
