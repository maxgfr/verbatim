//! Editing instructions in plain French or English, read without a model:
//! "je veux supprimer de 15:39 à 15:43 et les 5:42 dernières minutes",
//! "keep 1:00-2:30", "enlève les 30 premières secondes", "cut from 12:00 to
//! the end". Whatever is not understood is returned in `unparsed`, so the
//! caller can ask a model (or the person) instead of guessing.

use crate::time::{colon_time, parse_time, unit};
use crate::{Edit, Op, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Parsed {
    pub edits: Vec<Edit>,
    /// Words and times that carried meaning but matched no pattern.
    pub unparsed: Vec<String>,
}

impl Parsed {
    /// Every part of the instruction became an edit.
    pub fn is_complete(&self) -> bool {
        !self.edits.is_empty() && self.unparsed.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Word(String),
    /// A time or duration in seconds; `explicit` when it had a colon or a unit.
    Time {
        seconds: f64,
        explicit: bool,
        text: String,
    },
    Dash,
}

fn fold(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' => 'i',
        'ô' | 'ö' | 'ó' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        'ÿ' => 'y',
        'œ' => 'o',
        c => c,
    }
}

fn raw_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let push = |cur: &mut String, out: &mut Vec<String>| {
        // Sentence punctuation stuck to a word ("15:43," "fin.").
        let w = cur.trim_end_matches(['.', ',', ':']).to_string();
        if !w.is_empty() {
            out.push(w);
        }
        cur.clear();
    };
    for c in text.chars().flat_map(char::to_lowercase).map(fold) {
        match c {
            'a'..='z' | '0'..='9' | ':' => cur.push(c),
            // Decimal separators stay inside numbers only.
            '.' | ',' if cur.chars().last().is_some_and(|p| p.is_ascii_digit()) => cur.push(c),
            '-' | '–' | '—' => {
                push(&mut cur, &mut out);
                out.push("-".into());
            }
            _ => push(&mut cur, &mut out),
        }
    }
    push(&mut cur, &mut out);
    out
}

fn tokenize(text: &str) -> Vec<Tok> {
    let raw = raw_tokens(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let w = &raw[i];
        if w == "-" {
            out.push(Tok::Dash);
            i += 1;
            continue;
        }
        let Some(mut seconds) = parse_time(w) else {
            out.push(Tok::Word(w.clone()));
            i += 1;
            continue;
        };
        let bare = !w.contains(':') && !w.chars().any(|c| c.is_ascii_alphabetic());
        let mut explicit = !bare;
        let mut text = w.clone();
        i += 1;
        // "5 min", "2 min 30 s", "1 h 02", "5:42 minutes".
        if bare {
            if let Some(u) = raw.get(i).and_then(|n| unit(n)) {
                seconds *= u;
                explicit = true;
                text = format!("{text} {}", raw[i]);
                i += 1;
                let mut last = u;
                while let Some(v) = raw
                    .get(i)
                    .filter(|n| !n.contains(':'))
                    .and_then(|n| n.parse::<f64>().ok())
                {
                    let next = raw.get(i + 1).and_then(|n| unit(n));
                    let u2 = match next {
                        Some(u2) if u2 < last => u2,
                        None if last > 1.0 => last / 60.0,
                        _ => break,
                    };
                    seconds += v * u2;
                    text = format!("{text} {}", raw[i]);
                    i += if next.is_some() { 2 } else { 1 };
                    last = u2;
                }
            }
        } else if colon_time(w).is_some() && raw.get(i).and_then(|n| unit(n)).is_some() {
            i += 1;
        }
        out.push(Tok::Time {
            seconds,
            explicit,
            text,
        });
    }
    out
}

