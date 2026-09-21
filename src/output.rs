use chrono::{DateTime, Utc};
use i18n_embed_fl::fl;
use serde::Serialize;
use unicode_width::UnicodeWidthChar;

use crate::age::{AgeBreakdown, age_at, elapsed, next_birthday_days, zodiac};
use crate::model::{AgeView, Person, SortKey};
use crate::store::Store;

pub struct Row {
    pub alias: String,
    pub name: String,
    pub age: String,
    pub zodiac: String,
    pub birthday: String,
    pub today: bool,
}

pub fn format_age(a: &AgeBreakdown) -> String {
    let base = fl!(
        crate::i18n::LOADER,
        "age-long",
        years = a.years,
        months = a.months,
        days = a.days
    );
    match (a.hours, a.minutes, a.seconds) {
        (Some(h), Some(m), Some(s)) => format!("{base} {h:02}:{m:02}:{s:02}"),
        _ => base,
    }
}

/// Group thousands with a thin space: 22473 -> "22 473".
pub fn group_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Age in the requested view. `Calendar` is `format_age`; the others are totals.
pub fn format_age_view(p: &Person, now: DateTime<Utc>, view: AgeView) -> String {
    let l = &crate::i18n::LOADER;
    let el = elapsed(p, now);
    let days = el.num_days();
    match view {
        AgeView::Calendar => format_age(&age_at(p, now)),
        AgeView::Years => {
            let years = el.num_seconds() as f64 / (365.2425 * 86_400.0);
            fl!(l, "view-years-value", years = format!("{years:.2}"))
        }
        AgeView::Months => {
            let a = age_at(p, now);
            let months = group_thousands(i64::from(a.years * 12 + a.months));
            let rest = i64::from(a.days);
            fl!(l, "view-months-value", months = months, days = rest)
        }
        AgeView::Weeks => {
            let rest: i64 = days % 7;
            fl!(
                l,
                "view-weeks-value",
                weeks = group_thousands(days / 7),
                days = rest
            )
        }
        AgeView::Days => fl!(l, "view-days-value", days = group_thousands(days)),
        AgeView::Hours => fl!(
            l,
            "view-hours-value",
            hours = group_thousands(el.num_hours())
        ),
    }
}

pub fn sort_label(key: SortKey) -> String {
    let l = &crate::i18n::LOADER;
    match key {
        SortKey::Age => fl!(l, "sort-age"),
        SortKey::Alias => fl!(l, "sort-alias"),
        SortKey::Birthday => fl!(l, "sort-birthday"),
    }
}

pub fn view_label(view: AgeView) -> String {
    let l = &crate::i18n::LOADER;
    match view {
        AgeView::Calendar => fl!(l, "view-calendar"),
        AgeView::Years => fl!(l, "view-years"),
        AgeView::Months => fl!(l, "view-months"),
        AgeView::Weeks => fl!(l, "view-weeks"),
        AgeView::Days => fl!(l, "view-days"),
        AgeView::Hours => fl!(l, "view-hours"),
    }
}

pub fn format_birthday(days: i64) -> String {
    if days == 0 {
        fl!(crate::i18n::LOADER, "birthday-today")
    } else {
        fl!(crate::i18n::LOADER, "birthday-in", days = days)
    }
}

pub fn rows(people: &[Person], now: DateTime<Utc>, view: AgeView) -> Vec<Row> {
    people
        .iter()
        .map(|p| {
            let days = next_birthday_days(p, now);
            let z = zodiac(p.birth.date());
            Row {
                alias: p.alias.clone(),
                name: p.full_name(),
                age: format_age_view(p, now, view),
                zodiac: format!("{} {}", z.symbol(), z.label()),
                birthday: format_birthday(days),
                today: days == 0,
            }
        })
        .collect()
}

/// Terminal cell width. Zodiac symbols (U+2648–U+2653) are "ambiguous" in Unicode but every
/// modern terminal draws them as two-cell emoji, so count them as 2.
pub fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '\u{2648}'..='\u{2653}' => 2,
            _ => UnicodeWidthChar::width(c).unwrap_or(0),
        })
        .sum()
}

fn width(s: &str) -> usize {
    display_width(s)
}

fn pad(s: &str, w: usize) -> String {
    let mut out = s.to_string();
    out.extend(std::iter::repeat_n(' ', w.saturating_sub(width(s))));
    out
}

