mod age;
mod avatar;
mod cli;
mod commands;
mod dates;
mod i18n;
mod model;
mod output;
mod store;
mod tui;

use clap::FromArgMatches;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = cli::prescan_lang(&args);
    i18n::init(flag.as_deref(), None);
    let matches = cli::localized_command().get_matches_from(&args);
    let parsed = match cli::Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => e.exit(),
    };
    if let Err(e) = commands::run(parsed) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
