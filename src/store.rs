use std::fmt;
use std::path::{Path, PathBuf};

use i18n_embed_fl::fl;

use crate::model::{Person, StoreFile, validate_alias};

#[derive(Debug)]
pub enum StoreError {
    BadAlias(String),
    Duplicate(String),
    Unknown(String),
    Parse { path: PathBuf, reason: String },
    Write { path: PathBuf, reason: String },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let l = &crate::i18n::LOADER;
        let s = match self {
            StoreError::BadAlias(a) => fl!(l, "error-bad-alias", alias = a.as_str()),
            StoreError::Duplicate(a) => fl!(l, "error-duplicate-alias", alias = a.as_str()),
            StoreError::Unknown(a) => fl!(l, "error-unknown-alias", alias = a.as_str()),
            StoreError::Parse { path, reason } => fl!(
                l,
                "error-store-parse",
                path = path.display().to_string(),
                reason = reason.as_str()
            ),
            StoreError::Write { path, reason } => fl!(
                l,
                "error-store-write",
                path = path.display().to_string(),
                reason = reason.as_str()
            ),
        };
        f.write_str(&s)
    }
}

impl std::error::Error for StoreError {}

const PEOPLE_FILE: &str = "people.toml";

pub struct Store {
    pub dir: PathBuf,
    pub data: StoreFile,
}

impl Store {
    /// `flag` > `AGES_HOME` > `~/.ages`.
    pub fn resolve_dir(flag: Option<&Path>) -> PathBuf {
        if let Some(p) = flag {
            return p.to_path_buf();
        }
        if let Ok(p) = std::env::var("AGES_HOME")
            && !p.is_empty()
        {
            return PathBuf::from(p);
        }
        home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ages")
    }