pub fn render_table(rows: &[Row]) -> String {
    let l = &crate::i18n::LOADER;
    let header = [
        fl!(l, "col-alias"),
        fl!(l, "col-name"),
        fl!(l, "col-age"),
        fl!(l, "col-zodiac"),
        fl!(l, "col-birthday"),
    ];
    let cells: Vec<[String; 5]> = rows
        .iter()
        .map(|r| {
            let bday = if r.today {
                format!("🎂 {}", r.birthday)
            } else {
                r.birthday.clone()
            };
            [
                r.alias.clone(),
                r.name.clone(),
                r.age.clone(),
                r.zodiac.clone(),
                bday,
            ]
        })
        .collect();
    let mut widths: Vec<usize> = header.iter().map(|h| width(h)).collect();
    for c in &cells {
        for (i, v) in c.iter().enumerate() {
            widths[i] = widths[i].max(width(v));
        }
    }
    let line = |c: &[String; 5]| {
        c.iter()
            .enumerate()
            .map(|(i, v)| if i == 4 { v.clone() } else { pad(v, widths[i]) })
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let mut out = vec![line(&header)];
    out.extend(cells.iter().map(line));
    out.join("\n") + "\n"
}

#[derive(Serialize)]
struct JsonPerson<'a> {
    alias: &'a str,
    first_name: &'a str,
    last_name: Option<&'a str>,
    birth: String,
    has_time: bool,
    tz: Option<String>,
    avatar: Option<String>,
    age: AgeBreakdown,
    zodiac: &'static str,
    next_birthday_in_days: i64,
}

pub fn render_json(people: &[Person], store: &Store, now: DateTime<Utc>) -> String {
    let items: Vec<JsonPerson> = people
        .iter()
        .map(|p| JsonPerson {
            alias: &p.alias,
            first_name: &p.first_name,
            last_name: p.last_name.as_deref(),
            birth: p.birth.format("%Y-%m-%dT%H:%M:%S").to_string(),
            has_time: p.has_time,
            tz: p.tz.map(|t| t.name().to_string()),
            avatar: p
                .avatar
                .then(|| store.avatar_path(&p.alias).display().to_string()),
            age: age_at(p, now),
            zodiac: zodiac(p.birth.date()).key(),
            next_birthday_in_days: next_birthday_days(p, now),
        })
        .collect();
    serde_json::to_string(&items).expect("serializable") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_formats() {
        let _g = crate::i18n::test_lock();
        let a = AgeBreakdown {
            years: 61,
            months: 6,
            days: 7,
            hours: Some(9),
            minutes: Some(4),
            seconds: Some(32),
        };
        assert_eq!(format_age(&a), "61y 6m 7d 09:04:32");
        let b = AgeBreakdown {
            years: 27,
            months: 9,
            days: 3,
            hours: None,
            minutes: None,
            seconds: None,
        };
        assert_eq!(format_age(&b), "27y 9m 3d");
    }

    #[test]
    fn age_views() {
        let _g = crate::i18n::test_lock();
        let p = Person {
            alias: "a".into(),
            first_name: "A".into(),
            last_name: None,
            birth: chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            has_time: true,
            tz: Some(chrono_tz::Tz::UTC),
            avatar: false,
        };
        let now = chrono::TimeZone::with_ymd_and_hms(&Utc, 2001, 1, 8, 12, 0, 0).unwrap();
        assert_eq!(
            format_age_view(&p, now, AgeView::Calendar),
            "1y 0m 7d 12:00:00"
        );
        assert_eq!(format_age_view(&p, now, AgeView::Years), "1.02 years");
        assert_eq!(
            format_age_view(&p, now, AgeView::Months),
            "12 months 7 days"
        );
        assert_eq!(format_age_view(&p, now, AgeView::Weeks), "53 weeks 2 days");
        assert_eq!(format_age_view(&p, now, AgeView::Days), "373 days");
        assert_eq!(
            format_age_view(&p, now, AgeView::Hours),
            "8\u{202f}964 hours"
        );
        assert_eq!(group_thousands(22473), "22\u{202f}473");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(view_label(AgeView::Weeks), "weeks");
    }

    #[test]
    fn birthday_formats_plural() {
        let _g = crate::i18n::test_lock();
        assert_eq!(format_birthday(0), "birthday today");
        assert_eq!(format_birthday(1), "birthday in 1 day");
        assert_eq!(format_birthday(174), "birthday in 174 days");
    }

    #[test]
    fn table_pads_columns() {
        let _g = crate::i18n::test_lock();
        let rows = vec![
            Row {
                alias: "anne".into(),
                name: "Ayşe Aktaş".into(),
                age: "61y".into(),
                zodiac: "♓ Pisces".into(),
                birthday: "birthday in 3 days".into(),
                today: false,
            },
            Row {
                alias: "k".into(),
                name: "Can".into(),
                age: "1y".into(),
                zodiac: "♌ Leo".into(),
                birthday: "birthday today".into(),
                today: true,
            },
        ];
        let t = render_table(&rows);
        let lines: Vec<&str> = t.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("alias  name        age"));
        assert!(lines[1].starts_with("anne   Ayşe Aktaş  61y"));
        assert!(lines[2].contains("🎂 birthday today"));
        // Every column starts at the same terminal cell on every line, wide glyphs included.
        let col = |line: &str, needle: &str| display_width(&line[..line.find(needle).unwrap()]);
        assert_eq!(
            col(lines[0], "birthday"),
            col(lines[1], "birthday in 3 days")
        );
        assert_eq!(col(lines[0], "birthday"), col(lines[2], "🎂"));
        assert_eq!(col(lines[0], "zodiac"), col(lines[1], "♓"));
        assert_eq!(display_width("♓ Pisces"), 9);
        assert_eq!(display_width("🎂"), 2);
        assert_eq!(display_width("Ayşe"), 4);
    }
}
