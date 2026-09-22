use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use i18n_embed_fl::fl;

use crate::commands::build_person;
use crate::i18n::LOADER;
use crate::model::{Person, validate_alias};
use crate::store::{Store, StoreError};
use crate::tui::app::{App, Mode};

/// Field order: alias, first name, last name, date, time, tz, avatar path, pixel art.
pub const FIELDS: [&str; 8] = [
    "field-alias",
    "field-first-name",
    "field-last-name",
    "field-date",
    "field-time",
    "field-tz",
    "field-avatar",
    "field-pixel",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormState {
    /// Alias of the person being edited; `None` when adding.
    pub editing: Option<String>,
    pub fields: [String; 8],
    pub focus: usize,
    pub error: Option<String>,
    /// The person being edited already has a stored avatar (shown as a hint on the avatar field).
    pub has_avatar: bool,
}

impl FormState {
    pub fn new_add() -> Self {
        FormState {
            editing: None,
            fields: Default::default(),
            focus: 0,
            error: None,
            has_avatar: false,
        }
    }

    pub fn from_person(p: &Person) -> Self {
        FormState {
            editing: Some(p.alias.clone()),
            fields: [
                p.alias.clone(),
                p.first_name.clone(),
                p.last_name.clone().unwrap_or_default(),
                p.birth.format("%Y-%m-%d").to_string(),
                if p.has_time {
                    p.birth.format("%H:%M").to_string()
                } else {
                    String::new()
                },
                p.tz.map(|t| t.name().to_string()).unwrap_or_default(),
                String::new(),
                String::new(),
            ],
            focus: 0,
            error: None,
            has_avatar: p.avatar,
        }
    }

    pub fn next(&mut self) {
        self.focus = (self.focus + 1) % FIELDS.len();
    }

    pub fn prev(&mut self) {
        self.focus = (self.focus + FIELDS.len() - 1) % FIELDS.len();
    }

    pub fn input(&mut self, c: char) {
        self.fields[self.focus].push(c);
        self.error = None;
    }

    pub fn backspace(&mut self) {
        self.fields[self.focus].pop();
        self.error = None;
    }

    /// Validates every field against the store; the error string is already localized.
    pub fn validate(&self, store: &Store) -> Result<FormOutput, String> {
        let [alias, first, last, date, time, tz, avatar, pixel] = &self.fields;
        let alias = alias.trim();
        validate_alias(alias).map_err(|e| e.to_string())?;
        let is_self = self.editing.as_deref() == Some(alias);
        if !is_self && store.find(alias).is_some() {
            return Err(StoreError::Duplicate(alias.to_string()).to_string());
        }
        if first.trim().is_empty() {
            return Err(fl!(LOADER, "error-empty-name"));
        }
        let mut person = build_person(
            alias,
            first.trim(),
            Some(last.trim()),
            date,
            Some(time.trim()).filter(|t| !t.is_empty()),
            Some(tz.trim()).filter(|t| !t.is_empty()),
        )
        .map_err(|e| e.to_string())?;
        let avatar_src = Some(avatar.trim())
            .filter(|a| !a.is_empty())
            .map(PathBuf::from);
        if let Some(src) = &avatar_src {
            crate::avatar::probe(src).map_err(|e| e.to_string())?;
        }
        let pixel = parse_pixel(pixel)?;
        let existing = self.editing.as_deref().and_then(|a| store.find(a));
        // A new image decides its own style; without one the stored avatar (and its style) stays.
        (person.avatar, person.pixel) = if avatar_src.is_some() {
            (true, pixel.is_some())
        } else {
            existing.map_or((false, false), |p| (p.avatar, p.pixel))
        };
        let rename_from = self.editing.clone().filter(|old| old != alias);
        Ok(FormOutput {
            person,
            avatar_src,
            pixel,
            rename_from,
        })
    }
}

#[derive(Debug)]
pub struct FormOutput {
    pub person: Person,
    pub avatar_src: Option<PathBuf>,
    /// Mosaic grid when the new avatar should be stored as pixel art.
    pub pixel: Option<u32>,
    pub rename_from: Option<String>,
}

/// Parse the pixel-art field: empty / no → photo; yes (`y`, `e`, `evet`, `yes`) → default grid;
/// a number → that grid size. Anything else is an error.
pub fn parse_pixel(s: &str) -> Result<Option<u32>, String> {
    let s = s.trim().to_lowercase();
    match s.as_str() {
        "" | "n" | "h" | "no" | "hayır" | "hayir" => Ok(None),
        "y" | "e" | "yes" | "evet" => Ok(Some(crate::avatar::DEFAULT_PIXEL_GRID)),
        _ => s
            .parse::<u32>()
            .ok()
            .filter(|g| (2..=256).contains(g))
            .map(Some)
            .ok_or_else(|| fl!(LOADER, "error-bad-pixel", value = s.as_str())),
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    let Mode::Form(form) = &mut app.mode else {
        return Ok(());
    };
    match key.code {
        KeyCode::Esc => app.cancel(),
        KeyCode::Enter => app.submit_form()?,
        KeyCode::Tab | KeyCode::Down => form.next(),
        KeyCode::BackTab | KeyCode::Up => form.prev(),
        KeyCode::Backspace => form.backspace(),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => form.input(c),
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::test_support::app3;
    use crate::tui::app::{App, Mode};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(app: &mut App, code: KeyCode) {
        handle_key(app, KeyEvent::from(code)).unwrap();
    }
    fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            key(app, KeyCode::Char(c));
        }
    }
    fn tab(app: &mut App) {
        key(app, KeyCode::Tab);
    }
    fn form(app: &App) -> &FormState {
        match &app.mode {
            Mode::Form(f) => f,
            _ => panic!("not in form mode"),
        }
    }

    #[test]
    fn add_via_form() {
        let (mut app, d, _g) = app3();
        app.begin_add();
        type_str(&mut app, "dede");
        tab(&mut app);
        type_str(&mut app, "Ali");
        tab(&mut app);
        tab(&mut app);
        type_str(&mut app, "01.01.1940");
        tab(&mut app);
        type_str(&mut app, "07:15");
        tab(&mut app);
        type_str(&mut app, "Europe/Istanbul");
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::List));
        let p = app.store.find("dede").unwrap();
        assert_eq!(p.first_name, "Ali");
        assert!(p.has_time);
        assert_eq!(
            p.birth.format("%Y-%m-%d %H:%M").to_string(),
            "1940-01-01 07:15"
        );
        assert_eq!(app.selected_person().unwrap().alias, "dede");
        let reopened = crate::store::Store::open(d.path().into()).unwrap();
        assert!(reopened.find("dede").is_some());
    }

    #[test]
    fn duplicate_alias_error_keeps_form_open() {
        let (mut app, _d, _g) = app3();
        app.begin_add();
        type_str(&mut app, "anne");
        tab(&mut app);
        type_str(&mut app, "X");
        tab(&mut app);
        tab(&mut app);
        type_str(&mut app, "2000-01-01");
        key(&mut app, KeyCode::Enter);
        assert!(
            form(&app)
                .error
                .as_deref()
                .unwrap()
                .contains("already exists")
        );
        assert_eq!(app.people().len(), 3);
    }

    #[test]
    fn empty_name_and_bad_date_errors() {
        let (mut app, _d, _g) = app3();
        app.begin_add();
        type_str(&mut app, "x");
        key(&mut app, KeyCode::Enter);
        assert!(form(&app).error.as_deref().unwrap().contains("First name"));
        tab(&mut app);
        type_str(&mut app, "X");
        tab(&mut app);
        tab(&mut app);
        type_str(&mut app, "nope");
        key(&mut app, KeyCode::Enter);
        assert!(form(&app).error.as_deref().unwrap().contains("not a date"));
    }

    #[test]
    fn bad_avatar_path_error() {
        let (mut app, _d, _g) = app3();
        app.begin_add();
        type_str(&mut app, "x");
        tab(&mut app);
        type_str(&mut app, "X");
        tab(&mut app);
        tab(&mut app);
        type_str(&mut app, "2000-01-01");
        for _ in 0..3 {
            tab(&mut app);
        }
        type_str(&mut app, "/definitely/missing.png");
        key(&mut app, KeyCode::Enter);
        assert!(form(&app).error.as_deref().unwrap().contains("as an image"));
        assert!(app.store.find("x").is_none());
    }

    #[test]
    fn existing_non_image_avatar_keeps_form_open_and_does_not_rename() {
        let (mut app, d, _g) = app3();
        let txt = d.path().join("x.txt");
        std::fs::write(&txt, "not an image").unwrap();
        app.selected = 0;
        app.begin_edit();
        for _ in 0..4 {
            key(&mut app, KeyCode::Backspace);
        }
        type_str(&mut app, "mom");
        for _ in 0..6 {
            tab(&mut app);
        }
        type_str(&mut app, txt.to_str().unwrap());
        key(&mut app, KeyCode::Enter);
        assert!(form(&app).error.as_deref().unwrap().contains("as an image"));
        assert!(app.store.find("baba").is_some());
        assert!(app.store.find("mom").is_none());
    }

    #[test]
    fn edit_rename_via_form() {
        let (mut app, _d, _g) = app3();
        app.selected = 0;
        let old = app.selected_person().unwrap().alias.clone();
        app.begin_edit();
        assert_eq!(form(&app).fields[0], old);
        for _ in 0..old.len() {
            key(&mut app, KeyCode::Backspace);
        }
        type_str(&mut app, "mom");
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::List), "{:?}", form(&app).error);
        assert!(app.store.find("mom").is_some());
        assert!(app.store.find(&old).is_none());
        assert_eq!(app.selected_person().unwrap().alias, "mom");
    }

    #[test]
    fn edit_without_new_image_keeps_avatar_and_pixel_flag() {
        let (mut app, _d, _g) = app3();
        {
            let p = app.store.find_mut("baba").unwrap();
            p.avatar = true;
            p.pixel = true;
        }
        app.begin_edit();
        tab(&mut app);
        type_str(&mut app, "X");
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::List), "{:?}", form(&app).error);
        let p = app.store.find("baba").unwrap();
        assert!(p.avatar && p.pixel);
    }

    #[test]
    fn add_with_pixel_avatar_via_form() {
        let (mut app, d, _g) = app3();
        let img = d.path().join("in.png");
        image::RgbImage::from_fn(64, 64, |x, y| image::Rgb([x as u8 * 4, y as u8 * 4, 9]))
            .save(&img)
            .unwrap();
        app.begin_add();
        type_str(&mut app, "dede");
        tab(&mut app);
        type_str(&mut app, "Ali");
        tab(&mut app);
        tab(&mut app);
        type_str(&mut app, "1940-01-01");
        for _ in 0..3 {
            tab(&mut app);
        }
        type_str(&mut app, img.to_str().unwrap());
        tab(&mut app);
        type_str(&mut app, "maybe");
        key(&mut app, KeyCode::Enter);
        assert!(form(&app).error.as_deref().unwrap().contains("pixel"));
        for _ in 0..5 {
            key(&mut app, KeyCode::Backspace);
        }
        type_str(&mut app, "e");
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::List), "{:?}", form(&app).error);
        let p = app.store.find("dede").unwrap();
        assert!(p.avatar && p.pixel);
        assert!(app.store.avatar_path("dede").exists());
    }

    #[test]
    fn parse_pixel_values() {
        assert_eq!(parse_pixel(""), Ok(None));
        assert_eq!(parse_pixel("h"), Ok(None));
        assert_eq!(
            parse_pixel(" E "),
            Ok(Some(crate::avatar::DEFAULT_PIXEL_GRID))
        );
        assert_eq!(parse_pixel("48"), Ok(Some(48)));
        assert!(parse_pixel("1").is_err());
        assert!(parse_pixel("x").is_err());
    }

    #[test]
    fn edit_keeps_alias_when_unchanged() {
        let (mut app, _d, _g) = app3();
        app.begin_edit();
        tab(&mut app);
        type_str(&mut app, "X");
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::List), "{:?}", form(&app).error);
        assert!(app.store.find("baba").unwrap().first_name.ends_with('X'));
    }

    #[test]
    fn shift_tab_moves_back_and_wraps() {
        let (mut app, _d, _g) = app3();
        app.begin_add();
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
        )
        .unwrap();
        assert_eq!(form(&app).focus, FIELDS.len() - 1);
        tab(&mut app);
        assert_eq!(form(&app).focus, 0);
    }

    #[test]
    fn esc_cancels() {
        let (mut app, _d, _g) = app3();
        app.begin_add();
        key(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::List));
    }
}