    /// Missing or empty file yields an empty store; malformed file is an error.
    pub fn open(dir: PathBuf) -> Result<Store, StoreError> {
        let path = dir.join(PEOPLE_FILE);
        let data = match std::fs::read_to_string(&path) {
            Ok(text) if text.trim().is_empty() => StoreFile::default(),
            Ok(text) => toml::from_str(&text).map_err(|e| StoreError::Parse {
                path: path.clone(),
                reason: e.to_string(),
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => StoreFile::default(),
            Err(e) => {
                return Err(StoreError::Parse {
                    path,
                    reason: e.to_string(),
                });
            }
        };
        Ok(Store { dir, data })
    }

    /// Atomic: write `people.toml.tmp`, then rename.
    pub fn save(&self) -> Result<(), StoreError> {
        let path = self.people_path();
        let wrap = |reason: String| StoreError::Write {
            path: path.clone(),
            reason,
        };
        std::fs::create_dir_all(&self.dir).map_err(|e| wrap(e.to_string()))?;
        let text = toml::to_string_pretty(&self.data).map_err(|e| wrap(e.to_string()))?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text).map_err(|e| wrap(e.to_string()))?;
        std::fs::rename(&tmp, &path).map_err(|e| wrap(e.to_string()))
    }

    pub fn people_path(&self) -> PathBuf {
        self.dir.join(PEOPLE_FILE)
    }

    pub fn avatar_path(&self, alias: &str) -> PathBuf {
        self.dir.join("avatars").join(format!("{alias}.png"))
    }

    pub fn find(&self, alias: &str) -> Option<&Person> {
        self.data.people.iter().find(|p| p.alias == alias)
    }

    pub fn find_mut(&mut self, alias: &str) -> Option<&mut Person> {
        self.data.people.iter_mut().find(|p| p.alias == alias)
    }

    pub fn add(&mut self, p: Person) -> Result<(), StoreError> {
        validate_alias(&p.alias)?;
        if self.find(&p.alias).is_some() {
            return Err(StoreError::Duplicate(p.alias));
        }
        self.data.people.push(p);
        Ok(())
    }

    /// Removes the person and their avatar file (if any).
    pub fn remove(&mut self, alias: &str) -> Result<Person, StoreError> {
        let idx = self
            .data
            .people
            .iter()
            .position(|p| p.alias == alias)
            .ok_or_else(|| StoreError::Unknown(alias.to_string()))?;
        let removed = self.data.people.remove(idx);
        if removed.avatar {
            let _ = std::fs::remove_file(self.avatar_path(alias));
        }
        Ok(removed)
    }

    /// Renames the alias and moves the avatar file (if any).
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), StoreError> {
        validate_alias(to)?;
        if from != to && self.find(to).is_some() {
            return Err(StoreError::Duplicate(to.to_string()));
        }
        let (old, new) = (self.avatar_path(from), self.avatar_path(to));
        let person = self
            .find_mut(from)
            .ok_or_else(|| StoreError::Unknown(from.to_string()))?;
        if person.avatar && from != to && old.exists() {
            std::fs::rename(&old, &new).map_err(|e| StoreError::Write {
                path: new.clone(),
                reason: e.to_string(),
            })?;
        }
        person.alias = to.to_string();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use tempfile::tempdir;

    fn person(alias: &str) -> Person {
        Person {
            alias: alias.into(),
            first_name: "Ayşe".into(),
            last_name: Some("Aktaş".into()),
            birth: NaiveDate::from_ymd_opt(1965, 3, 14)
                .unwrap()
                .and_hms_opt(4, 30, 0)
                .unwrap(),
            has_time: true,
            tz: Some(chrono_tz::Europe::Istanbul),
            avatar: false,
            pixel: false,
        }
    }

    #[test]
    fn open_missing_is_empty() {
        let d = tempdir().unwrap();
        let s = Store::open(d.path().into()).unwrap();
        assert!(s.data.people.is_empty());
    }

    #[test]
    fn open_empty_file_is_empty() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("people.toml"), "").unwrap();
        assert!(Store::open(d.path().into()).unwrap().data.people.is_empty());
    }

    #[test]
    fn round_trip() {
        let d = tempdir().unwrap();
        let mut s = Store::open(d.path().into()).unwrap();
        s.add(person("anne")).unwrap();
        s.save().unwrap();
        let s2 = Store::open(d.path().into()).unwrap();
        assert_eq!(s2.data.people, vec![person("anne")]);
        assert!(!d.path().join("people.toml.tmp").exists());
    }

    #[test]
    fn duplicate_rejected() {
        let d = tempdir().unwrap();
        let mut s = Store::open(d.path().into()).unwrap();
        s.add(person("anne")).unwrap();
        assert!(matches!(
            s.add(person("anne")),
            Err(StoreError::Duplicate(_))
        ));
    }

    #[test]
    fn bad_alias_rejected() {
        for a in ["", "a/b", "a\\b", ".."] {
            assert!(
                matches!(validate_alias(a), Err(StoreError::BadAlias(_))),
                "{a}"
            );
            let d = tempdir().unwrap();
            let mut s = Store::open(d.path().into()).unwrap();
            assert!(s.add(person(a)).is_err());
        }
        assert!(validate_alias("anne_2-x").is_ok());
        assert!(validate_alias("Eşim").is_ok());
        assert!(validate_alias("anne baba").is_ok());
    }

    #[test]
    fn malformed_not_overwritten() {
        let d = tempdir().unwrap();
        let p = d.path().join("people.toml");
        std::fs::write(&p, "[[people]]\nalias = 3").unwrap();
        assert!(matches!(
            Store::open(d.path().into()),
            Err(StoreError::Parse { .. })
        ));
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "[[people]]\nalias = 3"
        );
    }

    #[test]
    fn rename_moves_avatar_and_rejects_dupe() {
        let d = tempdir().unwrap();
        let mut s = Store::open(d.path().into()).unwrap();
        let mut a = person("anne");
        a.avatar = true;
        s.add(a).unwrap();
        s.add(person("baba")).unwrap();
        std::fs::create_dir_all(d.path().join("avatars")).unwrap();
        std::fs::write(s.avatar_path("anne"), b"png").unwrap();
        assert!(matches!(
            s.rename("anne", "baba"),
            Err(StoreError::Duplicate(_))
        ));
        assert!(s.avatar_path("anne").exists());
        assert!(matches!(
            s.rename("anne", "bad/alias"),
            Err(StoreError::BadAlias(_))
        ));
        assert!(matches!(
            s.rename("ghost", "x"),
            Err(StoreError::Unknown(_))
        ));
        s.rename("anne", "mom").unwrap();
        assert!(s.avatar_path("mom").exists());
        assert!(!s.avatar_path("anne").exists());
        assert!(s.find("mom").is_some());
        assert!(s.find("anne").is_none());
    }

    #[test]
    fn remove_deletes_avatar() {
        let d = tempdir().unwrap();
        let mut s = Store::open(d.path().into()).unwrap();
        let mut a = person("anne");
        a.avatar = true;
        s.add(a).unwrap();
        std::fs::create_dir_all(d.path().join("avatars")).unwrap();
        std::fs::write(s.avatar_path("anne"), b"png").unwrap();
        let removed = s.remove("anne").unwrap();
        assert_eq!(removed.alias, "anne");
        assert!(!s.avatar_path("anne").exists());
        assert!(matches!(s.remove("anne"), Err(StoreError::Unknown(_))));
    }

    #[test]
    fn resolve_dir_precedence() {
        assert_eq!(
            Store::resolve_dir(Some(Path::new("/x"))),
            PathBuf::from("/x")
        );
        unsafe { std::env::set_var("AGES_HOME", "/y") };
        assert_eq!(Store::resolve_dir(None), PathBuf::from("/y"));
        assert_eq!(
            Store::resolve_dir(Some(Path::new("/x"))),
            PathBuf::from("/x")
        );
        unsafe { std::env::remove_var("AGES_HOME") };
        assert!(Store::resolve_dir(None).ends_with(".ages"));
    }
}
