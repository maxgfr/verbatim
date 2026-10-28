//! Cuts found in a transcript (fillers, silences, a phrase), and the
//! transcript moved onto the edited timeline.
//!
//! The shape is llm-connect's `Transcript`, the one the site exports as JSON
//! and the CLI writes: `segments` and `words` of `{ text, start, end }`.

use crate::ranges::{Range, merge};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(default)]
    pub segments: Vec<Piece>,
    #[serde(default)]
    pub words: Vec<Piece>,
    #[serde(default = "no_timings")]
    pub word_timings: String,
}

fn no_timings() -> String {
    "none".into()
}

/// Hesitations, in the languages the models hear most. Matched on the whole
/// word, case and punctuation aside.
const FILLERS: &[&str] = &[
    // French
    "euh", "euhh", "heu", "heuu", "hum", "hmm", "hm", "bah", "beh", "mmh", "mh", "eh",
    // English
    "um", "umm", "uh", "uhh", "uhm", "erm", "er", "ah", "mm", "mhm",
    // German, Spanish, Italian
    "äh", "ähm", "öh", "ehm", "eeh", "este",
];

fn bare(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// The filler words, each as its own range.
pub fn fillers(words: &[Piece]) -> Vec<Range> {
    merge(
        words
            .iter()
            .filter(|w| FILLERS.contains(&bare(&w.text).as_str()))
            .map(|w| Range::new(w.start, w.end))
            .collect(),
    )
}

/// Pauses of at least `min_gap` seconds between words (and before the first
/// and after the last one, given the duration), shortened by `pad` on each side
/// so speech is not clipped.
pub fn silences(words: &[Piece], duration: f64, min_gap: f64, pad: f64) -> Vec<Range> {
    let mut out = Vec::new();
    let mut at = 0.0_f64;
    let mut first = true;
    for w in words {
        let gap_start = if first { 0.0 } else { at + pad };
        let gap_end = w.start - pad;
        if w.start - at >= min_gap && gap_end > gap_start {
            out.push(Range::new(gap_start, gap_end));
        }
        at = at.max(w.end);
        first = false;
    }
    if !words.is_empty() && duration - at >= min_gap {
        out.push(Range::new(at + pad, duration));
    }
    merge(out)
}

/// Every place the words say `query` (a word or a phrase), from its first
/// word's start to its last word's end.
pub fn find_text(words: &[Piece], query: &str) -> Vec<Range> {
    let q: Vec<String> = query
        .split_whitespace()
        .map(bare)
        .filter(|w| !w.is_empty())
        .collect();
    if q.is_empty() {
        return vec![];
    }
    let norm: Vec<String> = words.iter().map(|w| bare(&w.text)).collect();
    let mut out = Vec::new();
    for i in 0..words.len().saturating_sub(q.len() - 1) {
        if norm[i..i + q.len()] == q[..] {
            out.push(Range::new(words[i].start, words[i + q.len() - 1].end));
        }
    }
    merge(out)
}

/// Words joined back into text, without a space before closing punctuation.
pub fn join_words<'a>(words: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for w in words.into_iter().map(str::trim).filter(|w| !w.is_empty()) {
        let glued = w.starts_with([',', '.', '!', '?', ';', ':', ')', '…', '%'])
            || out.ends_with(['(', '\'', '’']);
        if !out.is_empty() && !glued {
            out.push(' ');
        }
        out.push_str(w);
    }
    out
}

