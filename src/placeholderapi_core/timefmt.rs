//! Time patterns for `%server_time_<pattern>%`: the subset of Java's `SimpleDateFormat` that
//! PlaceholderAPI's server time uses in practice, in English.
//!
//! Letters: `y` (`yy` for two digits), `M` (`M`, `MM`, `MMM`, `MMMM`), `d`, `H`, `h` (1-12), `m`,
//! `s`, `S` (milliseconds), `a` (AM/PM), `E` (`EEE`, `EEEE`). Text in `'...'` is literal, `''` is a
//! quote. Any other letter makes the whole pattern invalid (`None`), as `SimpleDateFormat` rejects
//! it; other characters are copied as they are.

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// The pattern applied to `epoch_ms` as seen `offset_secs` east of UTC; `None` for a pattern with a
/// letter it does not know or an unterminated quote.
pub fn format(pattern: &str, epoch_ms: i64, offset_secs: i32) -> Option<String> {
    let local = epoch_ms.div_euclid(1000) + i64::from(offset_secs);
    let millis = epoch_ms.rem_euclid(1000);
    let days = local.div_euclid(86_400);
    let second_of_day = local.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let time = Clock {
        hour: second_of_day / 3600,
        minute: (second_of_day % 3600) / 60,
        second: second_of_day % 60,
        millis,
        year,
        month,
        day,
        // 1970-01-01 was a Thursday.
        weekday: (days + 4).rem_euclid(7) as usize,
    };

    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            if chars.get(i + 1) == Some(&'\'') {
                out.push('\'');
                i += 2;
                continue;
            }
            i += 1;
            loop {
                match chars.get(i) {
                    None => return None,
                    Some(&'\'') if chars.get(i + 1) == Some(&'\'') => {
                        out.push('\'');
                        i += 2;
                    }
                    Some(&'\'') => {
                        i += 1;
                        break;
                    }
                    Some(&other) => {
                        out.push(other);
                        i += 1;
                    }
                }
            }
            continue;
        }
        if c.is_ascii_alphabetic() {
            let mut count = 1usize;
            while chars.get(i + count) == Some(&c) {
                count += 1;
            }
            out.push_str(&field(c, count, &time)?);
            i += count;
            continue;
        }
        out.push(c);
        i += 1;
    }
    Some(out)
}

struct Clock {
    year: i64,
    month: u32,
    day: u32,
    hour: i64,
    minute: i64,
    second: i64,
    millis: i64,
    /// 0 is Sunday.
    weekday: usize,
}

/// One field of `count` letters `letter`, or `None` for a letter this subset does not have.
fn field(letter: char, count: usize, t: &Clock) -> Option<String> {
    Some(match letter {
        'y' => {
            if count == 2 {
                format!("{:02}", t.year.rem_euclid(100))
            } else {
                pad(t.year, count)
            }
        }
        'M' => match count {
            1 | 2 => pad(i64::from(t.month), count),
            3 => MONTHS[t.month as usize - 1][..3].to_owned(),
            _ => MONTHS[t.month as usize - 1].to_owned(),
        },
        'd' => pad(i64::from(t.day), count),
        'H' => pad(t.hour, count),
        'h' => {
            let twelve = match t.hour % 12 {
                0 => 12,
                h => h,
            };
            pad(twelve, count)
        }
        'm' => pad(t.minute, count),
        's' => pad(t.second, count),
        'S' => pad(t.millis, count),
        'a' => (if t.hour < 12 { "AM" } else { "PM" }).to_owned(),
        'E' => {
            let name = WEEKDAYS[t.weekday];
            if count >= 4 {
                name.to_owned()
            } else {
                name[..3].to_owned()
            }
        }
        _ => return None,
    })
}

/// A number, zero-padded to `width` digits.
fn pad(value: i64, width: usize) -> String {
    format!("{value:0width$}")
}

