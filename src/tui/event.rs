use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::app::{App, Mode, PickerKind};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return Ok(());
    }
    match &app.mode {
        Mode::List => match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::Down | KeyCode::Char('j') => app.move_down(),
            KeyCode::Char('a') => app.begin_add(),
            KeyCode::Char('e') => app.begin_edit(),
            KeyCode::Char('d') => app.begin_delete(),
            KeyCode::Char('s') => app.open_picker(PickerKind::Sort),
            KeyCode::Char('v') => app.open_picker(PickerKind::View),
            KeyCode::Char('l') => app.open_picker(PickerKind::Lang),
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            _ => {}
        },
        Mode::ConfirmDelete => match key.code {
            KeyCode::Char('y') | KeyCode::Char('e') | KeyCode::Enter => app.confirm_delete()?,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => app.cancel(),
            _ => {}
        },
        Mode::Error(_) => app.cancel(),
        Mode::Picker(_) => {
            let Mode::Picker(p) = &mut app.mode else {
                unreachable!()
            };
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => p.up(),
                KeyCode::Down | KeyCode::Char('j') => p.down(),
                KeyCode::Enter => app.confirm_picker()?,
                KeyCode::Esc | KeyCode::Char('q') => app.cancel(),
                _ => {}
            }
        }
        Mode::Form(_) => crate::tui::form::handle_key(app, key)?,
    }
    Ok(())
}
