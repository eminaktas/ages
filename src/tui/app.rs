use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use i18n_embed_fl::fl;
use ratatui_image::picker::Picker as ImagePicker;
use ratatui_image::protocol::StatefulProtocol;

use crate::age::sort_people;
use crate::model::{AgeView, Person, SortKey};
use crate::output::{sort_label, view_label};
use crate::store::Store;
use crate::tui::form::FormState;
use crate::{avatar, i18n};

pub const SORTS: [SortKey; 3] = [SortKey::Age, SortKey::Alias, SortKey::Birthday];

pub enum Mode {
    List,
    Form(Box<FormState>),
    ConfirmDelete,
    ConfirmRemoveAvatar,
    Error(String),
    Picker(Picker),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Lang,
    Sort,
    View,
}

/// A small single-choice popup (language / sort / age view).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    pub kind: PickerKind,
    /// Localized labels, in option order.
    pub items: Vec<String>,
    pub selected: usize,
}

impl Picker {
    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
    pub fn down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }
}

pub struct App {
    pub store: Store,
    pub selected: usize,
    pub mode: Mode,
    pub now: DateTime<Utc>,
    pub should_quit: bool,
    pub avatars: HashMap<String, StatefulProtocol>,
    pub picker: Option<ImagePicker>,
}

impl App {
    pub fn new(store: Store, picker: Option<ImagePicker>) -> App {
        let mut app = App {
            store,
            selected: 0,
            mode: Mode::List,
            now: Utc::now(),
            should_quit: false,
            avatars: HashMap::new(),
            picker,
        };
        app.resort();
        app.selected = 0;
        app
    }

    fn resort(&mut self) {
        let key = self.store.data.settings.sort;
        let keep = self.selected_person().map(|p| p.alias.clone());
        sort_people(&mut self.store.data.people, key, self.now);
        if let Some(alias) = keep {
            self.select_alias(&alias);
        }
        self.clamp();
    }

    fn clamp(&mut self) {
        let n = self.people().len();
        if n == 0 {
            self.selected = 0;
        } else if self.selected >= n {
            self.selected = n - 1;
        }
    }

    pub fn select_alias(&mut self, alias: &str) {
        if let Some(i) = self.people().iter().position(|p| p.alias == alias) {
            self.selected = i;
        }
    }

    pub fn people(&self) -> &[Person] {
        &self.store.data.people
    }

    pub fn selected_person(&self) -> Option<&Person> {
        self.people().get(self.selected)
    }

    pub fn tick(&mut self, now: DateTime<Utc>) {
        self.now = now;
        self.resort();
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.people().len() {
            self.selected += 1;
        }
    }

    pub fn set_sort(&mut self, key: SortKey) -> Result<()> {
        self.store.data.settings.sort = key;
        self.store.save()?;
        self.resort();
        Ok(())
    }

    pub fn set_lang(&mut self, code: &str) -> Result<()> {
        i18n::select(code);
        self.store.data.settings.lang = Some(code.to_string());
        self.store.save()?;
        Ok(())
    }

    pub fn set_view(&mut self, view: AgeView) -> Result<()> {
        self.store.data.settings.age_view = view;
        self.store.save()?;
        Ok(())
    }

    pub fn open_picker(&mut self, kind: PickerKind) {
        let l = &i18n::LOADER;
        let (items, selected) = match kind {
            PickerKind::Lang => (
                vec![fl!(l, "lang-en"), fl!(l, "lang-tr")],
                if i18n::current() == "tr" { 1 } else { 0 },
            ),
            PickerKind::Sort => (
                SORTS.iter().map(|k| sort_label(*k)).collect(),
                SORTS
                    .iter()
                    .position(|k| *k == self.store.data.settings.sort)
                    .unwrap_or(0),
            ),
            PickerKind::View => (
                AgeView::ALL.iter().map(|v| view_label(*v)).collect(),
                AgeView::ALL
                    .iter()
                    .position(|v| *v == self.store.data.settings.age_view)
                    .unwrap_or(0),
            ),
        };
        self.mode = Mode::Picker(Picker {
            kind,
            items,
            selected,
        });
    }

    /// Apply the highlighted picker option and return to the list.
    pub fn confirm_picker(&mut self) -> Result<()> {
        let Mode::Picker(p) = &self.mode else {
            return Ok(());
        };
        let (kind, i) = (p.kind, p.selected);
        self.mode = Mode::List;
        match kind {
            PickerKind::Lang => self.set_lang(i18n::SUPPORTED[i.min(i18n::SUPPORTED.len() - 1)]),
            PickerKind::Sort => self.set_sort(SORTS[i.min(SORTS.len() - 1)]),
            PickerKind::View => self.set_view(AgeView::ALL[i.min(AgeView::ALL.len() - 1)]),
        }
    }