/// Words grouped into segments: at the end of a sentence, before a pause of
/// `pause` seconds, or past `max_words` words (then at the next comma when one comes soon).
pub fn segments_from_words(words: &[Piece], pause: f64, max_words: usize) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut cur: Vec<&Piece> = Vec::new();
    let flush = |cur: &mut Vec<&Piece>, out: &mut Vec<Piece>| {
        if let (Some(first), Some(last)) = (cur.first(), cur.last()) {
            out.push(Piece {
                text: join_words(cur.iter().map(|w| w.text.as_str())),
                start: first.start,
                end: last.end,
            });
        }
        cur.clear();
    };
    for (i, w) in words.iter().enumerate() {
        cur.push(w);
        let t = w.text.trim_end();
        let next_gap = words.get(i + 1).map_or(0.0, |n| n.start - w.end);
        let sentence = t.ends_with(['.', '!', '?', '…']);
        let long = cur.len() >= max_words
            && (t.ends_with([',', ';', ':']) || cur.len() >= max_words + max_words / 2);
        if sentence || next_gap >= pause || long {
            flush(&mut cur, &mut out);
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// A time on the original timeline moved onto the edited one: earlier cuts
/// pulled out, and a time inside a cut taken to where the cut was.
fn shift(t: f64, delete: &[Range]) -> f64 {
    let mut removed = 0.0;
    for r in delete {
        if t >= r.end {
            removed += r.len();
        } else if t > r.start {
            removed += t - r.start;
        } else {
            break;
        }
    }
    ((t - removed) * 1000.0).round() / 1000.0
}

fn remap_pieces(pieces: &[Piece], delete: &[Range]) -> Vec<Piece> {
    pieces
        .iter()
        .filter_map(|p| {
            let (start, end) = (shift(p.start, delete), shift(p.end, delete));
            // A piece that lies entirely inside a cut is gone.
            (end - start > 0.01).then(|| Piece {
                text: p.text.clone(),
                start,
                end,
            })
        })
        .collect()
}

/// The transcript of the edited media: pieces that were cut are dropped, the
/// others moved earlier by what was cut before them. `delete` must be merged.
pub fn remap(t: &Transcript, delete: &[Range]) -> Transcript {
    let words = remap_pieces(&t.words, delete);
    let segments = if t.words.is_empty() {
        remap_pieces(&t.segments, delete)
    } else {
        // A segment cut in part keeps only the words that are left.
        let inside = |w: &Piece| {
            delete
                .iter()
                .any(|r| w.start >= r.start - 0.005 && w.end <= r.end + 0.005)
        };
        t.segments
            .iter()
            .filter_map(|s| {
                let kept: Vec<&Piece> = t
                    .words
                    .iter()
                    .filter(|w| w.start >= s.start - 0.005 && w.end <= s.end + 0.005 && !inside(w))
                    .collect();
                let (first, last) = (kept.first()?, kept.last()?);
                let text = join_words(kept.iter().map(|w| w.text.as_str()));
                text.chars().any(char::is_alphanumeric).then(|| Piece {
                    text,
                    start: shift(first.start, delete),
                    end: shift(last.end, delete),
                })
            })
            .collect()
    };
    let text = segments
        .iter()
        .map(|s| s.text.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Transcript {
        text,
        language: t.language.clone(),
        duration: t.duration.map(|d| shift(d, delete)),
        segments,
        words,
        word_timings: t.word_timings.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(text: &str, start: f64, end: f64) -> Piece {
        Piece {
            text: text.into(),
            start,
            end,
        }
    }

    #[test]
    fn finds_fillers() {
        let words = [
            w("Bonjour,", 0.0, 0.5),
            w("euh", 0.6, 0.9),
            w("je", 1.0, 1.1),
            w("Um.", 1.2, 1.4),
            w("euro", 1.5, 1.9),
        ];
        assert_eq!(
            fillers(&words),
            vec![Range::new(0.6, 0.9), Range::new(1.2, 1.4)]
        );
    }

    #[test]
    fn finds_silences_with_padding() {
        let words = [w("a", 2.0, 2.5), w("b", 2.6, 3.0), w("c", 6.0, 6.5)];
        assert_eq!(
            silences(&words, 10.0, 1.0, 0.1),
            vec![
                Range::new(0.0, 1.9),
                Range::new(3.1, 5.9),
                Range::new(6.6, 10.0)
            ]
        );
    }

    #[test]
    fn finds_phrases() {
        let words = [
            w("Le", 0.0, 0.2),
            w("prix,", 0.2, 0.6),
            w("c'est", 0.7, 1.0),
            w("le", 1.0, 1.1),
            w("Prix", 1.1, 1.5),
        ];
        assert_eq!(
            find_text(&words, "le prix"),
            vec![Range::new(0.0, 0.6), Range::new(1.0, 1.5)]
        );
        assert!(find_text(&words, "").is_empty());
        assert!(find_text(&words, "absent du texte entier ici").is_empty());
    }

    #[test]
    fn remaps_after_cuts() {
        let t = Transcript {
            text: String::new(),
            language: Some("fr".into()),
            duration: Some(10.0),
            segments: vec![w("un", 0.0, 2.0), w("deux", 3.0, 4.0), w("trois", 6.0, 8.0)],
            words: vec![],
            word_timings: "exact".into(),
        };
        let out = remap(&t, &[Range::new(2.5, 4.5), Range::new(9.0, 10.0)]);
        assert_eq!(out.segments, vec![w("un", 0.0, 2.0), w("trois", 4.0, 6.0)]);
        assert_eq!(out.duration, Some(7.0));
        assert_eq!(out.text, "un trois");
    }

    #[test]
    fn remap_rebuilds_partly_cut_segments_from_words() {
        let words = vec![
            w("Bonjour", 0.0, 0.5),
            w("euh", 0.6, 0.9),
            w("toi.", 1.0, 1.4),
            w("Euh.", 2.0, 2.3),
        ];
        let t = Transcript {
            text: String::new(),
            language: None,
            duration: Some(3.0),
            segments: vec![w("Bonjour euh toi.", 0.0, 1.4), w("Euh.", 2.0, 2.3)],
            words: words.clone(),
            word_timings: "exact".into(),
        };
        let out = remap(&t, &fillers(&words));
        assert_eq!(out.segments, vec![w("Bonjour toi.", 0.0, 1.1)]);
        assert_eq!(out.words.len(), 2);
    }

    #[test]
    fn groups_words_into_segments() {
        let words = [
            w("Bonjour", 0.0, 0.4),
            w(",", 0.4, 0.4),
            w("ça", 0.5, 0.7),
            w("va.", 0.7, 1.0),
            w("Oui", 1.1, 1.3),
            w("l'", 3.0, 3.1),
            w("ami", 3.1, 3.4),
        ];
        let segs = segments_from_words(&words, 1.0, 30);
        assert_eq!(
            segs,
            vec![
                w("Bonjour, ça va.", 0.0, 1.0),
                w("Oui", 1.1, 1.3),
                w("l'ami", 3.0, 3.4)
            ]
        );
    }

    #[test]
    fn reads_llm_connect_json() {
        let json = r#"{"text":"a","segments":[{"text":"a","start":0,"end":1}],"words":[],"wordTimings":"exact"}"#;
        let t: Transcript = serde_json::from_str(json).unwrap();
        assert_eq!(t.word_timings, "exact");
        assert!(serde_json::to_string(&t).unwrap().contains("wordTimings"));
    }
}
