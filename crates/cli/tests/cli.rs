//! The real binary on the e2e fixture (a 14 s French clip, its transcript in
//! e2e/fixtures/talk.json): every cut option, the read commands and the
//! errors, with ffmpeg and without any speech model.

use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_verbatim"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../e2e/fixtures")
        .join(name)
}

fn has_ffmpeg() -> bool {
    Command::new("ffprobe")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn duration(path: &std::path::Path) -> f64 {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

/// `verbatim plan talk.mp4 <args> --transcript talk.json` → the ranges to delete.
fn plan(args: &[&str]) -> Vec<[f64; 2]> {
    let out = bin()
        .args(["plan", "-q"])
        .arg(fixture("talk.mp4"))
        .arg("--transcript")
        .arg(fixture("talk.json"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    serde_json::from_value(v["delete"].clone()).unwrap()
}

fn err(out: Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8(out.stderr).unwrap()
}

#[test]
fn version_skill_and_no_natural_language() {
    let v = bin().arg("--version").output().unwrap();
    assert_eq!(
        String::from_utf8(v.stdout).unwrap().trim(),
        format!("verbatim {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(
        String::from_utf8(bin().arg("skill").output().unwrap().stdout)
            .unwrap()
            .starts_with("---\nname: verbatim\n")
    );
    // Sentences are the agent's job: the CLI takes options only.
    let out = bin()
        .args([
            "cut",
            "x.mp4",
            "supprime les 3 dernières secondes",
            "-o",
            "y.mp4",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn time_options() {
    if !has_ffmpeg() {
        return;
    }
    assert_eq!(
        plan(&["--delete", "0:04-0:06", "--delete-last", "3"]),
        vec![[4.0, 6.0], [11.0, 14.0]]
    );
    assert_eq!(
        plan(&["--delete", "start-0:02", "--delete", "12-end"]),
        vec![[0.0, 2.0], [12.0, 14.0]]
    );
    assert_eq!(
        plan(&["--delete-first", "1s", "--keep-last", "0:10"]),
        vec![[0.0, 4.0]]
    );
    assert_eq!(
        plan(&["--keep", "0:02-0:05", "--keep", "10-12"]),
        vec![[0.0, 2.0], [5.0, 10.0], [12.0, 14.0]]
    );
    // Shaping: grow, join close cuts, drop small ones.
    assert_eq!(
        plan(&["--delete", "4-5", "--expand", "0.5"]),
        vec![[3.5, 5.5]]
    );
    assert_eq!(
        plan(&["--delete", "4-5", "--expand=-0.25"]),
        vec![[4.25, 4.75]]
    );
    assert_eq!(
        plan(&["--delete", "1-2", "--delete", "2.5-3", "--merge-gap", "1"]),
        vec![[1.0, 3.0]]
    );
    assert_eq!(
        plan(&["--delete", "1-1.2", "--delete", "5-7", "--min-cut", "0.5"]),
        vec![[5.0, 7.0]]
    );
}

#[test]
fn transcript_options() {
    if !has_ffmpeg() {
        return;
    }
    let segments: Vec<serde_json::Value> = String::from_utf8(
        bin()
            .args(["segments", "-q", "--json"])
            .arg(fixture("talk.mp4"))
            .arg("--transcript")
            .arg(fixture("talk.json"))
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .lines()
    .map(|l| serde_json::from_str(l).unwrap())
    .collect();
    assert_eq!(segments.len(), 6);
    assert_eq!(segments[1]["n"], 2);
    assert_eq!(segments[1]["text"], "Euh.");
    let ms = |v: &serde_json::Value| (v.as_f64().unwrap() * 1000.0).round() / 1000.0;
    let seg = |n: usize| [ms(&segments[n - 1]["start"]), ms(&segments[n - 1]["end"])];
    assert_eq!(
        plan(&["--segments", "2,6"]),
        vec![seg(2), [seg(6)[0], 14.0_f64.min(seg(6)[1])]]
    );
    assert_eq!(plan(&["--keep-segments", "1-3"])[0][0], seg(3)[1]);
    // Hesitations in any language: the built-in list, or the speaker's own.
    let fillers = plan(&["--fillers"]);
    assert_eq!(fillers.len(), 1);
    assert_eq!(plan(&["--filler-words", "bonjour,merci"]).len(), 2);
    assert_eq!(plan(&["--phrase", "le premier point"]).len(), 1);
    assert!(!plan(&["--silences", "0.3"]).is_empty());
    // Filtered listing.
    let grep = String::from_utf8(
        bin()
            .args(["segments", "-q", "--grep", "POINT"])
            .arg(fixture("talk.mp4"))
            .arg("--transcript")
            .arg(fixture("talk.json"))
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(grep.lines().count(), 2, "{grep}");
}

#[test]
fn info_and_rendering() {
    if !has_ffmpeg() {
        return;
    }
    let info: serde_json::Value = serde_json::from_slice(
        &bin()
            .arg("info")
            .arg(fixture("talk.mp4"))
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(info["video"], true);
    assert_eq!(info["audio"], true);
    assert!((info["duration"].as_f64().unwrap() - 14.0).abs() < 0.1);

    let dir = tempfile::tempdir().unwrap();
    let exact = dir.path().join("exact.mp4");
    let out = bin()
        .args([
            "cut",
            "-q",
            "--json",
            "--delete",
            "0:02-0:05",
            "--delete-last",
            "4",
            "--crf",
            "28",
            "--preset",
            "veryfast",
        ])
        .arg(fixture("talk.mp4"))
        .arg("-o")
        .arg(&exact)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!((result["outputDuration"].as_f64().unwrap() - 7.0).abs() < 0.01);
    assert_eq!(
        result["command"],
        format!(
            "verbatim cut {} --delete 0:02-0:05 --delete 0:10-0:14 -o {}",
            fixture("talk.mp4").display(),
            exact.display()
        )
    );
    assert!(
        (duration(&exact) - 7.0).abs() < 0.15,
        "{}",
        duration(&exact)
    );

    let h265 = dir.path().join("h265.mp4");
    assert!(
        bin()
            .args([
                "cut",
                "-q",
                "--codec",
                "h265",
                "--delete",
                "0:02-0:05",
                "--preset",
                "ultrafast"
            ])
            .arg(fixture("talk.mp4"))
            .arg("-o")
            .arg(&h265)
            .status()
            .unwrap()
            .success()
    );
    let codec = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "csv=p=0",
        ])
        .arg(&h265)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(codec.stdout).unwrap().trim(), "hevc");

    let silent = dir.path().join("silent.mp4");
    assert!(
        bin()
            .args([
                "cut",
                "-q",
                "--no-audio",
                "--delete",
                "0:02-0:05",
                "--preset",
                "ultrafast"
            ])
            .arg(fixture("talk.mp4"))
            .arg("-o")
            .arg(&silent)
            .status()
            .unwrap()
            .success()
    );
    let streams = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
        ])
        .arg(&silent)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(streams.stdout).unwrap().trim(), "video");

    let fast = dir.path().join("fast.mp4");
    assert!(
        bin()
            .args(["cut", "-q", "--fast", "--delete", "0:02-0:05"])
            .arg(fixture("talk.mp4"))
            .arg("-o")
            .arg(&fast)
            .status()
            .unwrap()
            .success()
    );
    assert!(duration(&fast) < 13.5);

    let audio = dir.path().join("a.m4a");
    assert!(
        bin()
            .args(["cut", "-q", "--keep", "0:00-0:04"])
            .arg(fixture("talk.mp4"))
            .arg("-o")
            .arg(&audio)
            .status()
            .unwrap()
            .success()
    );
    assert!((duration(&audio) - 4.0).abs() < 0.15);
}

#[test]
fn clear_errors() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let o = dir.path().join("x.mp4");
    let cut = |args: &[&str]| {
        bin()
            .arg("cut")
            .arg(fixture("talk.mp4"))
            .args(args)
            .arg("-o")
            .arg(&o)
            .output()
            .unwrap()
    };
    assert!(err(cut(&["--delete", "soon"])).contains("is not a range"));
    assert!(err(cut(&["--delete", "0:05-0:02"])).contains("ends before it starts"));
    assert!(err(cut(&["--delete-last", "five"])).contains("is not a duration"));
    assert!(
        err(cut(&[
            "--segments",
            "99",
            "--transcript",
            fixture("talk.json").to_str().unwrap()
        ]))
        .contains("there is no segment 99")
    );
    assert!(
        err(cut(&[
            "--segments",
            "2-a",
            "--transcript",
            fixture("talk.json").to_str().unwrap()
        ]))
        .contains("not a segment number")
    );
    assert!(
        err(bin()
            .arg("cut")
            .arg(fixture("talk.mp4"))
            .args(["--delete", "0:01-0:02", "-o"])
            .arg(fixture("talk.mp4"))
            .output()
            .unwrap())
        .contains("overwrite the source")
    );
    assert!(err(cut(&["--delete", "start-end"])).contains("nothing would be left"));
    assert!(!o.exists());
    // The engine is checked before any download.
    let canary = |extra: &[&str]| {
        bin()
            .args(["transcribe", "-q", "--engine", "canary"])
            .args(extra)
            .arg(fixture("talk.mp4"))
            .output()
            .unwrap()
    };
    assert!(err(canary(&[])).contains("Canary needs the spoken language"));
    assert!(err(canary(&["--lang", "it"])).contains("not \"it\""));
    let status =
        String::from_utf8(bin().args(["model", "status"]).output().unwrap().stdout).unwrap();
    assert!(
        status.contains("Parakeet TDT 0.6B v3") && status.contains("Canary 180M Flash"),
        "{status}"
    );
}
