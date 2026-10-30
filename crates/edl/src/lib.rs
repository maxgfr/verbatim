//! Cut lists for verbatim, shared by the CLI and the site (through `verbatim-wasm`).
//!
//! An edit is a span to delete or keep (15:39–15:43, the first ten minutes,
//! the last 5:42); resolving the edits against the media's duration gives the
//! ranges to delete, sorted and merged. Nothing here reads natural language:
//! the agent does, and hands over option values (`spec`). Everything is pure:
//! no I/O, no clock, no randomness.

mod export;
mod plan;
mod ranges;
mod spec;
mod time;
mod transcript;

pub use export::{Chapter, ExportDoc, Format, render};
pub use plan::{PLAN_SYSTEM, Plan, parse_plan, plan_prompt};
pub use ranges::{Range, complement, keeps, merge, resolve, total};
pub use spec::{parse_range, parse_seconds};
pub use time::{format_clock, parse_duration, parse_time};
pub use transcript::{
    DEFAULT_FILLERS, Piece, Transcript, fillers, fillers_in, find_text, join_words, remap,
    segments_from_words, silences, words_from_segments,
};

use serde::{Deserialize, Serialize};

/// Delete the span, or keep only the span (everything outside the kept spans goes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Delete,
    Keep,
}

/// Where an edit applies, before the duration is known.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "span", rename_all = "lowercase")]
pub enum Span {
    /// From `start` to `end`, in seconds.
    Between { start: f64, end: f64 },
    /// The first `seconds` of the media.
    First { seconds: f64 },
    /// The last `seconds` of the media.
    Last { seconds: f64 },
    /// From `start` to the end.
    From { start: f64 },
    /// From the beginning to `end`.
    Until { end: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    pub op: Op,
    #[serde(flatten)]
    pub span: Span,
}

impl Edit {
    pub const fn delete(span: Span) -> Self {
        Self {
            op: Op::Delete,
            span,
        }
    }
    pub const fn keep(span: Span) -> Self {
        Self { op: Op::Keep, span }
    }
}

/// The file format both sides read and write (`cuts.json`): the resolved
/// ranges to delete, and the duration they were resolved against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutList {
    pub version: u32,
    /// Duration of the source, in seconds.
    pub duration: f64,
    /// Ranges to delete, sorted, merged, inside `[0, duration]`.
    pub delete: Vec<Range>,
    /// The source's file name, for the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl CutList {
    pub const VERSION: u32 = 1;

    pub fn new(duration: f64, delete: Vec<Range>) -> Self {
        // Milliseconds: finer is float noise from the model's timings.
        let ms = |t: f64| (t * 1000.0).round() / 1000.0;
        let delete = merge(
            delete
                .into_iter()
                .filter_map(|r| r.clamp(duration))
                .map(|r| Range::new(ms(r.start), ms(r.end)))
                .collect(),
        );
        Self {
            version: Self::VERSION,
            duration,
            delete,
            source: None,
        }
    }

    /// The ranges that stay, in order.
    pub fn keeps(&self) -> Vec<Range> {
        keeps(&self.delete, self.duration)
    }

    /// Length of the result, in seconds.
    pub fn output_duration(&self) -> f64 {
        self.duration - total(&self.delete)
    }

    /// The same cuts as `verbatim cut` arguments: `--delete 15:39-15:43 …`.
    pub fn cli_args(&self) -> Vec<String> {
        self.delete
            .iter()
            .flat_map(|r| {
                [
                    "--delete".to_string(),
                    format!(
                        "{}-{}",
                        format_clock(r.start, true),
                        format_clock(r.end, true)
                    ),
                ]
            })
            .collect()
    }
}
