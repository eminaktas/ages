use i18n_embed_fl::fl;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui_image::{FilterType, Resize, StatefulImage};

use crate::age::{next_birthday_days, zodiac};
use crate::i18n::LOADER;
use crate::output::{format_age_view, format_birthday, sort_label, view_label};
use crate::tui::app::{App, Mode, Picker, PickerKind};
use crate::tui::form::{FIELDS, FormState};

/// Content is capped to this width and centered; wide terminals stay airy instead of stretched.
const MAX_WIDTH: u16 = 90;
const AVATAR_COLS: u16 = 16;
const AVATAR_ROWS: u16 = 8;

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let screen = frame.area();
    let width = screen.width.min(MAX_WIDTH);
    let content = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y,
        width,
        height: screen.height,
    };
    // Header, list (exactly as tall as it needs), separator, detail, footer.
    let list_rows = (app.people().len().max(1) as u16).min(content.height.saturating_sub(8).max(1));
    let [header, list, sep, detail, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(list_rows),
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(content);
    draw_header(frame, app, header);
    draw_list(frame, app, list);
    frame.render_widget(
        Paragraph::new(Line::styled(
            "─".repeat(sep.width.saturating_sub(4) as usize),
            dim(),
        ))
        .alignment(Alignment::Center),
        Rect {
            y: sep.y + 1,
            height: 1,
            ..sep
        },
    );
    draw_detail(frame, app, detail);
    draw_footer(frame, app, footer);
    let alias = app
        .selected_person()
        .map(|p| p.alias.as_str())
        .unwrap_or("");
    match &app.mode {
        Mode::List => {}
        Mode::ConfirmDelete => draw_confirm(
            frame,
            screen,
            fl!(LOADER, "tui-delete-title"),
            fl!(LOADER, "tui-delete-body", alias = alias),
            Color::Red,
        ),
        Mode::ConfirmRemoveAvatar => draw_confirm(
            frame,
            screen,
            fl!(LOADER, "tui-remove-avatar-title"),
            fl!(LOADER, "tui-remove-avatar-body", alias = alias),
            Color::Yellow,
        ),
        Mode::Error(msg) => draw_error(frame, msg, screen),
        Mode::Form(form) => draw_form(frame, form, screen),
        Mode::Picker(p) => draw_picker(frame, p, screen),
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "{} · {}",
        fl!(
            LOADER,
            "tui-sort",
            sort = sort_label(app.store.data.settings.sort)
        ),
        fl!(
            LOADER,
            "tui-view",
            view = view_label(app.store.data.settings.age_view)
        ),
    );
    let title = format!("  {}", fl!(LOADER, "tui-title"));
    let pad = area.width as usize;
    let gap = pad.saturating_sub(title.chars().count() + status.chars().count() + 2);
    let line = Line::from(vec![
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(gap)),
        Span::styled(status, dim()),
    ]);
    frame.render_widget(Paragraph::new(line), Rect { height: 1, ..area });
}

fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    if app.people().is_empty() {
        let p =
            Paragraph::new(format!("  {}", fl!(LOADER, "empty-store"))).wrap(Wrap { trim: true });
        frame.render_widget(p, area);
        return;
    }
    let view = app.store.data.settings.age_view;
    let alias_w = app
        .people()
        .iter()
        .map(|p| p.alias.chars().count())
        .max()
        .unwrap_or(5)
        .max(5);
    let name_w = app
        .people()
        .iter()
        .map(|p| p.full_name().chars().count())
        .max()
        .unwrap_or(0);
    let lines: Vec<Line> = app
        .people()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let today = next_birthday_days(p, app.now) == 0;
            let selected = i == app.selected;
            let marker = if selected { "▶ " } else { "  " };
            let age = format_age_view(p, app.now, view);
            let cake = if today { " 🎂" } else { "" };
            let text = format!(
                "  {marker}{:<alias_w$}  {:<name_w$}  {age}{cake}",
                p.alias,
                p.full_name()
            );
            let text: String = text.chars().take(area.width as usize).collect();
            let mut style = Style::default();
            if selected {
                style = style.add_modifier(Modifier::BOLD);
            } else {
                style = style.fg(Color::Gray);
            }
            if today {
                style = style.fg(Color::Yellow);
            }
            Line::styled(text, style)
        })
        .collect();
    let visible = area.height as usize;
    let offset = if visible == 0 {
        0
    } else {
        app.selected.saturating_sub(visible - 1)
    };
    frame.render_widget(Paragraph::new(lines).scroll((offset as u16, 0)), area);
}

