//! Interface languages (Preferences ▸ Language; the reference is localized into the eleven languages its manual
//! lists). Text is drawn through [`tr`], keyed by the English text, so the English strings stay the identifiers the
//! code compares; anything without a translation shows in English.

use std::sync::atomic::{AtomicUsize, Ordering};

mod tables;

/// The languages, by the names the Language menu shows (each in its own language), English first.
pub const LANGUAGES: &[&str] = &[
    "English",
    "简体中文",
    "繁體中文",
    "Deutsch",
    "Español",
    "Français",
    "Italiano",
    "日本語",
    "한국어",
    "Português (Brasil)",
    "Русский",
    "Slovenčina",
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

fn table(i: usize) -> &'static [(&'static str, &'static str)] {
    match i {
        1 => tables::ZH_HANS,
        2 => tables::ZH_HANT,
        3 => tables::DE,
        4 => tables::ES,
        5 => tables::FR,
        6 => tables::IT,
        7 => tables::JA,
        8 => tables::KO,
        9 => tables::PT_BR,
        10 => tables::RU,
        11 => tables::SK,
        _ => &[],
    }
}

/// Switches the interface to a language by its [`LANGUAGES`] name (or its English name, e.g. "German"); unknown
/// names leave it as it was. Returns whether the name was known.
pub fn set(name: &str) -> bool {
    const ENGLISH_NAMES: &[&str] = &[
        "english",
        "chinese (simplified)",
        "chinese (traditional)",
        "german",
        "spanish",
        "french",
        "italian",
        "japanese",
        "korean",
        "portuguese (brazil)",
        "russian",
        "slovak",
    ];
    let lower = name.trim().to_lowercase();
    let found = LANGUAGES.iter().position(|l| l.to_lowercase() == lower).or_else(|| ENGLISH_NAMES.iter().position(|l| *l == lower));
    if let Some(i) = found {
        CURRENT.store(i, Ordering::Relaxed);
    }
    found.is_some()
}

/// The current language's [`LANGUAGES`] name.
pub fn current() -> &'static str {
    LANGUAGES.get(CURRENT.load(Ordering::Relaxed)).copied().unwrap_or("English")
}

/// [`tr`] for text that comes as a `String` or `&String` (a label picked at run time, a message with a name or a number
/// in it, see [`filled`]); the result is owned.
pub fn tr_s(english: impl AsRef<str>) -> String {
    let english = english.as_ref();
    let current = CURRENT.load(Ordering::Relaxed);
    let t = table(current);
    if t.is_empty() {
        return english.to_owned();
    }
    if let Ok(i) = t.binary_search_by(|(k, _)| (*k).cmp(english)) {
        return t[i].1.to_owned();
    }
    thread_local! {
        /// What the messages shown lately came to (looked up again every frame they show).
        static MADE: std::cell::RefCell<std::collections::HashMap<(usize, String), Option<String>>> = Default::default();
    }
    MADE.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 4000 {
            m.clear();
        }
        m.entry((current, english.to_owned())).or_insert_with(|| filled(t, english)).clone().unwrap_or_else(|| english.to_owned())
    })
}

/// A message made around a name or a number ("Couldn't save: …"), by the table's entry with `{}` in their place (`{#}`:
/// a number only). The translation takes them in the same order, or by place: `{0}`, `{1}`.
fn filled(t: &[(&str, &str)], english: &str) -> Option<String> {
    let mut best: Option<(usize, &str, Vec<&str>)> = None;
    for (key, to) in t.iter().filter(|(k, _)| k.contains("{}") || k.contains("{#}")) {
        let weight = key.len();
        if best.as_ref().is_none_or(|(w, ..)| weight > *w)
            && let Some(found) = captures(key, english)
        {
            best = Some((weight, to, found));
        }
    }
    let (_, to, found) = best?;
    let mut out = String::with_capacity(to.len() + english.len());
    let (mut rest, mut next) = (to, 0);
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let Some(end) = rest[i..].find('}') else { break };
        let inside = &rest[i + 1..i + end];
        let k = match inside {
            "" | "#" => {
                next += 1;
                Some(next - 1)
            }
            n => n.parse::<usize>().ok(),
        };
        // What fills it may be a name of ours itself (a tool's, a word's).
        match k.and_then(|k| found.get(k)) {
            Some(f) => out.push_str(t.binary_search_by(|(k, _)| (*k).cmp(f)).map_or(f, |j| t[j].1)),
            None => out.push_str(&rest[i..i + end + 1]),
        }
        rest = &rest[i + end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// What fills a key's places in `text`, if the text is that key filled in.
fn captures<'a>(key: &str, text: &'a str) -> Option<Vec<&'a str>> {
    let mut found = Vec::new();
    let (mut key, mut text) = (key, text);
    loop {
        let at = [key.find("{}").map(|i| (i, 2, false)), key.find("{#}").map(|i| (i, 3, true))].into_iter().flatten().min();
        let Some((i, len, number)) = at else {
            return (key == text).then_some(found);
        };
        text = text.strip_prefix(&key[..i])?;
        key = &key[i + len..];
        // Up to the words that follow (or the end).
        let next = [key.find("{}"), key.find("{#}")].into_iter().flatten().min().unwrap_or(key.len());
        let words = &key[..next];
        let end = if words.is_empty() || next == key.len() {
            text.len().checked_sub(words.len()).filter(|e| text.is_char_boundary(*e) && &text[*e..] == words)?
        } else {
            text.find(words)?
        };
        let fill = &text[..end];
        if fill.is_empty() || number && !fill.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-')) {
            return None;
        }
        found.push(fill);
        text = &text[end..];
    }
}

