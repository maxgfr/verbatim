//! The question a model is asked when an instruction needs one ("cut the
//! part about pricing"), and how its answer is read. Shared so the site and
//! the CLI ask the same thing and accept the same answers.

use crate::Edit;
use crate::time::format_clock;
use crate::transcript::Transcript;
use serde::{Deserialize, Serialize};

/// What the model may answer: edits on the timeline, plus cuts the CLI
/// computes itself from the words (fillers, pauses, phrases).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    #[serde(default)]
    pub edits: Vec<Edit>,
    #[serde(default)]
    pub fillers: bool,
    #[serde(default)]
    pub silences: Option<f64>,
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

pub const PLAN_SYSTEM: &str = r#"You turn a video editing instruction into a cut plan. Answer with one JSON object and nothing else:
{"edits": [...], "fillers": false, "silences": null, "phrases": [], "note": "one short sentence on what you understood"}
Each edit is {"op": "delete" | "keep", "span": ..., ...} with times in seconds:
  {"op":"delete","span":"between","start":939,"end":943}
  {"op":"delete","span":"last","seconds":342}      the last 5:42
  {"op":"delete","span":"first","seconds":30}
  {"op":"delete","span":"from","start":720}        to the end
  {"op":"delete","span":"until","end":45}          from the beginning
"keep" edits mean everything outside them is deleted.
"fillers": true removes hesitations (euh, um, uh…). "silences": a number of seconds removes every pause at least that long.
"phrases": exact words or phrases from the transcript to cut wherever they are said.
Times like 15:39 are minutes:seconds. Use the transcript's times to find passages the instruction describes by content; cut whole sentences.
Never invent times the instruction or transcript does not support; if the instruction is unclear, return no edits and say why in "note"."#;

fn transcript_lines(t: &Transcript, limit: usize) -> String {
    let mut out = String::new();
    for s in &t.segments {
        let line = format!(
            "[{}–{}] {}\n",
            format_clock(s.start, true),
            format_clock(s.end, true),
            s.text.trim()
        );
        if out.len() + line.len() > limit {
            out.push_str("[… transcript truncated]\n");
            break;
        }
        out.push_str(&line);
    }
    out
}

/// The user message: the instruction, the duration and the transcript with its times.
pub fn plan_prompt(instruction: &str, duration: f64, transcript: Option<&Transcript>) -> String {
    let mut user = format!(
        "Instruction: {instruction}\nDuration: {} ({duration:.2} s)\n",
        format_clock(duration, true)
    );
    match transcript {
        Some(t) => user.push_str(&format!(
            "Transcript (segment times in m:ss):\n{}",
            transcript_lines(t, 400_000)
        )),
        None => user.push_str("No transcript is available: only time-based edits are possible.\n"),
    }
    user
}

/// The plan inside a model's answer (which may wrap the JSON in prose or a code fence).
pub fn parse_plan(text: &str) -> Result<Plan, String> {
    let json = text
        .find('{')
        .zip(text.rfind('}'))
        .filter(|(a, b)| a < b)
        .map(|(a, b)| &text[a..=b])
        .ok_or("the model's answer has no JSON object")?;
    serde_json::from_str(json).map_err(|e| format!("the model's plan is not valid ({e}): {json}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;

    #[test]
    fn reads_wrapped_answers() {
        let p = parse_plan("Sure:\n```json\n{\"edits\":[{\"op\":\"delete\",\"span\":\"last\",\"seconds\":342}],\"fillers\":true,\"note\":\"ok\"}\n```").unwrap();
        assert_eq!(p.edits, vec![Edit::delete(Span::Last { seconds: 342.0 })]);
        assert!(p.fillers);
        assert!(parse_plan("no json here").is_err());
        assert!(parse_plan("{\"edits\": [{\"op\": \"explode\"}]}").is_err());
    }

    #[test]
    fn prompt_carries_times() {
        let t = Transcript {
            text: String::new(),
            language: None,
            duration: Some(10.0),
            segments: vec![crate::Piece {
                text: "Bonjour".into(),
                start: 1.5,
                end: 2.0,
            }],
            words: vec![],
            word_timings: "exact".into(),
        };
        let p = plan_prompt("coupe le bonjour", 10.0, Some(&t));
        assert!(p.contains("[0:01.5–0:02] Bonjour"), "{p}");
        assert!(plan_prompt("x", 10.0, None).contains("No transcript"));
    }
}
