pub mod app;
pub mod event;
pub mod form;
pub mod ui;

use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use crossterm::event::{Event, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;

use crate::store::Store;
use app::{App, Mode};

pub fn run(store: Store) -> Result<()> {
    // Query the terminal for its graphics protocol before entering raw mode.
    // `AGES_HALFBLOCKS=1` skips the query (for terminals that never answer it).
    let picker = if std::env::var_os("AGES_HALFBLOCKS").is_some() {
        Some(Picker::halfblocks())
    } else {
        Picker::from_query_stdio().ok()
    };
    let mut terminal = ratatui::init();
    let mut app = App::new(store, picker);
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    let tick = Duration::from_millis(250);
    while !app.should_quit {
        // `Picker::from_query_stdio` leaves a helper thread waiting for the terminal's reply;
        // in terminals that never answer, that thread may disable raw mode after we enabled it.
        if !crossterm::terminal::is_raw_mode_enabled()? {
            crossterm::terminal::enable_raw_mode()?;
        }
        app.tick(Utc::now());
        terminal.draw(|f| ui::draw(f, app))?;
        if crossterm::event::poll(tick)?
            && let Event::Key(k) = crossterm::event::read()?
            && k.kind == KeyEventKind::Press
            && let Err(e) = event::handle_key(app, k)
        {
            app.mode = Mode::Error(e.to_string());
        }
    }
    Ok(())
}