/// `english` in the current language (as it is when there's no translation).
pub fn tr(english: &str) -> &str {
    let t = table(CURRENT.load(Ordering::Relaxed));
    match t.binary_search_by(|(k, _)| (*k).cmp(english)) {
        Ok(i) => t[i].1,
        Err(_) => english,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_is_sorted_and_has_the_same_keys() {
        let de = table(3);
        assert!(!de.is_empty());
        for i in 1..LANGUAGES.len() {
            let t = table(i);
            assert!(t.windows(2).all(|w| w[0].0 < w[1].0), "{} isn't sorted", LANGUAGES[i]);
            assert_eq!(t.iter().map(|p| p.0).collect::<Vec<_>>(), de.iter().map(|p| p.0).collect::<Vec<_>>(), "{}", LANGUAGES[i]);
            assert!(t.iter().all(|p| !p.1.trim().is_empty()), "{} has an empty translation", LANGUAGES[i]);
        }
    }

    #[test]
    fn messages_with_a_name_or_a_number_translate_around_it() {
        assert_eq!(captures("{#} edges", "12 edges"), Some(vec!["12"]));
        assert_eq!(captures("{#} edges", "Soft edges"), None);
        assert_eq!(captures("Couldn't save: {}", "Couldn't save: disk full"), Some(vec!["disk full"]));
        assert_eq!(captures("Opened {} in {}", "Opened Cup in Cura"), Some(vec!["Cup", "Cura"]));
        assert_eq!(captures("Opened {} in {}", "Opened in Cura"), None);
        assert_eq!(captures("{} is ready", "é is ready"), Some(vec!["é"]));
        let t: &[(&str, &str)] = &[
            ("Fillet", "Verrundung"),
            ("Opened {} in {}", "{1}: {0} offen"),
            ("{#} edges", "{#} Kanten"),
            ("{} canceled", "{} abgebrochen"),
        ];
        assert_eq!(filled(t, "Opened Cup in Cura").as_deref(), Some("Cura: Cup offen"));
        assert_eq!(filled(t, "3 edges").as_deref(), Some("3 Kanten"));
        assert_eq!(filled(t, "Fillet canceled").as_deref(), Some("Verrundung abgebrochen"));
        assert_eq!(filled(t, "My project"), None);
        // Every translation keeps the places its English has.
        for (i, lang) in LANGUAGES.iter().enumerate().skip(1) {
            for (k, v) in table(i) {
                let n = k.matches("{}").count() + k.matches("{#}").count();
                let m = v.matches("{}").count() + v.matches("{#}").count() + (0..4).filter(|d| v.contains(&format!("{{{d}}}"))).count();
                assert_eq!(n, m, "{lang}: {k} → {v}");
            }
        }
    }

    #[test]
    fn switching_translates_and_falls_back_to_english() {
        assert!(set("Deutsch"));
        assert_ne!(tr("Sketch"), "Sketch");
        assert_eq!(tr("no such text"), "no such text");
        assert!(set("german") && current() == "Deutsch");
        assert!(!set("Klingon") && current() == "Deutsch");
        assert!(set("English"));
        assert_eq!(tr("Sketch"), "Sketch");
    }
}
