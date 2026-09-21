use chrono::NaiveDateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::store::StoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub alias: String,
    pub first_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    pub birth: NaiveDateTime,
    #[serde(default)]
    pub has_time: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tz: Option<Tz>,
    #[serde(default)]
    pub avatar: bool,
}

impl Person {
    pub fn full_name(&self) -> String {
        match &self.last_name {
            Some(l) if !l.is_empty() => format!("{} {}", self.first_name, l),
            _ => self.first_name.clone(),
        }
    }

    /// Uppercased first letters of first and last name, e.g. "AA".
    pub fn initials(&self) -> String {
        let mut s: String = self
            .first_name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default();
        if let Some(l) = &self.last_name {
            s.extend(
                l.chars()
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>()),
            );
        }
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Age,
    Alias,
    Birthday,
}

impl SortKey {}

/// How an age is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgeView {
    #[default]
    Calendar,
    Years,
    Months,
    Weeks,
    Days,
    Hours,
}

impl AgeView {
    pub const ALL: [AgeView; 6] = [
        AgeView::Calendar,
        AgeView::Years,
        AgeView::Months,
        AgeView::Weeks,
        AgeView::Days,
        AgeView::Hours,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default)]
    pub sort: SortKey,
    #[serde(default)]
    pub age_view: AgeView,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StoreFile {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub people: Vec<Person>,
}

pub fn validate_alias(alias: &str) -> Result<(), StoreError> {
    let ok = !alias.is_empty()
        && alias
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(StoreError::BadAlias(alias.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn p(first: &str, last: Option<&str>) -> Person {
        Person {
            alias: "x".into(),
            first_name: first.into(),
            last_name: last.map(Into::into),
            birth: NaiveDate::from_ymd_opt(2000, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            has_time: false,
            tz: None,
            avatar: false,
        }
    }

    #[test]
    fn full_name_and_initials() {
        assert_eq!(p("Ayşe", Some("Aktaş")).full_name(), "Ayşe Aktaş");
        assert_eq!(p("Ayşe", Some("Aktaş")).initials(), "AA");
        assert_eq!(p("can", None).full_name(), "can");
        assert_eq!(p("can", None).initials(), "C");
    }

    #[test]
    fn age_view_cycles_and_defaults() {
        assert_eq!(AgeView::default(), AgeView::Calendar);
        let s: Settings = toml::from_str("sort = \"alias\"").unwrap();
        assert_eq!(s.age_view, AgeView::Calendar);
        let s: Settings = toml::from_str("age_view = \"weeks\"").unwrap();
        assert_eq!(s.age_view, AgeView::Weeks);
    }
}
