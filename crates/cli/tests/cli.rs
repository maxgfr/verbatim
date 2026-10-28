//! The real binary on the e2e fixture (a 14 s French clip): plans, cuts and
//! errors, with ffmpeg and without the speech model.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_verbatim"))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/talk.mp4")
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

#[test]
fn version_and_skill() {
    let v = bin().arg("--version").output().unwrap();
    assert_eq!(
        String::from_utf8(v.stdout).unwrap().trim(),
        format!("verbatim {}", env!("CARGO_PKG_VERSION"))
    );
    let s = bin().arg("skill").output().unwrap();
    assert!(
        String::from_utf8(s.stdout)
            .unwrap()
            .starts_with("---\nname: verbatim\n")
    );
}

#[test]
fn plans_a_french_instruction_without_a_model() {
    if !has_ffmpeg() {
        return;
    }
    let out = bin()
        .args(["plan", "-q"])
        .arg(fixture())
        .arg("je veux supprimer de 0:04 à 0:06 et les 3 dernières secondes")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(list["version"], 1);
    assert_eq!(
        list["delete"],
        serde_json::json!([[4.0, 6.0], [11.0, 14.0]])
    );
}

#[test]
fn cuts_exactly_fast_and_to_audio() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let exact = dir.path().join("exact.mp4");
    let out = bin()
        .args([
            "cut",
            "-q",
            "--json",
            "--delete",
            "0:02-0:05",
            "--delete",
            "last 4",
        ])
        .arg(fixture())
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
    assert_eq!(result["source"], "flags");
    assert!((result["outputDuration"].as_f64().unwrap() - 7.0).abs() < 0.01);
    assert!(
        (duration(&exact) - 7.0).abs() < 0.15,
        "{}",
        duration(&exact)
    );

    let fast = dir.path().join("fast.mp4");
    assert!(
        bin()
            .args(["cut", "-q", "--fast", "--delete", "0:02-0:05"])
            .arg(fixture())
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
            .arg(fixture())
            .arg("-o")
            .arg(&audio)
            .status()
            .unwrap()
            .success()
    );
    assert!((duration(&audio) - 4.0).abs() < 0.15);
}

#[test]
fn refuses_what_it_cannot_do() {
    if !has_ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let o = dir.path().join("x.mp4");
    // Content-based instructions need a model; --no-ai refuses instead of guessing.
    let out = bin()
        .args(["cut", "--no-ai"])
        .arg(fixture())
        .arg("enlève le passage sur le prix")
        .arg("-o")
        .arg(&o)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not understood without a model: prix"));
    // An unreadable range.
    let out = bin()
        .args(["cut", "--delete", "soon"])
        .arg(fixture())
        .arg("-o")
        .arg(&o)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot read the range"));
    // Never over the source.
    let out = bin()
        .args(["cut", "--delete", "0:01-0:02"])
        .arg(fixture())
        .arg("-o")
        .arg(fixture())
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("overwrite the source"));
    // Cutting everything leaves nothing.
    let out = bin()
        .args(["cut", "--delete", "start-end"])
        .arg(fixture())
        .arg("-o")
        .arg(&o)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!o.exists());
}
