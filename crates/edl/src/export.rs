//! Every text export, shared by the site and the CLI. A translation is
//! exported by handing in a transcript whose segment texts are translated.

use crate::time::format_clock;
use crate::transcript::{Piece, Transcript};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Txt,
    TxtTimed,
    Srt,
    Vtt,
    Md,
    Json,
    Csv,
}

impl Format {
    pub const ALL: [Format; 7] = [
        Format::Txt,
        Format::TxtTimed,
        Format::Srt,
        Format::Vtt,
        Format::Md,
        Format::Json,
        Format::Csv,
    ];

    pub fn ext(self) -> &'static str {
        match self {
            Format::Txt | Format::TxtTimed => "txt",
            Format::Srt => "srt",
            Format::Vtt => "vtt",
            Format::Md => "md",
            Format::Json => "json",
            Format::Csv => "csv",
        }
    }

    pub fn parse(s: &str) -> Option<Format> {
        Some(match s.to_ascii_lowercase().as_str() {
            "txt" | "text" => Format::Txt,
            "txt-timed" | "timed" => Format::TxtTimed,
            "srt" => Format::Srt,
            "vtt" | "webvtt" => Format::Vtt,
            "md" | "markdown" => Format::Md,
            "json" => Format::Json,
            "csv" => Format::Csv,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub start: f64,
    pub title: String,
}

/// What an export knows about the file besides its transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDoc {
    pub name: String,
    #[serde(default)]
    pub engine: String,
    pub transcript: Transcript,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<Chapter>,
}

fn segments(doc: &ExportDoc) -> Vec<Piece> {
    doc.transcript
        .segments
        .iter()
        .map(|s| Piece {
            text: s.text.trim().to_string(),
            ..s.clone()
        })
        .filter(|s| !s.text.is_empty())
        .collect()
}

/// `00:15:39,250`, or with a dot for WebVTT.
fn stamp(seconds: f64, sep: char) -> String {
    let ms = (seconds.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}{sep}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// A cue lasts at least 0.2 s, or players skip it.
fn cue_end(s: &Piece) -> f64 {
    s.end.max(s.start + 0.2)
}

/// Paragraphs: segments joined until a pause of two seconds or more.
fn txt(doc: &ExportDoc) -> String {
    let mut paras: Vec<Vec<String>> = vec![];
    let mut last = f64::NEG_INFINITY;
    for s in segments(doc) {
        if paras.is_empty() || s.start - last >= 2.0 {
            paras.push(vec![]);
        }
        paras.last_mut().expect("pushed above").push(s.text);
        last = s.end;
    }
    format!(
        "{}\n",
        paras
            .iter()
            .map(|p| p.join(" "))
            .collect::<Vec<_>>()
            .join("\n\n")
    )
}

fn txt_timed(doc: &ExportDoc) -> String {
    segments(doc)
        .iter()
        .map(|s| format!("[{}] {}\n", format_clock(s.start, false), s.text))
        .collect()
}

fn srt(doc: &ExportDoc) -> String {
    segments(doc)
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                i + 1,
                stamp(s.start, ','),
                stamp(cue_end(s), ','),
                s.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn vtt(doc: &ExportDoc) -> String {
    // "-->" inside a cue's text would end it early.
    let cues: Vec<String> = segments(doc)
        .iter()
        .map(|s| {
            format!(
                "{} --> {}\n{}",
                stamp(s.start, '.'),
                stamp(cue_end(s), '.'),
                s.text.replace("-->", "->")
            )
        })
        .collect();
    format!("WEBVTT\n\n{}\n", cues.join("\n\n"))
}

fn csv_cell(v: &str) -> String {
    if v.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn csv(doc: &ExportDoc) -> String {
    let rows: String = segments(doc)
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{},{:.3},{:.3},{}\n",
                i + 1,
                s.start,
                s.end,
                csv_cell(&s.text)
            )
        })
        .collect();
    format!("index,start,end,text\n{rows}")
}

fn md(doc: &ExportDoc) -> String {
    let t = &doc.transcript;
    let meta: Vec<String> = [
        t.duration.map(|d| format_clock(d, false)),
        t.language.clone(),
        Some(doc.engine.clone()),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect();
    let mut parts = vec![format!("# {}", doc.name)];
    if !meta.is_empty() {
        parts.push(format!("_{}_", meta.join(" · ")));
    }
    if let Some(summary) = doc
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("## Summary\n\n{summary}"));
    }
    let segs = segments(doc);
    let line = |s: &Piece| format!("**{}** {}", format_clock(s.start, false), s.text);
    let mut chapters = doc.chapters.clone();
    chapters.sort_by(|a, b| a.start.total_cmp(&b.start));
    if chapters.is_empty() {
        parts.push(format!(
            "## Transcript\n\n{}",
            segs.iter().map(line).collect::<Vec<_>>().join("\n\n")
        ));
    } else {
        parts.push(format!(
            "## Chapters\n\n{}",
            chapters
                .iter()
                .map(|c| format!("- {} {}", format_clock(c.start, false), c.title))
                .collect::<Vec<_>>()
                .join("\n")
        ));
        parts.push("## Transcript".into());
        for (i, c) in chapters.iter().enumerate() {
            let end = chapters.get(i + 1).map_or(f64::INFINITY, |n| n.start);
            // The first chapter also takes whatever comes before it.
            let inside: Vec<String> = segs
                .iter()
                .filter(|s| s.start < end && (i == 0 || s.start >= c.start))
                .map(line)
                .collect();
            parts.push(format!(
                "### {} {}\n\n{}",
                format_clock(c.start, false),
                c.title,
                inside.join("\n\n")
            ));
        }
    }
    format!("{}\n", parts.join("\n\n"))
}

