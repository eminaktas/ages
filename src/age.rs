use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};
use i18n_embed_fl::fl;
use serde::Serialize;

use crate::model::{Person, SortKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AgeBreakdown {
    pub years: u32,
    pub months: u32,
    pub days: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Zodiac {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

impl Zodiac {
    pub fn symbol(self) -> &'static str {
        match self {
            Zodiac::Aries => "♈",
            Zodiac::Taurus => "♉",
            Zodiac::Gemini => "♊",
            Zodiac::Cancer => "♋",
            Zodiac::Leo => "♌",
            Zodiac::Virgo => "♍",
            Zodiac::Libra => "♎",
            Zodiac::Scorpio => "♏",
            Zodiac::Sagittarius => "♐",
            Zodiac::Capricorn => "♑",
            Zodiac::Aquarius => "♒",
            Zodiac::Pisces => "♓",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Zodiac::Aries => "aries",
            Zodiac::Taurus => "taurus",
            Zodiac::Gemini => "gemini",
            Zodiac::Cancer => "cancer",
            Zodiac::Leo => "leo",
            Zodiac::Virgo => "virgo",
            Zodiac::Libra => "libra",
            Zodiac::Scorpio => "scorpio",
            Zodiac::Sagittarius => "sagittarius",
            Zodiac::Capricorn => "capricorn",
            Zodiac::Aquarius => "aquarius",
            Zodiac::Pisces => "pisces",
        }
    }

    pub fn label(self) -> String {
        let l = &crate::i18n::LOADER;
        match self {
            Zodiac::Aries => fl!(l, "zodiac-aries"),
            Zodiac::Taurus => fl!(l, "zodiac-taurus"),
            Zodiac::Gemini => fl!(l, "zodiac-gemini"),
            Zodiac::Cancer => fl!(l, "zodiac-cancer"),
            Zodiac::Leo => fl!(l, "zodiac-leo"),
            Zodiac::Virgo => fl!(l, "zodiac-virgo"),
            Zodiac::Libra => fl!(l, "zodiac-libra"),
            Zodiac::Scorpio => fl!(l, "zodiac-scorpio"),
            Zodiac::Sagittarius => fl!(l, "zodiac-sagittarius"),
            Zodiac::Capricorn => fl!(l, "zodiac-capricorn"),
            Zodiac::Aquarius => fl!(l, "zodiac-aquarius"),
            Zodiac::Pisces => fl!(l, "zodiac-pisces"),
        }
    }
}

fn local_from_naive<T: TimeZone>(tz: &T, naive: NaiveDateTime) -> DateTime<T> {
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(t) => t,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .unwrap_or_else(|| tz.from_utc_datetime(&naive)),
    }
}

/// Resolved timezone view of a person: birth instant and "now" as a local naive datetime.
struct Zone {
    birth: DateTime<Utc>,
    now_local: NaiveDateTime,
}

fn zone(p: &Person, now: DateTime<Utc>) -> Zone {
    match p.tz {
        Some(tz) => Zone {
            birth: local_from_naive(&tz, p.birth).with_timezone(&Utc),
            now_local: now.with_timezone(&tz).naive_local(),
        },
        None => Zone {
            birth: local_from_naive(&chrono::Local, p.birth).with_timezone(&Utc),
            now_local: now.with_timezone(&chrono::Local).naive_local(),
        },
    }
}

fn days_in_month(y: i32, m: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
    let next = if m == 12 {
        NaiveDate::from_ymd_opt(y + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(y, m + 1, 1)
    }
    .unwrap();
    (next - first).num_days() as u32
}

/// Calendar difference `b -> a` (requires `b <= a`): years, months, days, hours, minutes, seconds.
fn calendar_diff(b: NaiveDateTime, a: NaiveDateTime) -> (u32, u32, u32, u32, u32, u32) {
    let mut secs = a.num_seconds_from_midnight() as i64 - b.num_seconds_from_midnight() as i64;
    let mut a_date = a.date();
    if secs < 0 {
        secs += 86_400;
        a_date = a_date.pred_opt().unwrap();
    }
    let (by, bm, bd) = (b.year(), b.month() as i32, b.day() as i32);
    let (mut ay, mut am, ad) = (a_date.year(), a_date.month() as i32, a_date.day() as i32);
    // Anniversary day clamps to the current month's length (Feb 29 -> Feb 28).
    let effective_bd = bd.min(days_in_month(ay, am as u32) as i32);
    let mut days = ad - effective_bd;
    if days < 0 {
        am -= 1;
        if am == 0 {
            am = 12;
            ay -= 1;
        }
        let dim = days_in_month(ay, am as u32) as i32;
        let anchor = bd.min(dim);
        days = ad + (dim - anchor);
    }
    let mut months = am - bm;
    if months < 0 {
        months += 12;
        ay -= 1;
    }
    let years = (ay - by).max(0);
    (
        years as u32,
        months as u32,
        days.max(0) as u32,
        (secs / 3600) as u32,
        ((secs % 3600) / 60) as u32,
        (secs % 60) as u32,
    )
}

/// The birth moment as a UTC instant, resolved in the person's timezone (or local).
pub fn birth_instant(p: &Person) -> DateTime<Utc> {
    zone(p, Utc::now()).birth
}

pub fn age_at(p: &Person, now: DateTime<Utc>) -> AgeBreakdown {
    let z = zone(p, now);
    let (birth, now_local) = if p.has_time {
        (p.birth, z.now_local)
    } else {
        (
            p.birth.date().and_hms_opt(0, 0, 0).unwrap(),
            z.now_local.date().and_hms_opt(0, 0, 0).unwrap(),
        )
    };
    if now_local < birth {
        return AgeBreakdown {
            years: 0,
            months: 0,
            days: 0,
            hours: p.has_time.then_some(0),
            minutes: p.has_time.then_some(0),
            seconds: p.has_time.then_some(0),
        };
    }
    let (y, m, d, h, mi, s) = calendar_diff(birth, now_local);
    AgeBreakdown {
        years: y,
        months: m,
        days: d,
        hours: p.has_time.then_some(h),
        minutes: p.has_time.then_some(mi),
        seconds: p.has_time.then_some(s),
    }
}

/// Exact elapsed time since birth (in whole seconds; day-granular when the time is unknown).
pub fn elapsed(p: &Person, now: DateTime<Utc>) -> Duration {
    let z = zone(p, now);
    if p.has_time {
        (now - z.birth).max(Duration::zero())
    } else {
        let days = (z.now_local.date() - p.birth.date()).num_days().max(0);
        Duration::days(days)
    }
}

pub fn zodiac(date: NaiveDate) -> Zodiac {
    match (date.month(), date.day()) {
        (3, 21..) | (4, ..=19) => Zodiac::Aries,
        (4, _) | (5, ..=20) => Zodiac::Taurus,
        (5, _) | (6, ..=20) => Zodiac::Gemini,
        (6, _) | (7, ..=22) => Zodiac::Cancer,
        (7, _) | (8, ..=22) => Zodiac::Leo,
        (8, _) | (9, ..=22) => Zodiac::Virgo,
        (9, _) | (10, ..=22) => Zodiac::Libra,
        (10, _) | (11, ..=21) => Zodiac::Scorpio,
        (11, _) | (12, ..=21) => Zodiac::Sagittarius,
        (12, _) | (1, ..=19) => Zodiac::Capricorn,
        (1, _) | (2, ..=18) => Zodiac::Aquarius,
        _ => Zodiac::Pisces,
    }
}

fn birthday_in_year(birth: NaiveDate, year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, birth.month(), birth.day())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, 3, 1).unwrap())
}