    pub fn begin_delete(&mut self) {
        if self.selected_person().is_some() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn confirm_delete(&mut self) -> Result<()> {
        if let Some(alias) = self.selected_person().map(|p| p.alias.clone()) {
            self.store.remove(&alias)?;
            self.avatars.remove(&alias);
            self.store.save()?;
        }
        self.mode = Mode::List;
        self.clamp();
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.mode = Mode::List;
    }

    /// Ask before dropping the selected person's avatar; no-op when they have none.
    pub fn begin_remove_avatar(&mut self) {
        if self.selected_person().is_some_and(|p| p.avatar) {
            self.mode = Mode::ConfirmRemoveAvatar;
        }
    }

    pub fn confirm_remove_avatar(&mut self) -> Result<()> {
        if let Some(alias) = self.selected_person().map(|p| p.alias.clone()) {
            let _ = std::fs::remove_file(self.store.avatar_path(&alias));
            if let Some(p) = self.store.find_mut(&alias) {
                p.avatar = false;
                p.pixel = false;
            }
            self.avatars.remove(&alias);
            self.store.save()?;
        }
        self.mode = Mode::List;
        Ok(())
    }

    pub fn begin_add(&mut self) {
        self.mode = Mode::Form(Box::new(FormState::new_add()));
    }

    pub fn begin_edit(&mut self) {
        if let Some(p) = self.selected_person() {
            self.mode = Mode::Form(Box::new(FormState::from_person(p)));
        }
    }

    /// Validate the open form and persist it; on validation failure the form stays open with an error.
    pub fn submit_form(&mut self) -> Result<()> {
        let Mode::Form(form) = &self.mode else {
            return Ok(());
        };
        let out = match form.validate(&self.store) {
            Ok(out) => out,
            Err(msg) => {
                if let Mode::Form(f) = &mut self.mode {
                    f.error = Some(msg);
                }
                return Ok(());
            }
        };
        let editing = form.editing.clone();
        let alias = out.person.alias.clone();
        // Import first (into the *final* alias path) so a bad image never leaves a half-applied rename.
        if let Some(src) = &out.avatar_src {
            if let Err(e) = avatar::import(src, &self.store.avatar_path(&alias), out.pixel) {
                if let Mode::Form(f) = &mut self.mode {
                    f.error = Some(e.to_string());
                }
                return Ok(());
            }
            self.avatars.remove(&alias);
        }
        if let Some(old) = &out.rename_from {
            // The avatar (if any) may already live at the new path; move the old one only if present.
            self.store.rename(old, &alias)?;
            self.avatars.remove(old);
        }
        match editing {
            Some(_) => {
                let p = self.store.find_mut(&alias).expect("edited person exists");
                *p = out.person;
            }
            None => self.store.add(out.person)?,
        }
        self.store.save()?;
        self.mode = Mode::List;
        self.resort();
        self.select_alias(&alias);
        Ok(())
    }

    /// Lazily loads the avatar image for `alias` into a resize protocol; `None` without picker or file.
    pub fn avatar_state(&mut self, alias: &str) -> Option<&mut StatefulProtocol> {
        if !self.avatars.contains_key(alias) {
            let picker = self.picker.as_ref()?;
            self.store.find(alias).filter(|p| p.avatar)?;
            let img = avatar::load(&self.store.avatar_path(alias))?;
            let proto = picker.new_resize_protocol(img);
            self.avatars.insert(alias.to_string(), proto);
        }
        self.avatars.get_mut(alias)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use chrono::NaiveDate;

    pub fn person(alias: &str, first: &str, last: &str, y: i32, m: u32, d: u32) -> Person {
        Person {
            alias: alias.into(),
            first_name: first.into(),
            last_name: if last.is_empty() {
                None
            } else {
                Some(last.into())
            },
            birth: NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(4, 30, 0)
                .unwrap(),
            has_time: true,
            tz: Some(chrono_tz::Europe::Istanbul),
            avatar: false,
            pixel: false,
        }
    }

    /// Three people in a temp store (kept alive by the returned TempDir): anne (oldest), baba, kardes.
    pub fn app3() -> (App, tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
        let guard = crate::i18n::test_lock();
        let d = tempfile::tempdir().unwrap();
        let mut store = Store::open(d.path().into()).unwrap();
        store
            .add(person("baba", "Mehmet", "Aktaş", 1962, 8, 1))
            .unwrap();
        store
            .add(person("anne", "Ayşe", "Aktaş", 1965, 3, 14))
            .unwrap();
        store
            .add(person("kardes", "Can", "Aktaş", 1998, 12, 1))
            .unwrap();
        store.save().unwrap();
        (App::new(store, None), d, guard)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::app3;
    use super::*;
    use crate::model::SortKey;
    use crate::tui::event::handle_key;
    use crossterm::event::{KeyCode, KeyEvent};

    #[test]
    fn sorted_oldest_first_on_new() {
        let (app, _d, _g) = app3();
        assert_eq!(app.selected, 0);
        assert_eq!(app.people()[0].alias, "baba");
        assert_eq!(app.people()[2].alias, "kardes");
    }

    #[test]
    fn navigation_clamps() {
        let (mut app, _d, _g) = app3();
        app.move_up();
        assert_eq!(app.selected, 0);
        app.move_down();
        app.move_down();
        app.move_down();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn cycle_sort_persists_and_keeps_selection() {
        let (mut app, _d, _g) = app3();
        app.selected = 1;
        let alias = app.selected_person().unwrap().alias.clone();
        app.set_sort(SortKey::Alias).unwrap();
        assert_eq!(app.store.data.settings.sort, SortKey::Alias);
        assert_eq!(app.selected_person().unwrap().alias, alias);
        let reopened = Store::open(app.store.dir.clone()).unwrap();
        assert_eq!(reopened.data.settings.sort, SortKey::Alias);
    }

    #[test]
    fn lang_picker_selects_turkish() {
        let (mut app, _d, _g) = app3();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('l'))).unwrap();
        match &app.mode {
            Mode::Picker(p) => {
                assert_eq!(p.kind, PickerKind::Lang);
                assert_eq!(p.items, vec!["English", "Türkçe"]);
                assert_eq!(p.selected, 0);
            }
            _ => panic!("expected picker"),
        }
        handle_key(&mut app, KeyEvent::from(KeyCode::Down)).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Enter)).unwrap();
        assert!(matches!(app.mode, Mode::List));
        assert_eq!(app.store.data.settings.lang.as_deref(), Some("tr"));
        assert_eq!(crate::i18n::current(), "tr");
        let reopened = Store::open(app.store.dir.clone()).unwrap();
        assert_eq!(reopened.data.settings.lang.as_deref(), Some("tr"));
    }