const DELETE_STEMS: &[&str] = &[
    "supprim", "enlev", "coup", "retir", "effac", "vire", "degag", "delet", "remov", "cut", "trim",
    "drop", "rogn",
];
const KEEP_STEMS: &[&str] = &["gard", "conserv", "keep", "retain", "preserv"];
const LAST: &[&str] = &[
    "dernier",
    "derniere",
    "derniers",
    "dernieres",
    "last",
    "final",
    "finale",
    "finales",
    "finaux",
];
const FIRST: &[&str] = &["premier", "premiere", "premiers", "premieres", "first"];
const RANGE_START: &[&str] = &["de", "du", "from", "entre", "between", "depuis"];
const RANGE_MID: &[&str] = &[
    "a", "au", "to", "until", "till", "through", "et", "and", "jusqu", "jusqua", "jusque",
];
const FROM_ONLY: &[&str] = &["apres", "after", "des", "partir", "since", "past"];
const UNTIL: &[&str] = &[
    "jusqu", "jusqua", "jusque", "until", "till", "avant", "before", "upto",
];
const END_WORDS: &[&str] = &["fin", "end", "bout"];
const START_WORDS: &[&str] = &["debut", "beginning", "start", "commencement"];
const STOP: &[&str] = &[
    "je",
    "j",
    "tu",
    "il",
    "elle",
    "on",
    "nous",
    "vous",
    "ils",
    "veux",
    "veut",
    "voudrais",
    "voulais",
    "aimerais",
    "souhaite",
    "peux",
    "pourrais",
    "pouvez",
    "merci",
    "stp",
    "svp",
    "please",
    "i",
    "want",
    "wanna",
    "would",
    "like",
    "to",
    "could",
    "can",
    "you",
    "the",
    "a",
    "an",
    "of",
    "de",
    "du",
    "des",
    "la",
    "le",
    "les",
    "l",
    "un",
    "une",
    "et",
    "and",
    "then",
    "puis",
    "ensuite",
    "aussi",
    "also",
    "avec",
    "with",
    "video",
    "clip",
    "passage",
    "partie",
    "part",
    "parts",
    "section",
    "segment",
    "bout",
    "morceau",
    "extrait",
    "moi",
    "me",
    "m",
    "ca",
    "ce",
    "cela",
    "that",
    "this",
    "it",
    "qui",
    "que",
    "ne",
    "n",
    "seulement",
    "only",
    "just",
    "juste",
    "sur",
    "in",
    "dans",
    "au",
    "aux",
    "est",
    "sont",
    "faut",
    "y",
    "a",
    "s",
    "d",
    "also",
    "minutes",
    "minute",
    "secondes",
    "seconde",
    "seconds",
    "second",
    "min",
    "sec",
    "s",
    "heure",
    "heures",
    "hour",
    "hours",
    "mn",
    "from",
    "at",
    "please",
    "pls",
    "aussi",
    "ok",
    "bon",
    "alors",
    "donc",
    "so",
    "too",
    "en",
    "plus",
    "fois",
    "egalement",
    "tout",
    "toute",
    "toutes",
    "tous",
    "all",
    "time",
    "temps",
    "il",
    "faudrait",
    "faire",
    "do",
    "make",
    "need",
    "besoin",
    "go",
    "lets",
    "let",
    "us",
    "et",
    "puis",
    "ou",
    "or",
    "but",
    "mais",
    "par",
    "pour",
    "for",
    "by",
    "media",
    "fichier",
    "file",
    "film",
    "audio",
    "enregistrement",
    "recording",
];

fn is(w: &Tok, set: &[&str]) -> bool {
    matches!(w, Tok::Word(x) if set.contains(&x.as_str()))
}

fn stem(w: &Tok, stems: &[&str]) -> bool {
    matches!(w, Tok::Word(x) if stems.iter().any(|s| x.starts_with(s)))
}

struct Reader {
    toks: Vec<Tok>,
    used: Vec<bool>,
}

