use chrono::{NaiveDate, NaiveTime};
use chrono_tz::Tz;
use i18n_embed_fl::fl;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum DateError {
    BadDate(String),
    BadTime(String),
    BadTz(String),
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DateError::BadDate(v) => fl!(crate::i18n::LOADER, "error-bad-date", value = v.as_str()),
            DateError::BadTime(v) => fl!(crate::i18n::LOADER, "error-bad-time", value = v.as_str()),
            DateError::BadTz(v) => fl!(crate::i18n::LOADER, "error-bad-tz", value = v.as_str()),
        };
        f.write_str(&s)
    }
}

impl std::error::Error for DateError {}

pub fn parse_date(s: &str) -> Result<NaiveDate, DateError> {
    let s = s.trim();
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%d.%m.%Y"))
        .map_err(|_| DateError::BadDate(s.to_string()))
}

pub fn parse_time(s: &str) -> Result<NaiveTime, DateError> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| DateError::BadTime(s.to_string()))
}

pub fn parse_tz(s: &str) -> Result<Tz, DateError> {
    let s = s.trim();
    s.parse::<Tz>().map_err(|_| DateError::BadTz(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_date() {
        assert_eq!(
            parse_date("1994-05-12").unwrap(),
            NaiveDate::from_ymd_opt(1994, 5, 12).unwrap()
        );
    }
    #[test]
    fn tr_date() {
        assert_eq!(
            parse_date("12.05.1994").unwrap(),
            NaiveDate::from_ymd_opt(1994, 5, 12).unwrap()
        );
    }
    #[test]
    fn rejects_feb_31() {
        assert!(matches!(
            parse_date("31.02.2000"),
            Err(DateError::BadDate(_))
        ));
    }
    #[test]
    fn rejects_garbage() {
        assert!(parse_date("yesterday").is_err());
    }
    #[test]
    fn time_ok() {
        assert_eq!(
            parse_time("04:30").unwrap(),
            NaiveTime::from_hms_opt(4, 30, 0).unwrap()
        );
    }
    #[test]
    fn time_bad() {
        assert!(matches!(parse_time("4h30"), Err(DateError::BadTime(_))));
    }
    #[test]
    fn tz_ok() {
        assert_eq!(
            parse_tz("Europe/Istanbul").unwrap(),
            chrono_tz::Europe::Istanbul
        );
    }
    #[test]
    fn tz_bad() {
        assert!(matches!(parse_tz("Mars/Olympus"), Err(DateError::BadTz(_))));
    }
    #[test]
    fn bad_date_message_is_localized() {
        let _g = crate::i18n::test_lock();
        assert_eq!(
            DateError::BadDate("x".into()).to_string(),
            "\"x\" is not a date. Use YYYY-MM-DD or DD.MM.YYYY."
        );
    }
}