    #[test]
    fn picker_esc_changes_nothing() {
        let (mut app, _d, _g) = app3();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('s'))).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Down)).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc)).unwrap();
        assert!(matches!(app.mode, Mode::List));
        assert_eq!(app.store.data.settings.sort, SortKey::Age);
    }

    #[test]
    fn sort_and_view_pickers_apply() {
        let (mut app, _d, _g) = app3();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('s'))).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.store.data.settings.sort, SortKey::Birthday);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('v'))).unwrap();
        match &app.mode {
            Mode::Picker(p) => assert_eq!(p.items.len(), AgeView::ALL.len()),
            _ => panic!("expected picker"),
        }
        handle_key(&mut app, KeyEvent::from(KeyCode::Down)).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Down)).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Down)).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.store.data.settings.age_view, AgeView::Weeks);
        let reopened = Store::open(app.store.dir.clone()).unwrap();
        assert_eq!(reopened.data.settings.age_view, AgeView::Weeks);
    }

    #[test]
    fn delete_flow() {
        let (mut app, _d, _g) = app3();
        app.selected = 2;
        app.begin_delete();
        assert!(matches!(app.mode, Mode::ConfirmDelete));
        app.confirm_delete().unwrap();
        assert_eq!(app.people().len(), 2);
        assert_eq!(app.selected, 1);
        assert!(matches!(app.mode, Mode::List));
    }

    #[test]
    fn keys() {
        let (mut app, _d, _g) = app3();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.selected, 1);
        handle_key(&mut app, KeyEvent::from(KeyCode::Up)).unwrap();
        assert_eq!(app.selected, 0);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('d'))).unwrap();
        assert!(matches!(app.mode, Mode::ConfirmDelete));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('n'))).unwrap();
        assert!(matches!(app.mode, Mode::List));
        assert_eq!(app.people().len(), 3);
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('a'))).unwrap();
        assert!(matches!(app.mode, Mode::Form(_)));
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc)).unwrap();
        assert!(matches!(app.mode, Mode::List));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('q'))).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits_in_every_mode() {
        use crossterm::event::KeyModifiers;
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let (mut app, _d, _g) = app3();
        handle_key(&mut app, ctrl_c).unwrap();
        assert!(app.should_quit);
        app.should_quit = false;
        app.begin_add();
        handle_key(&mut app, ctrl_c).unwrap();
        assert!(app.should_quit);
        app.should_quit = false;
        app.begin_delete();
        handle_key(&mut app, ctrl_c).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn remove_avatar_flow() {
        let (mut app, d, _g) = app3();
        // `x` does nothing for a person without an avatar.
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('x'))).unwrap();
        assert!(matches!(app.mode, Mode::List));
        // Give baba an avatar on disk, then remove it through the confirm popup.
        let path = app.store.avatar_path("baba");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"png").unwrap();
        app.store.find_mut("baba").unwrap().avatar = true;
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('x'))).unwrap();
        assert!(matches!(app.mode, Mode::ConfirmRemoveAvatar));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('n'))).unwrap();
        assert!(app.store.find("baba").unwrap().avatar);
        assert!(path.exists());
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('x'))).unwrap();
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('y'))).unwrap();
        assert!(matches!(app.mode, Mode::List));
        assert!(!app.store.find("baba").unwrap().avatar);
        assert!(!path.exists());
        assert!(
            !Store::open(d.path().into())
                .unwrap()
                .find("baba")
                .unwrap()
                .avatar
        );
    }

    #[test]
    fn avatar_state_none_without_picker() {
        let (mut app, _d, _g) = app3();
        assert!(app.avatar_state("anne").is_none());
    }
}
