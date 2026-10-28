//! Ranges in seconds and the edits resolved into them.

use crate::{Edit, Op, Span};
use serde::{Deserialize, Serialize};

/// `[start, end)` in seconds, serialized as `[start, end]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "(f64, f64)", into = "(f64, f64)")]
pub struct Range {
    pub start: f64,
    pub end: f64,
}

impl From<(f64, f64)> for Range {
    fn from((start, end): (f64, f64)) -> Self {
        Self { start, end }
    }
}

impl From<Range> for (f64, f64) {
    fn from(r: Range) -> Self {
        (r.start, r.end)
    }
}

/// Shorter ranges are rounding noise, not cuts.
const EPSILON: f64 = 0.001;

impl Range {
    pub const fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> f64 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() < EPSILON
    }

    /// The range inside `[0, duration]`, or None when nothing of it is left.
    pub fn clamp(self, duration: f64) -> Option<Self> {
        let (a, b) = (self.start.min(self.end), self.start.max(self.end));
        let r = Self::new(a.max(0.0), b.min(duration));
        (r.start.is_finite() && r.end.is_finite() && !r.is_empty()).then_some(r)
    }
}

/// Sorted, with overlapping and touching ranges joined.
pub fn merge(mut ranges: Vec<Range>) -> Vec<Range> {
    ranges.retain(|r| r.start.is_finite() && r.end.is_finite() && !r.is_empty());
    ranges.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut out: Vec<Range> = Vec::with_capacity(ranges.len());
    for r in ranges {
        match out.last_mut() {
            Some(last) if r.start <= last.end + EPSILON => last.end = last.end.max(r.end),
            _ => out.push(r),
        }
    }
    out
}

/// What `[0, duration]` holds outside `ranges` (which must be merged).
pub fn complement(ranges: &[Range], duration: f64) -> Vec<Range> {
    let mut out = Vec::new();
    let mut at = 0.0;
    for r in ranges {
        if r.start > at {
            out.push(Range::new(at, r.start));
        }
        at = at.max(r.end);
    }
    if duration > at {
        out.push(Range::new(at, duration));
    }
    out.into_iter().filter(|r| !r.is_empty()).collect()
}

/// The ranges that stay once `delete` (merged) is cut out.
pub fn keeps(delete: &[Range], duration: f64) -> Vec<Range> {
    complement(delete, duration)
}

pub fn total(ranges: &[Range]) -> f64 {
    ranges.iter().map(Range::len).sum()
}

fn span_range(span: Span, duration: f64) -> Range {
    match span {
        Span::Between { start, end } => Range::new(start, end),
        Span::First { seconds } => Range::new(0.0, seconds),
        Span::Last { seconds } => Range::new(duration - seconds, duration),
        Span::From { start } => Range::new(start, duration),
        Span::Until { end } => Range::new(0.0, end),
    }
}

/// The ranges to delete: every delete edit, plus everything outside the keep
/// edits when there are any. Sorted, merged, inside `[0, duration]`.
pub fn resolve(edits: &[Edit], duration: f64) -> Vec<Range> {
    let pick = |op: Op| {
        merge(
            edits
                .iter()
                .filter(|e| e.op == op)
                .filter_map(|e| span_range(e.span, duration).clamp(duration))
                .collect(),
        )
    };
    let mut delete = pick(Op::Delete);
    if edits.iter().any(|e| e.op == Op::Keep) {
        delete.extend(complement(&pick(Op::Keep), duration));
    }
    merge(delete)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_overlaps_and_sorts() {
        let m = merge(vec![
            Range::new(10.0, 20.0),
            Range::new(0.0, 5.0),
            Range::new(18.0, 25.0),
            Range::new(25.0, 26.0),
            Range::new(3.0, 3.0),
        ]);
        assert_eq!(m, vec![Range::new(0.0, 5.0), Range::new(10.0, 26.0)]);
    }

    #[test]
    fn resolves_the_brief_example() {
        // "supprimer de 15:39 à 15:43 et les 5:42 dernières minutes" on a 20-minute video.
        let edits = [
            Edit::delete(Span::Between {
                start: 939.0,
                end: 943.0,
            }),
            Edit::delete(Span::Last { seconds: 342.0 }),
        ];
        assert_eq!(resolve(&edits, 1200.0), vec![Range::new(858.0, 1200.0)]);
        assert_eq!(
            resolve(&edits, 3600.0),
            vec![Range::new(939.0, 943.0), Range::new(3258.0, 3600.0)]
        );
    }

    #[test]
    fn keep_deletes_the_rest() {
        let edits = [
            Edit::keep(Span::Between {
                start: 60.0,
                end: 120.0,
            }),
            Edit::delete(Span::Between {
                start: 90.0,
                end: 95.0,
            }),
        ];
        assert_eq!(
            resolve(&edits, 300.0),
            vec![
                Range::new(0.0, 60.0),
                Range::new(90.0, 95.0),
                Range::new(120.0, 300.0)
            ]
        );
    }

    #[test]
    fn clamps_and_flips() {
        assert_eq!(
            Range::new(50.0, -5.0).clamp(30.0),
            Some(Range::new(0.0, 30.0))
        );
        assert_eq!(Range::new(40.0, 50.0).clamp(30.0), None);
        assert_eq!(
            resolve(&[Edit::delete(Span::Last { seconds: 999.0 })], 30.0),
            vec![Range::new(0.0, 30.0)]
        );
    }

    #[test]
    fn complement_of_nothing_is_everything() {
        assert_eq!(complement(&[], 10.0), vec![Range::new(0.0, 10.0)]);
        assert_eq!(complement(&[Range::new(0.0, 10.0)], 10.0), vec![]);
    }
}