/// Whole days until the next birthday in the person's timezone; 0 means today.
pub fn next_birthday_days(p: &Person, now: DateTime<Utc>) -> i64 {
    let today = zone(p, now).now_local.date();
    let mut candidate = birthday_in_year(p.birth.date(), today.year());
    if candidate < today {
        candidate = birthday_in_year(p.birth.date(), today.year() + 1);
    }
    (candidate - today).num_days()
}

pub fn sort_people(people: &mut [Person], key: SortKey, now: DateTime<Utc>) {
    match key {
        SortKey::Age => people.sort_by(|a, b| {
            birth_instant(a)
                .cmp(&birth_instant(b))
                .then_with(|| a.alias.cmp(&b.alias))
        }),
        SortKey::Alias => people.sort_by(|a, b| a.alias.cmp(&b.alias)),
        SortKey::Birthday => people.sort_by(|a, b| {
            next_birthday_days(a, now)
                .cmp(&next_birthday_days(b, now))
                .then_with(|| a.alias.cmp(&b.alias))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::Tz;

    fn p(y: i32, m: u32, d: u32, hm: Option<(u32, u32)>, tz: Option<Tz>) -> Person {
        let (h, mi) = hm.unwrap_or((0, 0));
        Person {
            alias: format!("p{y}{m}{d}"),
            first_name: "X".into(),
            last_name: None,
            birth: NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, mi, 0)
                .unwrap(),
            has_time: hm.is_some(),
            tz,
            avatar: false,
        }
    }
    fn utc(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, mi, s).unwrap()
    }

    #[test]
    fn borrow_from_january() {
        let a = age_at(
            &p(2023, 1, 31, None, Some(Tz::UTC)),
            utc(2023, 3, 1, 12, 0, 0),
        );
        assert_eq!((a.years, a.months, a.days), (0, 1, 1));
    }

    #[test]
    fn leap_birthday_non_leap_year() {
        let a = age_at(
            &p(2000, 2, 29, None, Some(Tz::UTC)),
            utc(2023, 2, 28, 12, 0, 0),
        );
        assert_eq!((a.years, a.months, a.days), (23, 0, 0));
        let b = age_at(
            &p(2000, 2, 29, None, Some(Tz::UTC)),
            utc(2023, 2, 27, 12, 0, 0),
        );
        assert_eq!(b.years, 22);
    }

    #[test]
    fn hms_hidden_without_time() {
        let a = age_at(
            &p(1990, 5, 5, None, Some(Tz::UTC)),
            utc(2020, 5, 5, 10, 0, 0),
        );
        assert_eq!((a.years, a.months, a.days, a.hours), (30, 0, 0, None));
    }

    #[test]
    fn hms_borrow_day() {
        let a = age_at(
            &p(1990, 5, 5, Some((10, 0)), Some(Tz::UTC)),
            utc(2020, 5, 5, 9, 30, 15),
        );
        assert_eq!(
            (a.years, a.months, a.days, a.hours, a.minutes, a.seconds),
            (29, 11, 29, Some(23), Some(30), Some(15))
        );
    }

    #[test]
    fn same_instant_two_timezones_same_age() {
        let ist = p(1990, 1, 1, Some((3, 0)), Some(chrono_tz::Europe::Istanbul));
        let utc0 = p(1990, 1, 1, Some((1, 0)), Some(Tz::UTC));
        assert_eq!(birth_instant(&ist), birth_instant(&utc0));
        let now = utc(2020, 6, 1, 0, 0, 0);
        assert_eq!(age_at(&ist, now).years, age_at(&utc0, now).years);
    }

    #[test]
    fn dst_gap_does_not_panic() {
        let a = p(2024, 3, 31, Some((2, 30)), Some(chrono_tz::Europe::Berlin));
        let _ = birth_instant(&a);
        let _ = age_at(&a, utc(2025, 1, 1, 0, 0, 0));
    }

    #[test]
    fn elapsed_totals() {
        let a = p(2000, 1, 1, Some((0, 0)), Some(Tz::UTC));
        assert_eq!(
            elapsed(&a, utc(2000, 1, 8, 12, 0, 0)),
            Duration::hours(7 * 24 + 12)
        );
        let b = p(2000, 1, 1, None, Some(Tz::UTC));
        assert_eq!(elapsed(&b, utc(2000, 1, 8, 12, 0, 0)), Duration::days(7));
        assert_eq!(elapsed(&b, utc(1999, 1, 1, 0, 0, 0)), Duration::zero());
    }

    #[test]
    fn zodiac_boundaries() {
        use Zodiac::*;
        let z = |m, d| zodiac(NaiveDate::from_ymd_opt(2001, m, d).unwrap());
        assert_eq!(z(3, 20), Pisces);
        assert_eq!(z(3, 21), Aries);
        assert_eq!(z(4, 19), Aries);
        assert_eq!(z(4, 20), Taurus);
        assert_eq!(z(8, 1), Leo);
        assert_eq!(z(12, 21), Sagittarius);
        assert_eq!(z(12, 22), Capricorn);
        assert_eq!(z(1, 19), Capricorn);
        assert_eq!(z(1, 20), Aquarius);
        assert_eq!(z(2, 18), Aquarius);
        assert_eq!(z(2, 19), Pisces);
    }

    #[test]
    fn next_birthday() {
        let a = p(1990, 6, 15, None, Some(Tz::UTC));
        assert_eq!(next_birthday_days(&a, utc(2020, 6, 15, 12, 0, 0)), 0);
        assert_eq!(next_birthday_days(&a, utc(2020, 6, 14, 12, 0, 0)), 1);
        assert_eq!(next_birthday_days(&a, utc(2020, 6, 16, 12, 0, 0)), 364);
    }

    #[test]
    fn next_birthday_leap_to_mar1() {
        let a = p(2000, 2, 29, None, Some(Tz::UTC));
        assert_eq!(next_birthday_days(&a, utc(2023, 2, 28, 12, 0, 0)), 1);
        assert_eq!(next_birthday_days(&a, utc(2024, 2, 29, 12, 0, 0)), 0);
    }

    #[test]
    fn sort_age_desc_ties_alias() {
        let mut v = vec![
            p(2000, 1, 1, None, Some(Tz::UTC)),
            p(1990, 1, 1, None, Some(Tz::UTC)),
            p(1995, 1, 1, None, Some(Tz::UTC)),
        ];
        v[0].alias = "b".into();
        v[1].alias = "c".into();
        v[2].alias = "a".into();
        let now = utc(2020, 1, 1, 0, 0, 0);
        sort_people(&mut v, SortKey::Age, now);
        assert_eq!(
            v.iter().map(|p| p.alias.as_str()).collect::<Vec<_>>(),
            ["c", "a", "b"]
        );
        sort_people(&mut v, SortKey::Alias, now);
        assert_eq!(
            v.iter().map(|p| p.alias.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        let mut same = vec![
            p(2000, 1, 1, None, Some(Tz::UTC)),
            p(2000, 1, 1, None, Some(Tz::UTC)),
        ];
        same[0].alias = "z".into();
        same[1].alias = "y".into();
        sort_people(&mut same, SortKey::Age, now);
        assert_eq!(same[0].alias, "y");
    }

    #[test]
    fn sort_birthday_soonest_first() {
        let now = utc(2020, 6, 1, 0, 0, 0);
        let mut v = vec![
            p(1990, 12, 1, None, Some(Tz::UTC)),
            p(1990, 6, 2, None, Some(Tz::UTC)),
        ];
        sort_people(&mut v, SortKey::Birthday, now);
        assert_eq!(v[0].birth.month(), 6);
    }
}
