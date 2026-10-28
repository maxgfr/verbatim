//! Times as people write them: `15:39`, `1:02:03.5`, `90`, `5min`, `1h02`,
//! `2 min 30 s`. A colon time is minutes:seconds (or hours:minutes:seconds),
//! never hours:minutes: in a video, `15:39` is a quarter past fifteen minutes.

/// Seconds per unit word, accents already removed and lowercased.
pub(crate) fn unit(word: &str) -> Option<f64> {
    Some(match word {
        "ms" => 0.001,
        "s" | "sec" | "secs" | "seconde" | "secondes" | "second" | "seconds" => 1.0,
        "m" | "mn" | "min" | "mins" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hrs" | "heure" | "heures" | "hour" | "hours" => 3600.0,
        _ => return None,
    })
}

fn number(s: &str) -> Option<f64> {
    if s.is_empty()
        || !s
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
    {
        return None;
    }
    s.replace(',', ".")
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite() && *v >= 0.0)
}

/// `mm:ss`, `h:mm:ss`, with an optional fraction (`.5` or `,5`).
pub(crate) fn colon_time(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let (last, rest) = parts.split_last()?;
    let secs = number(last)?;
    if secs >= 60.0 {
        return None;
    }
    let mut total = secs;
    let mut mul = 60.0;
    for (i, p) in rest.iter().rev().enumerate() {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let v: f64 = p.parse().ok()?;
        // Minutes under an hour field must be below 60.
        if i == 0 && rest.len() == 2 && v >= 60.0 {
            return None;
        }
        total += v * mul;
        mul *= 60.0;
    }
    Some(total)
}

/// A compact unit time: `5min`, `90s`, `1h02`, `15m39`, `1h2m3s`.
fn compact_time(s: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut num = String::new();
    let mut word = String::new();
    let mut last_unit: Option<f64> = None;
    let mut seen = false;
    let flush = |num: &mut String,
                 word: &mut String,
                 total: &mut f64,
                 last_unit: &mut Option<f64>|
     -> Option<()> {
        if num.is_empty() {
            return if word.is_empty() { Some(()) } else { None };
        }
        let v = number(num)?;
        let u = if word.is_empty() {
            // A trailing bare number takes the next smaller unit: 1h02 → 2 min, 15m39 → 39 s.
            match *last_unit {
                Some(u) if u >= 3600.0 => 60.0,
                Some(u) if u >= 60.0 => 1.0,
                _ => return None,
            }
        } else {
            unit(word)?
        };
        *total += v * u;
        *last_unit = Some(u);
        num.clear();
        word.clear();
        Some(())
    };
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' || c == ',' {
            if !word.is_empty() {
                flush(&mut num, &mut word, &mut total, &mut last_unit)?;
            }
            num.push(c);
            seen = true;
        } else if c.is_ascii_alphabetic() {
            if num.is_empty() {
                return None;
            }
            word.push(c);
        } else {
            return None;
        }
    }
    if !seen {
        return None;
    }
    if num.is_empty() {
        return None;
    }
    if word.is_empty() && last_unit.is_none() {
        return None;
    }
    flush(&mut num, &mut word, &mut total, &mut last_unit)?;
    Some(total)
}

/// One time token: a colon time, a unit time, or a bare number of seconds.
pub fn parse_time(s: &str) -> Option<f64> {
    let s = s.trim().to_ascii_lowercase();
    if s.contains(':') {
        return colon_time(&s);
    }
    number(&s).or_else(|| compact_time(&s))
}

/// A duration where bare numbers are seconds: `5:42`, `90`, `2min`, `1h`.
pub fn parse_duration(s: &str) -> Option<f64> {
    parse_time(s)
}

/// `15:39`, `1:02:03`; with `millis`, `15:39.250` when the time has a fraction.
pub fn format_clock(seconds: f64, millis: bool) -> String {
    let ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let (h, m, s, f) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000);
    let base = if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    };
    if millis && f > 0 {
        let frac = format!("{f:03}");
        format!("{base}.{}", frac.trim_end_matches('0'))
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colon_times_are_minutes_seconds() {
        assert_eq!(parse_time("15:39"), Some(939.0));
        assert_eq!(parse_time("0:07"), Some(7.0));
        assert_eq!(parse_time("1:02:03"), Some(3723.0));
        assert_eq!(parse_time("5:42.5"), Some(342.5));
        assert_eq!(parse_time("5:42,5"), Some(342.5));
        assert_eq!(parse_time("5:60"), None);
        assert_eq!(parse_time("1:75:00"), None);
        assert_eq!(parse_time("1::2"), None);
    }

    #[test]
    fn unit_times() {
        assert_eq!(parse_time("90"), Some(90.0));
        assert_eq!(parse_time("5min"), Some(300.0));
        assert_eq!(parse_time("90s"), Some(90.0));
        assert_eq!(parse_time("1h02"), Some(3720.0));
        assert_eq!(parse_time("15m39"), Some(939.0));
        assert_eq!(parse_time("1h2m3s"), Some(3723.0));
        assert_eq!(parse_time("1,5min"), Some(90.0));
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time("min"), None);
        assert_eq!(parse_time("5x"), None);
    }

    #[test]
    fn clock_format() {
        assert_eq!(format_clock(939.0, false), "15:39");
        assert_eq!(format_clock(3723.0, false), "1:02:03");
        assert_eq!(format_clock(7.25, true), "0:07.25");
        assert_eq!(format_clock(7.0, true), "0:07");
        assert_eq!(format_clock(-3.0, false), "0:00");
    }
}