/// Days since 1970-01-01 to (year, month, day) in the proleptic Gregorian calendar.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2024-01-01T00:00:00Z, a Monday.
    const NEW_YEAR: i64 = 1_704_067_200_000;
    /// 2024-02-29T12:00:00Z, a Thursday.
    const LEAP_NOON: i64 = 1_709_208_000_000;
    const THIRTEEN_HOURS: i32 = 13 * 3600;

    #[test]
    fn days_convert_to_calendar_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
    }

    #[test]
    fn numeric_fields_are_padded_to_the_pattern_width() {
        assert_eq!(format("yyyy-MM-dd HH:mm:ss", NEW_YEAR, 0).unwrap(), "2024-01-01 00:00:00");
        assert_eq!(format("y M d H m s", NEW_YEAR, 0).unwrap(), "2024 1 1 0 0 0");
    }

    #[test]
    fn two_digit_years() {
        assert_eq!(format("yy", NEW_YEAR, 0).unwrap(), "24");
        assert_eq!(format("yy", 0, 0).unwrap(), "70");
    }

    #[test]
    fn the_offset_moves_the_day_and_the_hour() {
        assert_eq!(format("yyyy-MM-dd HH:mm", NEW_YEAR, THIRTEEN_HOURS).unwrap(), "2024-01-01 13:00");
        assert_eq!(format("yyyy-MM-dd HH:mm", NEW_YEAR, -3600).unwrap(), "2023-12-31 23:00");
    }

    #[test]
    fn twelve_hour_clock_and_am_pm() {
        assert_eq!(format("h:mm a", NEW_YEAR, 0).unwrap(), "12:00 AM");
        assert_eq!(format("h:mm a", NEW_YEAR, THIRTEEN_HOURS).unwrap(), "1:00 PM");
        assert_eq!(format("hh a", NEW_YEAR, 12 * 3600).unwrap(), "12 PM");
    }

    #[test]
    fn month_and_weekday_names() {
        assert_eq!(format("MMMM MMM MM M", NEW_YEAR, 0).unwrap(), "January Jan 01 1");
        assert_eq!(format("EEEE EEE", NEW_YEAR, 0).unwrap(), "Monday Mon");
        assert_eq!(format("EEEE", LEAP_NOON, 0).unwrap(), "Thursday");
        assert_eq!(format("EEEE", 0, 0).unwrap(), "Thursday", "1970-01-01 was a Thursday");
        assert_eq!(format("EEEE", -4 * 86_400_000, 0).unwrap(), "Sunday");
    }

    #[test]
    fn the_leap_day_is_there() {
        assert_eq!(format("d MMMM yyyy", LEAP_NOON, 0).unwrap(), "29 February 2024");
    }

    #[test]
    fn milliseconds() {
        assert_eq!(format("ss.SSS", NEW_YEAR + 7, 0).unwrap(), "00.007");
        assert_eq!(format("S", NEW_YEAR + 123, 0).unwrap(), "123");
    }

    #[test]
    fn times_before_1970_keep_their_second() {
        assert_eq!(format("yyyy-MM-dd HH:mm:ss.SSS", -1, 0).unwrap(), "1969-12-31 23:59:59.999");
    }

    #[test]
    fn quoted_text_is_literal_and_two_quotes_are_one() {
        assert_eq!(format("'at' HH", NEW_YEAR + 3_600_000, 0).unwrap(), "at 01");
        assert_eq!(format("HH''mm", NEW_YEAR, 0).unwrap(), "00'00");
        assert_eq!(format("'it''s'", 0, 0).unwrap(), "it's");
        assert_eq!(format("''", 0, 0).unwrap(), "'");
    }

    #[test]
    fn other_characters_are_copied() {
        assert_eq!(format("MM/dd/yy HH:mm:ss", NEW_YEAR, 0).unwrap(), "01/01/24 00:00:00");
        assert_eq!(format("[HH] - ", NEW_YEAR, 0).unwrap(), "[00] - ");
    }

    #[test]
    fn an_empty_pattern_is_an_empty_text() {
        assert_eq!(format("", NEW_YEAR, 0).unwrap(), "");
    }

    #[test]
    fn a_letter_the_subset_does_not_have_makes_the_pattern_invalid() {
        for bad in ["x", "HH z", "G", "k", "Z", "u", "w"] {
            assert_eq!(format(bad, NEW_YEAR, 0), None, "{bad:?}");
        }
    }

    #[test]
    fn an_unterminated_quote_is_invalid() {
        assert_eq!(format("'abc", NEW_YEAR, 0), None);
    }
}