fn draw_detail(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(p) = app.selected_person().cloned() else {
        return;
    };
    let inner = Rect {
        x: area.x + 2,
        width: area.width.saturating_sub(4),
        ..area
    };
    let [avatar_area, text_area] =
        Layout::horizontal([Constraint::Length(AVATAR_COLS + 3), Constraint::Min(10)]).areas(inner);
    let avatar_box = Rect {
        width: AVATAR_COLS.min(avatar_area.width),
        height: AVATAR_ROWS.min(avatar_area.height),
        ..avatar_area
    };
    match app.avatar_state(&p.alias) {
        Some(state) => {
            // Scale (not Fit): always fill the box, upscaling small stored avatars too.
            // Pixel art keeps hard edges; photos get a smooth filter.
            let filter = if p.pixel {
                FilterType::Nearest
            } else {
                FilterType::Lanczos3
            };
            let widget = StatefulImage::default().resize(Resize::Scale(Some(filter)));
            frame.render_stateful_widget(widget, avatar_box, state);
        }
        None => {
            let initials = Paragraph::new(p.initials())
                .alignment(Alignment::Center)
                .block(Block::bordered().border_style(dim()));
            frame.render_widget(initials, avatar_box);
        }
    }

    let view = app.store.data.settings.age_view;
    let z = zodiac(p.birth.date());
    let days = next_birthday_days(&p, app.now);
    let birth = if p.has_time {
        p.birth.format("%Y-%m-%d %H:%M").to_string()
    } else {
        format!(
            "{}  ({})",
            p.birth.format("%Y-%m-%d"),
            fl!(LOADER, "time-unknown")
        )
    };
    let mut tz =
        p.tz.map(|t| t.name().to_string())
            .unwrap_or_else(|| fl!(LOADER, "tz-local"));
    if p.avatar && p.pixel {
        tz.push_str(" · ▦ ");
        tz.push_str(&fl!(LOADER, "pixel-tag"));
    }
    let mut bday = Line::from(format!(
        "{} {} · {}",
        z.symbol(),
        z.label(),
        format_birthday(days)
    ));
    bday = if days == 0 {
        bday.style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        bday.style(dim())
    };
    let lines = vec![
        Line::from(Span::styled(
            p.full_name(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::styled(format!("{birth} · {tz}"), dim()),
        Line::from(""),
        Line::from(Span::styled(
            format_age_view(&p, app.now, view),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        bday,
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), text_area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let keys = match &app.mode {
        Mode::List => fl!(LOADER, "tui-help"),
        Mode::Form(_) => fl!(LOADER, "tui-help-form"),
        Mode::ConfirmDelete | Mode::ConfirmRemoveAvatar => fl!(LOADER, "tui-help-confirm"),
        Mode::Error(_) => fl!(LOADER, "tui-help-error"),
        Mode::Picker(_) => fl!(LOADER, "tui-help-pick"),
    };
    frame.render_widget(
        Paragraph::new(Line::styled(format!("  {keys}"), dim())),
        area,
    );
}

fn popup(frame: &mut Frame, area: Rect, title: String, lines: Vec<Line>, color: Color) {
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_style(Style::default().fg(color));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}

/// A popup `w`×`h` centered in `area`, clamped to fit.
fn centered_box(w: u16, h: u16, area: Rect) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// Yes/no popup with a single centered line of text.
fn draw_confirm(frame: &mut Frame, area: Rect, title: String, body: String, color: Color) {
    let r = centered_box((body.chars().count() as u16 + 8).max(30), 5, area);
    popup(
        frame,
        r,
        title,
        vec![Line::from(""), Line::from(body).centered()],
        color,
    );
}

fn draw_error(frame: &mut Frame, msg: &str, area: Rect) {
    let r = centered_box(60, 6, area);
    popup(
        frame,
        r,
        fl!(LOADER, "tui-error-title"),
        vec![Line::from(""), Line::from(msg.to_string())],
        Color::Red,
    );
}

fn draw_picker(frame: &mut Frame, p: &Picker, area: Rect) {
    let title = match p.kind {
        PickerKind::Lang => fl!(LOADER, "tui-pick-lang"),
        PickerKind::Sort => fl!(LOADER, "tui-pick-sort"),
        PickerKind::View => fl!(LOADER, "tui-pick-view"),
    };
    let widest = p.items.iter().map(|s| s.chars().count()).max().unwrap_or(0) as u16;
    let r = centered_box(
        (widest + 8).max(title.chars().count() as u16 + 6).max(20),
        p.items.len() as u16 + 2,
        area,
    );
    let lines = p
        .items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == p.selected {
                Line::styled(
                    format!(" ▶ {label}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Line::from(format!("   {label}"))
            }
        })
        .collect();
    popup(frame, r, title, lines, Color::Cyan);
}

fn field_label(key: &str) -> String {
    match key {
        "field-alias" => fl!(LOADER, "field-alias"),
        "field-first-name" => fl!(LOADER, "field-first-name"),
        "field-last-name" => fl!(LOADER, "field-last-name"),
        "field-date" => fl!(LOADER, "field-date"),
        "field-time" => fl!(LOADER, "field-time"),
        "field-tz" => fl!(LOADER, "field-tz"),
        "field-pixel" => fl!(LOADER, "field-pixel"),
        _ => fl!(LOADER, "field-avatar"),
    }
}

fn draw_form(frame: &mut Frame, form: &FormState, area: Rect) {
    let title = if form.editing.is_some() {
        fl!(LOADER, "tui-form-edit")
    } else {
        fl!(LOADER, "tui-form-add")
    };
    let needed = (FIELDS.len() as u16) * 2 + 4;
    let r = centered_box(70, needed, area);
    let mut lines = Vec::new();
    for (i, key) in FIELDS.iter().enumerate() {
        let focused = i == form.focus;
        let label_style = if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            dim()
        };
        lines.push(Line::from(Span::styled(field_label(key), label_style)));
        let is_avatar_hint = key == &"field-avatar" && form.has_avatar && form.fields[i].is_empty();
        let value = if is_avatar_hint {
            format!(
                "{}{}",
                if focused { "▏" } else { "" },
                fl!(LOADER, "field-avatar-kept")
            )
        } else if focused {
            format!("{}▏", form.fields[i])
        } else {
            form.fields[i].clone()
        };
        let value_style = if is_avatar_hint {
            dim()
        } else if focused {
            Style::default().add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(format!("  {value}"), value_style)));
    }
    if let Some(err) = &form.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    }
    popup(frame, r, title, lines, Color::Cyan);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::test_support::app3;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut t = Terminal::new(backend).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn wide_layout_shows_list_and_detail() {
        let (mut app, _d, _g) = app3();
        let s = render(&mut app, 100, 30);
        assert!(s.contains("▶ baba"), "{s}");
        assert!(s.contains("  anne"), "{s}");
        assert!(s.contains("Mehmet Aktaş"), "{s}");
        assert!(s.contains("Europe/Istanbul"), "{s}");
        assert!(s.contains("1962-08-01 04:30"), "{s}");
        assert!(s.contains("♌"), "{s}");
        assert!(s.contains("Leo"), "{s}");
        assert!(s.contains("a add"), "{s}");
        assert!(s.contains("sort: age · view: calendar"), "{s}");
        assert!(
            !s.contains("┌ ages"),
            "no framed panels in the main layout: {s}"
        );
    }

    #[test]
    fn narrow_layout_stacks() {
        let (mut app, _d, _g) = app3();
        let s = render(&mut app, 60, 40);
        let list_y = s.lines().position(|l| l.contains("▶ baba")).unwrap();
        let detail_y = s
            .lines()
            .position(|l| l.contains("Europe/Istanbul"))
            .unwrap();
        assert!(detail_y > list_y + 5, "{s}");
    }

    #[test]
    fn initials_when_no_avatar() {
        let (mut app, _d, _g) = app3();
        let s = render(&mut app, 100, 30);
        assert!(s.contains("MA"), "{s}");
    }

    #[test]
    fn delete_popup() {
        let (mut app, _d, _g) = app3();
        app.begin_delete();
        let s = render(&mut app, 100, 30);
        assert!(s.contains("Delete baba?"), "{s}");
        assert!(s.contains("y yes"), "{s}");
    }

    #[test]
    fn error_popup() {
        let (mut app, _d, _g) = app3();
        app.mode = crate::tui::app::Mode::Error("boom".into());
        let s = render(&mut app, 100, 30);
        assert!(s.contains("boom"), "{s}");
        assert!(s.contains("press any key"), "{s}");
    }

    #[test]
    fn empty_store_shows_hint() {
        let _g = crate::i18n::test_lock();
        let d = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(d.path().into()).unwrap();
        let mut app = App::new(store, None);
        let s = render(&mut app, 100, 30);
        assert!(s.contains("No people yet"), "{s}");
    }

    #[test]
    fn form_popup_shows_fields_and_error() {
        let (mut app, _d, _g) = app3();
        app.begin_add();
        if let crate::tui::app::Mode::Form(f) = &mut app.mode {
            f.fields[0] = "dede".into();
            f.error = Some("bad thing".into());
        }
        let s = render(&mut app, 100, 30);
        assert!(s.contains("Add person"), "{s}");
        assert!(s.contains("alias"), "{s}");
        assert!(s.contains("dede"), "{s}");
        assert!(s.contains("bad thing"), "{s}");
        assert!(s.contains("Tab next field"), "{s}");
    }

    #[test]
    fn remove_avatar_popup_and_pixel_tag() {
        let (mut app, _d, _g) = app3();
        {
            let p = app.store.find_mut("baba").unwrap();
            p.avatar = true;
            p.pixel = true;
        }
        let s = render(&mut app, 100, 30);
        assert!(s.contains("▦ pixel art"), "{s}");
        assert!(s.contains("x avatar off"), "{s}");
        app.begin_remove_avatar();
        let s = render(&mut app, 100, 30);
        assert!(s.contains("Remove the avatar of baba?"), "{s}");
    }

    #[test]
    fn edit_form_hints_kept_avatar() {
        let (mut app, _d, _g) = app3();
        app.store.find_mut("baba").unwrap().avatar = true;
        app.begin_edit();
        let s = render(&mut app, 100, 30);
        assert!(s.contains("current avatar is kept"), "{s}");
    }

    #[test]
    fn list_scrolls_to_keep_selection_visible() {
        let (mut app, _d, _g) = app3();
        for i in 0..30 {
            app.store
                .add(crate::tui::app::test_support::person(
                    &format!("p{i:02}"),
                    "P",
                    "",
                    2001,
                    1,
                    1 + (i % 28) as u32,
                ))
                .unwrap();
        }
        app.tick(app.now);
        app.selected = app.people().len() - 1;
        let last = app.people().last().unwrap().alias.clone();
        let s = render(&mut app, 100, 12);
        assert!(s.contains(&format!("▶ {last}")), "{s}");
    }
}