impl Reader {
    fn time(&self, i: usize) -> Option<(f64, bool)> {
        match self.toks.get(i) {
            Some(Tok::Time {
                seconds, explicit, ..
            }) => Some((*seconds, *explicit)),
            _ => None,
        }
    }
    fn word(&self, i: usize, set: &[&str]) -> bool {
        self.toks.get(i).is_some_and(|t| is(t, set))
    }
    fn mark(&mut self, from: usize, to: usize) {
        for u in &mut self.used[from..to.min(self.toks.len())] {
            *u = true;
        }
    }
    /// A range's middle ("à", "to", "-", "jusqu'à"): the index after it.
    fn mid(&self, i: usize) -> Option<usize> {
        match self.toks.get(i) {
            Some(Tok::Dash) => Some(i + 1),
            Some(t) if is(t, &["jusqu", "jusqua", "jusque"]) => {
                Some(if self.word(i + 1, &["a", "au"]) {
                    i + 2
                } else {
                    i + 1
                })
            }
            Some(t) if is(t, RANGE_MID) => Some(i + 1),
            _ => None,
        }
    }
    /// "la fin", "the end", "fin": the index after it.
    fn end_word(&self, i: usize) -> Option<usize> {
        let j = if self.word(i, &["la", "le", "the"]) {
            i + 1
        } else {
            i
        };
        self.word(j, END_WORDS).then_some(j + 1)
    }
    fn start_word(&self, i: usize) -> Option<usize> {
        let j = if self.word(i, &["le", "la", "the"]) {
            i + 1
        } else {
            i
        };
        self.word(j, START_WORDS).then_some(j + 1)
    }
    /// A unit word right after a bare number that is followed by "dernières"/"first"…
    fn unit_at(&self, i: usize) -> Option<f64> {
        match self.toks.get(i) {
            Some(Tok::Word(w)) => unit(w),
            _ => None,
        }
    }
}

/// Bare numbers in a range take the unit of the other side ("de 30 à 45 secondes", "from 2 to 3 min").
fn pair(a: (f64, bool), b: (f64, bool), a_raw: f64, b_text: &Tok) -> (f64, f64) {
    match (a.1, b.1, b_text) {
        (false, true, Tok::Time { text, .. }) => {
            let u = text.split_whitespace().nth(1).and_then(unit).unwrap_or(1.0);
            (a_raw * u, b.0)
        }
        _ => (a.0, b.0),
    }
}

