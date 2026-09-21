use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use i18n_embed_fl::fl;

use crate::model::{AgeView, SortKey};

#[derive(Parser, Debug)]
#[command(name = "ages", version, disable_help_subcommand = true)]
pub struct Cli {
    /// UI language.
    #[arg(long, global = true, value_parser = ["en", "tr"])]
    pub lang: Option<String>,
    /// Data directory (default: ~/.ages, or $AGES_HOME).
    #[arg(long, global = true, value_name = "PATH")]
    pub data_dir: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    List {
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum)]
        sort: Option<SortArg>,
        #[arg(long, value_enum)]
        view: Option<ViewArg>,
    },
    Add {
        alias: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        surname: Option<String>,
        #[arg(long)]
        birth: String,
        #[arg(long)]
        time: Option<String>,
        #[arg(long)]
        tz: Option<String>,
        #[arg(long, value_name = "IMAGE")]
        avatar: Option<PathBuf>,
    },
    Edit {
        alias: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        surname: Option<String>,
        #[arg(long)]
        birth: Option<String>,
        #[arg(long)]
        time: Option<String>,
        #[arg(long)]
        tz: Option<String>,
        #[arg(long, value_name = "IMAGE")]
        avatar: Option<PathBuf>,
        #[arg(long, conflicts_with = "avatar")]
        no_avatar: bool,
        #[arg(long, value_name = "ALIAS")]
        rename: Option<String>,
    },
    Remove {
        alias: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
    Tui,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SortArg {
    Age,
    Alias,
    Birthday,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ViewArg {
    Calendar,
    Years,
    Months,
    Weeks,
    Days,
    Hours,
}

impl From<ViewArg> for AgeView {
    fn from(v: ViewArg) -> Self {
        match v {
            ViewArg::Calendar => AgeView::Calendar,
            ViewArg::Years => AgeView::Years,
            ViewArg::Months => AgeView::Months,
            ViewArg::Weeks => AgeView::Weeks,
            ViewArg::Days => AgeView::Days,
            ViewArg::Hours => AgeView::Hours,
        }
    }
}

impl From<SortArg> for SortKey {
    fn from(s: SortArg) -> Self {
        match s {
            SortArg::Age => SortKey::Age,
            SortArg::Alias => SortKey::Alias,
            SortArg::Birthday => SortKey::Birthday,
        }
    }
}

/// `Cli::command()` with about texts taken from the active Fluent language.
pub fn localized_command() -> clap::Command {
    let l = &crate::i18n::LOADER;
    Cli::command()
        .about(fl!(l, "app-about"))
        .mut_subcommand("list", |c| c.about(fl!(l, "cmd-list")))
        .mut_subcommand("add", |c| c.about(fl!(l, "cmd-add")))
        .mut_subcommand("edit", |c| c.about(fl!(l, "cmd-edit")))
        .mut_subcommand("remove", |c| c.about(fl!(l, "cmd-remove")))
        .mut_subcommand("tui", |c| c.about(fl!(l, "cmd-tui")))
}

/// Find `--lang X` / `--lang=X` before clap runs, so help text is localized.
pub fn prescan_lang(args: &[String]) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--lang" {
            return it.next().cloned();
        }
        if let Some(v) = a.strip_prefix("--lang=") {
            return Some(v.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prescan_finds_lang() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            prescan_lang(&v(&["ages", "--lang", "tr", "list"])),
            Some("tr".into())
        );
        assert_eq!(
            prescan_lang(&v(&["ages", "list", "--lang=en"])),
            Some("en".into())
        );
        assert_eq!(prescan_lang(&v(&["ages", "list"])), None);
    }

    #[test]
    fn command_is_valid() {
        localized_command().debug_assert();
    }
}
