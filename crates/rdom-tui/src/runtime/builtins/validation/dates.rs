//! The date and time microsyntaxes of HTML §2.3.5, as the date-like
//! `<input>` states parse their value, `min` and `max`
//! (§4.10.5.1.7–§4.10.5.1.11): a string either is a valid date, month,
//! week, time or normalized-or-not local date and time string, or it is
//! no value at all (value sanitization turns it into `""`).
//!
//! Each parse returns a number that orders like the moment it names —
//! days, months, weeks or seconds on one axis per state — which is all
//! the range checks (`rangeUnderflow` / `rangeOverflow`, `:in-range`)
//! compare. Steps are not checked for these states (DIVERGENCES §2).

use rdom_core::InputTypeState;

/// `s` parsed by the microsyntax of the date-like `state`, or `None`
/// when it is not a valid string of that kind (or `state` is not one of
/// date, month, week, time, datetime-local).
pub(super) fn parse(state: InputTypeState, s: &str) -> Option<f64> {
    use InputTypeState as T;
    match state {
        T::Date => date(s).map(|d| d as f64),
        T::Month => {
            let (y, m, rest) = year_month(s)?;
            rest.is_empty()
                .then_some((y * 12 + i64::from(m) - 1) as f64)
        }
        T::Week => week(s),
        T::Time => time(s),
        T::DateTimeLocal => {
            let split = s.find(['T', ' '])?;
            let days = date(&s[..split])?;
            let secs = time(&s[split + 1..])?;
            Some(days as f64 * 86_400.0 + secs)
        }
        _ => None,
    }
}

/// Whether `state` is one of the date-like states this module parses.
pub(super) fn is_date_like(state: InputTypeState) -> bool {
    use InputTypeState as T;
    matches!(
        state,
        T::Date | T::Month | T::Week | T::Time | T::DateTimeLocal
    )
}

/// §2.3.5.1 "valid month string" prefix: four or more digits (a year
/// above zero), `-`, two digits (01–12). Returns the rest of the string.
fn year_month(s: &str) -> Option<(i64, u32, &str)> {
    let (year, rest) = digits(s, 4, usize::MAX)?;
    if year == 0 {
        return None;
    }
    let rest = rest.strip_prefix('-')?;
    let (month, rest) = digits(rest, 2, 2)?;
    (1..=12)
        .contains(&month)
        .then_some((year as i64, month as u32, rest))
}

/// §2.3.5.2 "valid date string" — `YYYY-MM-DD`, the day valid in its
/// month and year — as days since 1970-01-01.
fn date(s: &str) -> Option<i64> {
    let (y, m, rest) = year_month(s)?;
    let rest = rest.strip_prefix('-')?;
    let (d, rest) = digits(rest, 2, 2)?;
    let d = d as u32;
    (rest.is_empty() && (1..=days_in_month(y, m)).contains(&d)).then(|| days_from_civil(y, m, d))
}

/// §2.3.5.8 "valid week string" — `YYYY-Www`, the week between 1 and
/// the year's week count (53 when the year starts on a Thursday, or on
/// a Wednesday in a leap year; else 52).
fn week(s: &str) -> Option<f64> {
    let (year, rest) = digits(s, 4, usize::MAX)?;
    if year == 0 {
        return None;
    }
    let rest = rest.strip_prefix("-W")?;
    let (w, rest) = digits(rest, 2, 2)?;
    let year = year as i64;
    // 1970-01-01 was a Thursday: weekday 0 = Monday.
    let jan1 = (days_from_civil(year, 1, 1) + 3).rem_euclid(7);
    let weeks = if jan1 == 3 || (jan1 == 2 && is_leap(year)) {
        53
    } else {
        52
    };
    (rest.is_empty() && (1..=weeks).contains(&w)).then_some((year * 53 + w as i64) as f64)
}

