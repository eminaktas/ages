use std::io::{IsTerminal, Write};

use anyhow::{Result, bail};
use chrono::Utc;
use i18n_embed_fl::fl;

use crate::age::sort_people;
use crate::cli::{Cli, Command};
use crate::dates::{parse_date, parse_time, parse_tz};
use crate::model::{AgeView, Person, SortKey};
use crate::output::{render_json, render_table, rows};
use crate::store::{Store, StoreError};
use crate::{avatar, i18n};

pub fn build_person(
    alias: &str,
    name: &str,
    surname: Option<&str>,
    birth: &str,
    time: Option<&str>,
    tz: Option<&str>,
) -> Result<Person> {
    let date = parse_date(birth)?;
    let t = time.map(parse_time).transpose()?;
    let tz = tz.map(parse_tz).transpose()?;
    Ok(Person {
        alias: alias.to_string(),
        first_name: name.to_string(),
        last_name: surname.filter(|s| !s.is_empty()).map(str::to_string),
        birth: date.and_time(t.unwrap_or_default()),
        has_time: t.is_some(),
        tz,
        avatar: false,
        pixel: false,
    })
}

fn list(store: &Store, json: bool, sort: Option<SortKey>, view: Option<AgeView>) -> Result<()> {
    let now = Utc::now();
    let mut people = store.data.people.clone();
    sort_people(&mut people, sort.unwrap_or(store.data.settings.sort), now);
    let out = if json {
        render_json(&people, store, now)
    } else if people.is_empty() {
        fl!(i18n::LOADER, "empty-store") + "\n"
    } else {
        render_table(&rows(
            &people,
            now,
            view.unwrap_or(store.data.settings.age_view),
        ))
    };
    print!("{out}");
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(matches!(
        line.trim().to_lowercase().as_str(),
        "y" | "yes" | "e" | "evet"
    ))
}

pub fn run(cli: Cli) -> Result<()> {
    let dir = Store::resolve_dir(cli.data_dir.as_deref());
    let mut store = Store::open(dir)?;
    if cli.lang.is_none()
        && let Some(lang) = store.data.settings.lang.as_deref()
    {
        i18n::select(lang);
    }

    match cli.command {
        None => {
            if std::io::stdout().is_terminal() {
                crate::tui::run(store)
            } else {
                list(&store, false, None, None)
            }
        }
        Some(Command::Tui) => crate::tui::run(store),
        Some(Command::List { json, sort, view }) => {
            list(&store, json, sort.map(Into::into), view.map(Into::into))
        }
        Some(Command::Add {
            alias,
            name,
            surname,
            birth,
            time,
            tz,
            avatar: img,
            pixel,
        }) => {
            let alias = alias.trim().to_string();
            let p = build_person(
                &alias,
                &name,
                surname.as_deref(),
                &birth,
                time.as_deref(),
                tz.as_deref(),
            )?;
            // `add` validates the alias and rejects duplicates before any file is written.
            store.add(p)?;
            if let Some(src) = img {
                avatar::import(&src, &store.avatar_path(&alias), pixel)?;
                let p = store.find_mut(&alias).expect("just added");
                p.avatar = true;
                p.pixel = pixel.is_some();
            }
            store.save()?;
            println!("{}", fl!(i18n::LOADER, "added", alias = alias.as_str()));
            Ok(())
        }
        Some(Command::Edit {
            alias,
            name,
            surname,
            birth,
            time,
            tz,
            avatar: img,
            no_avatar,
            pixel,
            rename,
        }) => {
            if store.find(&alias).is_none() {
                bail!(StoreError::Unknown(alias));
            }
            let rename = rename.map(|r| r.trim().to_string());
            // Validate every input before touching memory or disk.
            let new_date = birth.as_deref().map(parse_date).transpose()?;
            let new_time = time.as_deref().map(parse_time).transpose()?;
            let new_tz = match tz.as_deref() {
                None => None,
                Some("") => Some(None),
                Some(z) => Some(Some(parse_tz(z)?)),
            };
            if let Some(new_alias) = &rename {
                crate::model::validate_alias(new_alias)?;
                if new_alias != &alias && store.find(new_alias).is_some() {
                    bail!(StoreError::Duplicate(new_alias.clone()));
                }
            }
            if let Some(src) = &img {
                avatar::probe(src)?;
            }

            if let Some(new_alias) = &rename {
                store.rename(&alias, new_alias)?;
            }
            let current = rename.clone().unwrap_or_else(|| alias.clone());
            let avatar_path = store.avatar_path(&current);
            if let Some(src) = &img {
                avatar::import(src, &avatar_path, pixel)?;
            }
            let p = store.find_mut(&current).expect("renamed person exists");
            if let Some(n) = name {
                p.first_name = n;
            }
            if let Some(s) = surname {
                p.last_name = if s.is_empty() { None } else { Some(s) };
            }
            if let Some(d) = new_date {
                p.birth = d.and_time(p.birth.time());
            }
            if let Some(t) = new_time {
                p.birth = p.birth.date().and_time(t);
                p.has_time = true;
            }
            if let Some(z) = new_tz {
                p.tz = z;
            }
            if no_avatar {
                let _ = std::fs::remove_file(&avatar_path);
                p.avatar = false;
                p.pixel = false;
            }
            if img.is_some() {
                p.avatar = true;
                p.pixel = pixel.is_some();
            }
            store.save()?;
            println!("{}", fl!(i18n::LOADER, "updated", alias = current.as_str()));
            Ok(())
        }
        Some(Command::Remove { alias, yes }) => {
            if store.find(&alias).is_none() {
                bail!(StoreError::Unknown(alias));
            }
            if !yes {
                if !std::io::stdin().is_terminal() {
                    bail!(fl!(i18n::LOADER, "error-confirm-needs-tty"));
                }
                if !confirm(&fl!(i18n::LOADER, "confirm-remove", alias = alias.as_str()))? {
                    return Ok(());
                }
            }
            store.remove(&alias)?;
            store.save()?;
            println!("{}", fl!(i18n::LOADER, "removed", alias = alias.as_str()));
            Ok(())
        }
    }
}