/// Read an instruction into edits; see the module docs.
pub fn parse_instruction(text: &str) -> Parsed {
    let toks = tokenize(text);
    let mut r = Reader {
        used: vec![false; toks.len()],
        toks,
    };
    let mut edits = Vec::new();
    let mut op = Op::Delete;
    let mut i = 0;
    while i < r.toks.len() {
        let t = r.toks[i].clone();
        if stem(&t, KEEP_STEMS) {
            op = Op::Keep;
            r.mark(i, i + 1);
            i += 1;
            continue;
        }
        if stem(&t, DELETE_STEMS) {
            op = Op::Delete;
            r.mark(i, i + 1);
            i += 1;
            continue;
        }
        // "de/from/entre X à/to/et Y", "du début à Y", "de X à la fin".
        if is(&t, RANGE_START) {
            let a = r
                .time(i + 1)
                .map(|a| (a, i + 2))
                .or_else(|| r.start_word(i + 1).map(|j| ((0.0, true), j)));
            if let Some((a, j)) = a
                && let Some(k) = r.mid(j)
            {
                if let Some(b) = r.time(k) {
                    let (s, e) = pair(a, b, a.0, &r.toks[k]);
                    edits.push(Edit {
                        op,
                        span: Span::Between { start: s, end: e },
                    });
                    r.mark(i, k + 1);
                    i = k + 1;
                    continue;
                }
                if let Some(end) = r.end_word(k) {
                    edits.push(Edit {
                        op,
                        span: Span::From { start: a.0 },
                    });
                    r.mark(i, end);
                    i = end;
                    continue;
                }
            }
        }
        // "start-0:30", "début à 1:00" (no "de" before it).
        if let Some(j) = r.start_word(i)
            && let Some(k) = r.mid(j)
            && let Some((e, _)) = r.time(k)
        {
            edits.push(Edit {
                op,
                span: Span::Between { start: 0.0, end: e },
            });
            r.mark(i, k + 1);
            i = k + 1;
            continue;
        }
        // "à partir de X", "après X", "from X" (to the end), optionally "jusqu'à la fin".
        if is(&t, FROM_ONLY) || (is(&t, RANGE_START) && r.time(i + 1).is_some()) {
            let j = if r.word(i, &["partir"]) && r.word(i + 1, &["de", "du"]) {
                i + 2
            } else {
                i + 1
            };
            if let Some((s, _)) = r.time(j) {
                let mut end = j + 1;
                if let Some(k) = r.mid(end)
                    && let Some(e) = r.end_word(k)
                {
                    end = e;
                } else if let Some(e) = r.end_word(end) {
                    end = e;
                }
                if r.word(i.wrapping_sub(1), &["a"]) {
                    r.mark(i - 1, i);
                }
                edits.push(Edit {
                    op,
                    span: Span::From { start: s },
                });
                r.mark(i, end);
                i = end;
                continue;
            }
        }
        // "jusqu'à X", "until X", "avant X" (from the start).
        if is(&t, UNTIL) {
            let j = if r.word(i + 1, &["a", "au"]) {
                i + 2
            } else {
                i + 1
            };
            if let Some((e, _)) = r.time(j) {
                edits.push(Edit {
                    op,
                    span: Span::Until { end: e },
                });
                r.mark(i, j + 1);
                i = j + 1;
                continue;
            }
        }
        // "les/the last X", "dernières X".
        if is(&t, LAST) || is(&t, FIRST) {
            let last = is(&t, LAST);
            if let Some((mut s, explicit)) = r.time(i + 1) {
                let mut end = i + 2;
                if !explicit && let Some(u) = r.unit_at(end) {
                    s *= u;
                    end += 1;
                }
                edits.push(Edit {
                    op,
                    span: if last {
                        Span::Last { seconds: s }
                    } else {
                        Span::First { seconds: s }
                    },
                });
                r.mark(i, end);
                i = end;
                continue;
            }
        }
        if let Some((mut s, explicit)) = r.time(i) {
            // "les 5:42 dernières minutes", "les 30 premières secondes".
            if r.word(i + 1, LAST) || r.word(i + 1, FIRST) {
                let last = r.word(i + 1, LAST);
                let mut end = i + 2;
                if let Some(u) = r.unit_at(end) {
                    if !explicit {
                        s *= u;
                    }
                    end += 1;
                }
                edits.push(Edit {
                    op,
                    span: if last {
                        Span::Last { seconds: s }
                    } else {
                        Span::First { seconds: s }
                    },
                });
                r.mark(i, end);
                i = end;
                continue;
            }
            // "X à/to/- Y", "X à la fin".
            if let Some(k) = r.mid(i + 1) {
                if let Some(b) = r.time(k) {
                    let (st, e) = pair((s, explicit), b, s, &r.toks[k]);
                    edits.push(Edit {
                        op,
                        span: Span::Between { start: st, end: e },
                    });
                    r.mark(i, k + 1);
                    i = k + 1;
                    continue;
                }
                if let Some(end) = r.end_word(k) {
                    edits.push(Edit {
                        op,
                        span: Span::From { start: s },
                    });
                    r.mark(i, end);
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    let mut unparsed: Vec<String> = Vec::new();
    for (t, used) in r.toks.iter().zip(&r.used) {
        let s = match t {
            _ if *used => continue,
            Tok::Word(w)
                if STOP.contains(&w.as_str())
                    || is(t, LAST)
                    || is(t, FIRST)
                    || is(t, END_WORDS)
                    || is(t, START_WORDS)
                    || w.len() < 2 =>
            {
                continue;
            }
            Tok::Word(w) => w.clone(),
            Tok::Time { text, .. } => text.clone(),
            Tok::Dash => continue,
        };
        if !unparsed.contains(&s) {
            unparsed.push(s);
        }
    }
    Parsed { edits, unparsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span::*;

    fn spans(text: &str) -> Vec<Edit> {
        let p = parse_instruction(text);
        assert!(p.unparsed.is_empty(), "{text}: unparsed {:?}", p.unparsed);
        p.edits
    }

    #[test]
    fn the_brief_example() {
        assert_eq!(
            spans("je veux supprimer de 15:39 à 15:43 à supprimer et les 5:42 dernières minutes"),
            vec![
                Edit::delete(Between {
                    start: 939.0,
                    end: 943.0
                }),
                Edit::delete(Last { seconds: 342.0 })
            ]
        );
    }

    #[test]
    fn french_phrasings() {
        assert_eq!(
            spans("enlève les 30 premières secondes"),
            vec![Edit::delete(First { seconds: 30.0 })]
        );
        assert_eq!(
            spans("coupe les 5 dernières minutes"),
            vec![Edit::delete(Last { seconds: 300.0 })]
        );
        assert_eq!(
            spans("supprime à partir de 12:00"),
            vec![Edit::delete(From { start: 720.0 })]
        );
        assert_eq!(
            spans("supprime de 12:00 jusqu'à la fin"),
            vec![Edit::delete(From { start: 720.0 })]
        );
        assert_eq!(
            spans("supprime du début à 0:45"),
            vec![Edit::delete(Between {
                start: 0.0,
                end: 45.0
            })]
        );
        assert_eq!(
            spans("supprime jusqu'à 1:10"),
            vec![Edit::delete(Until { end: 70.0 })]
        );
        assert_eq!(
            spans("supprime entre 2 et 3 minutes"),
            vec![Edit::delete(Between {
                start: 120.0,
                end: 180.0
            })]
        );
        assert_eq!(
            spans("ne garde que de 1:00 à 2:30"),
            vec![Edit::keep(Between {
                start: 60.0,
                end: 150.0
            })]
        );
        assert_eq!(
            spans("supprime de 1 min 30 à 2 min"),
            vec![Edit::delete(Between {
                start: 90.0,
                end: 120.0
            })]
        );
    }

    #[test]
    fn english_phrasings() {
        assert_eq!(
            spans("delete 1:00-1:20 and the last 2 minutes"),
            vec![
                Edit::delete(Between {
                    start: 60.0,
                    end: 80.0
                }),
                Edit::delete(Last { seconds: 120.0 })
            ]
        );
        assert_eq!(
            spans("cut from 12:00 to the end"),
            vec![Edit::delete(From { start: 720.0 })]
        );
        assert_eq!(
            spans("remove the first 10 seconds"),
            vec![Edit::delete(First { seconds: 10.0 })]
        );
        assert_eq!(
            spans("keep 0:30 to 1:45"),
            vec![Edit::keep(Between {
                start: 30.0,
                end: 105.0
            })]
        );
        assert_eq!(
            spans("start-0:30"),
            vec![Edit::delete(Between {
                start: 0.0,
                end: 30.0
            })]
        );
        assert_eq!(
            spans("12:00-end"),
            vec![Edit::delete(From { start: 720.0 })]
        );
        assert_eq!(
            spans("trim 15:39–15:43, then 20:00 - 20:05."),
            vec![
                Edit::delete(Between {
                    start: 939.0,
                    end: 943.0
                }),
                Edit::delete(Between {
                    start: 1200.0,
                    end: 1205.0
                })
            ]
        );
    }

    #[test]
    fn leaves_what_it_cannot_read() {
        let p = parse_instruction("supprime le passage où je parle du prix");
        assert!(p.edits.is_empty());
        assert!(p.unparsed.contains(&"prix".to_string()), "{:?}", p.unparsed);
        let p = parse_instruction("supprime de 1:00 à 1:10 et les euh");
        assert_eq!(p.edits.len(), 1);
        assert_eq!(p.unparsed, vec!["euh".to_string()]);
        assert!(!p.is_complete());
        // A lone time is not a range.
        assert_eq!(
            parse_instruction("supprime 15:39").unparsed,
            vec!["15:39".to_string()]
        );
    }
}