/// §2.3.5.4 "valid time string" — `HH:MM`, optionally `:SS` and a
/// fraction of one to three digits — as seconds since midnight.
fn time(s: &str) -> Option<f64> {
    let (h, rest) = digits(s, 2, 2)?;
    let rest = rest.strip_prefix(':')?;
    let (m, rest) = digits(rest, 2, 2)?;
    if h > 23 || m > 59 {
        return None;
    }
    let mut secs = (h * 3600 + m * 60) as f64;
    let Some(rest) = rest.strip_prefix(':') else {
        return rest.is_empty().then_some(secs);
    };
    let (sec, rest) = digits(rest, 2, 2)?;
    if sec > 59 {
        return None;
    }
    secs += sec as f64;
    let Some(fraction) = rest.strip_prefix('.') else {
        return rest.is_empty().then_some(secs);
    };
    let (f, rest) = digits(fraction, 1, 3)?;
    rest.is_empty()
        .then(|| secs + f as f64 / 10f64.powi(fraction.len() as i32))
}

/// Between `min` and `max` ASCII digits at the start of `s`, as a
/// number, and the rest.
fn digits(s: &str, min: usize, max: usize) -> Option<(u64, &str)> {
    let n = s.bytes().take_while(u8::is_ascii_digit).count();
    if n < min || n > max || n > 18 {
        return None;
    }
    Some((s[..n].parse().ok()?, &s[n..]))
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the proleptic Gregorian date `y-m-d`
/// (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputTypeState as T;

    /// HTML §2.3.5.2: a valid date string, its day valid for the month
    /// and year; anything else is no date.
    #[test]
    fn dates_parse_and_order() {
        assert_eq!(parse(T::Date, "1970-01-01"), Some(0.0));
        assert_eq!(parse(T::Date, "1970-01-02"), Some(1.0));
        assert_eq!(parse(T::Date, "2024-02-29"), Some(19_782.0));
        for bad in [
            "2023-02-29",
            "2026-13-01",
            "2026-1-01",
            "0000-01-01",
            "26-01-01",
            "2026-01-01T",
            "",
        ] {
            assert_eq!(parse(T::Date, bad), None, "{bad}");
        }
        assert!(parse(T::Date, "12026-01-01") > parse(T::Date, "9999-12-31"));
    }

    /// §2.3.5.1, §2.3.5.8: months; weeks, 53 of them only in long years.
    #[test]
    fn months_and_weeks_parse_and_order() {
        assert!(parse(T::Month, "2026-03") > parse(T::Month, "2026-02"));
        assert!(parse(T::Month, "2027-01") > parse(T::Month, "2026-12"));
        assert_eq!(parse(T::Month, "2026-00"), None);
        assert!(
            parse(T::Week, "2026-W53").is_some(),
            "2026 starts on a Thursday"
        );
        assert_eq!(parse(T::Week, "2025-W53"), None);
        assert!(
            parse(T::Week, "2020-W53").is_some(),
            "a leap year starting on Wednesday"
        );
        assert!(parse(T::Week, "2027-W01") > parse(T::Week, "2026-W53"));
        assert_eq!(parse(T::Week, "2026-W00"), None);
        assert_eq!(parse(T::Week, "2026-w10"), None);
    }

    /// §2.3.5.4, §2.3.5.5: times with optional seconds and a one- to
    /// three-digit fraction; local date and times with `T` or a space.
    #[test]
    fn times_and_local_date_times_parse() {
        assert_eq!(parse(T::Time, "00:00"), Some(0.0));
        assert_eq!(parse(T::Time, "23:59:59.5"), Some(86_399.5));
        assert_eq!(parse(T::Time, "08:30:05.125"), Some(30_605.125));
        for bad in [
            "24:00",
            "8:30",
            "08:60",
            "08:30:5",
            "08:30:05.",
            "08:30:05.1234",
        ] {
            assert_eq!(parse(T::Time, bad), None, "{bad}");
        }
        assert_eq!(parse(T::DateTimeLocal, "1970-01-02T00:00"), Some(86_400.0));
        assert_eq!(
            parse(T::DateTimeLocal, "1970-01-02 00:00"),
            parse(T::DateTimeLocal, "1970-01-02T00:00")
        );
        assert_eq!(parse(T::DateTimeLocal, "1970-01-02t00:00"), None);
        assert_eq!(parse(T::Text, "1970-01-01"), None);
    }
}