fn json(doc: &ExportDoc) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(doc).expect("plain data serializes")
    )
}

pub fn render(doc: &ExportDoc, format: Format) -> String {
    match format {
        Format::Txt => txt(doc),
        Format::TxtTimed => txt_timed(doc),
        Format::Srt => srt(doc),
        Format::Vtt => vtt(doc),
        Format::Md => md(doc),
        Format::Json => json(doc),
        Format::Csv => csv(doc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> ExportDoc {
        let p = |text: &str, start: f64, end: f64| Piece {
            text: text.into(),
            start,
            end,
        };
        ExportDoc {
            name: "interview.mp4".into(),
            engine: "Parakeet".into(),
            transcript: Transcript {
                text: String::new(),
                language: Some("fr".into()),
                duration: Some(3725.0),
                segments: vec![
                    p("Bonjour.", 0.0, 1.5),
                    p("  ", 1.5, 1.6),
                    p("Ça va, \"oui\" ?", 1.7, 1.7),
                    p("Suite --> ici", 3723.0, 3724.5),
                ],
                words: vec![],
                word_timings: "exact".into(),
            },
            summary: None,
            chapters: vec![],
        }
    }

    #[test]
    fn subtitles() {
        assert_eq!(
            render(&doc(), Format::Srt),
            "1\n00:00:00,000 --> 00:00:01,500\nBonjour.\n\n2\n00:00:01,700 --> 00:00:01,900\nÇa va, \"oui\" ?\n\n3\n01:02:03,000 --> 01:02:04,500\nSuite --> ici\n"
        );
        let vtt = render(&doc(), Format::Vtt);
        assert!(vtt.starts_with("WEBVTT\n\n00:00:00.000 --> 00:00:01.500\nBonjour.\n\n"));
        assert!(vtt.contains("Suite -> ici"));
    }

    #[test]
    fn text_and_tables() {
        assert_eq!(
            render(&doc(), Format::Txt),
            "Bonjour. Ça va, \"oui\" ?\n\nSuite --> ici\n"
        );
        assert_eq!(
            render(&doc(), Format::TxtTimed).lines().last(),
            Some("[1:02:03] Suite --> ici")
        );
        assert_eq!(
            render(&doc(), Format::Csv).lines().nth(2),
            Some("2,1.700,1.700,\"Ça va, \"\"oui\"\" ?\"")
        );
    }

    #[test]
    fn markdown_with_chapters() {
        let mut d = doc();
        d.summary = Some("Un résumé.".into());
        d.chapters = vec![
            Chapter {
                start: 3700.0,
                title: "Fin".into(),
            },
            Chapter {
                start: 1.0,
                title: "Accueil".into(),
            },
        ];
        let md = render(&d, Format::Md);
        assert!(md.starts_with("# interview.mp4\n\n_1:02:05 · fr · Parakeet_\n\n## Summary\n\nUn résumé.\n\n## Chapters\n\n- 0:01 Accueil\n- 1:01:40 Fin"), "{md}");
        assert!(md.contains("### 0:01 Accueil\n\n**0:00** Bonjour."));
        assert!(md.contains("### 1:01:40 Fin\n\n**1:02:03** Suite --> ici"));
    }

    #[test]
    fn formats_parse() {
        assert_eq!(Format::parse("SRT"), Some(Format::Srt));
        assert_eq!(Format::parse("markdown"), Some(Format::Md));
        assert_eq!(Format::parse("docx"), None);
        assert_eq!(
            serde_json::to_string(&Format::TxtTimed).unwrap(),
            "\"txt-timed\""
        );
    }
}
