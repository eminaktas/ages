use i18n_embed::{
    DesktopLanguageRequester, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;
use std::sync::LazyLock;
use unic_langid::{LanguageIdentifier, langid};

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    loader.set_use_isolating(false);
    loader
        .load_languages(&Localizations, &[loader.fallback_language().clone()])
        .expect("embedded en translations");
    loader
});

pub const SUPPORTED: [&str; 2] = ["en", "tr"];

fn to_langid(code: &str) -> LanguageIdentifier {
    match code {
        "tr" => langid!("tr"),
        _ => langid!("en"),
    }
}

/// Switch the active language. Unknown codes fall back to English.
pub fn select(code: &str) {
    LOADER
        .load_languages(&Localizations, &[to_langid(code)])
        .expect("embedded translations");
    // Bundles are rebuilt on load; re-disable Unicode isolation marks around placeables.
    LOADER.set_use_isolating(false);
}

pub fn current() -> String {
    LOADER.current_language().language.as_str().to_string()
}

/// Resolve precedence: flag > desktop locale > en. (The stored setting is applied
/// later by `commands::run`, once the store is open.)
pub fn init(flag: Option<&str>) {
    let code = flag
        .map(str::to_string)
        .or_else(|| {
            DesktopLanguageRequester::requested_languages()
                .into_iter()
                .map(|l| l.language.as_str().to_string())
                .find(|l| SUPPORTED.contains(&l.as_str()))
        })
        .unwrap_or_else(|| "en".to_string());
    select(&code);
}

/// Tests that read or change the global language must hold this guard (English is selected on acquire).
#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    select("en");
    guard
}

#[cfg(test)]
mod tests {
    use super::*;
    use i18n_embed_fl::fl;

    #[test]
    fn switches_language() {
        let _g = test_lock();
        select("tr");
        assert_eq!(fl!(LOADER, "birthday-today"), "doğum günü bugün");
        select("en");
        assert_eq!(fl!(LOADER, "birthday-today"), "birthday today");
    }
}
