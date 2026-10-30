//! Option values, read strictly and without any language: a range is
//! `START-END` in times (`15:39-15:43`, `939-943`, `1:02:03.5-1:05:00`),
//! where START may be `start` and END `end`; a duration is a time
//! (`5:42`, `342`, `90s`, `5min`, `1h02`). Understanding what someone said,
//! in whatever language, is the agent's job; it hands over these values.

use crate::Span;
use crate::time::parse_time;

/// `START-END` → the span it names.
pub fn parse_range(spec: &str) -> Result<Span, String> {
    let s = spec.trim();
    let (a, b) = s.split_once('-').ok_or_else(|| {
        format!("{spec:?} is not a range: write START-END, as 15:39-15:43, start-0:30 or 12:00-end")
    })?;
    let (a, b) = (a.trim().to_ascii_lowercase(), b.trim().to_ascii_lowercase());
    let time = |t: &str| {
        parse_time(t).ok_or_else(|| {
            format!("{t:?} in {spec:?} is not a time: write 15:39, 1:02:03.5, 939 or 90s")
        })
    };
    let span = match (a.as_str(), b.as_str()) {
        ("start", "end") => Span::Between {
            start: 0.0,
            end: f64::INFINITY,
        },
        ("start", b) => Span::Until { end: time(b)? },
        (a, "end") => Span::From { start: time(a)? },
        (a, b) => {
            let (start, end) = (time(a)?, time(b)?);
            if end < start {
                return Err(format!("{spec:?} ends before it starts"));
            }
            Span::Between { start, end }
        }
    };
    Ok(span)
}

/// A duration (`5:42`, `342`, `90s`, `5min`, `1h02`) in seconds.
pub fn parse_seconds(spec: &str) -> Result<f64, String> {
    parse_time(spec)
        .ok_or_else(|| format!("{spec:?} is not a duration: write 5:42, 342, 90s or 5min"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span::*;

    #[test]
    fn ranges() {
        assert_eq!(
            parse_range("15:39-15:43"),
            Ok(Between {
                start: 939.0,
                end: 943.0
            })
        );
        assert_eq!(
            parse_range(" 1:02:03.5 - 1:05:00 "),
            Ok(Between {
                start: 3723.5,
                end: 3900.0
            })
        );
        assert_eq!(parse_range("start-0:30"), Ok(Until { end: 30.0 }));
        assert_eq!(parse_range("12:00-END"), Ok(From { start: 720.0 }));
        assert_eq!(
            parse_range("90s-2min"),
            Ok(Between {
                start: 90.0,
                end: 120.0
            })
        );
        assert!(
            parse_range("15:43-15:39")
                .unwrap_err()
                .contains("ends before")
        );
        assert!(parse_range("15:39").unwrap_err().contains("not a range"));
        assert!(parse_range("début-fin").is_err());
    }

    #[test]
    fn durations() {
        assert_eq!(parse_seconds("5:42"), Ok(342.0));
        assert_eq!(parse_seconds("90s"), Ok(90.0));
        assert!(parse_seconds("cinq minutes").is_err());
    }
}
