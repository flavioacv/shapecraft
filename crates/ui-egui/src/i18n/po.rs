//! Reads the translations (`crates/ui-egui/lang/*.po`, gettext's format, so translators can use their usual tools).
//! They are part of the app, but people edit them by hand: an entry that doesn't read is left out, and its text shows
//! in English.

/// A table's entries: English and translation, sorted by the English (byte order). Untranslated (empty) and `fuzzy`
/// ones are left out.
pub fn read(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (mut id, mut to) = (None::<String>, None::<String>);
    let (mut fuzzy, mut next_fuzzy) = (false, false);
    let mut keep = |id: &mut Option<String>, to: &mut Option<String>, fuzzy: bool| {
        if let (Some(id), Some(to)) = (id.take(), to.take())
            && !id.is_empty()
            && !to.trim().is_empty()
            && !fuzzy
        {
            out.push((id, to));
        }
    };
    for line in text.lines().map(str::trim) {
        if let Some(flags) = line.strip_prefix("#,") {
            next_fuzzy |= flags.split(',').any(|f| f.trim() == "fuzzy");
        } else if let Some(s) = line.strip_prefix("msgid ") {
            keep(&mut id, &mut to, fuzzy);
            fuzzy = std::mem::take(&mut next_fuzzy);
            (id, to) = (quoted(s), None);
        } else if let Some(s) = line.strip_prefix("msgstr ") {
            to = quoted(s);
        } else if line.starts_with('"')
            && let Some(more) = quoted(line)
            && let Some(s) = to.as_mut().or(id.as_mut())
        {
            // A text can go on over several lines.
            s.push_str(&more);
        }
    }
    keep(&mut id, &mut to, fuzzy);
    // A text listed twice counts as its first entry.
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|b, a| a.0 == b.0);
    out
}

/// The text inside the quotes; `None` when they aren't there.
fn quoted(s: &str) -> Option<String> {
    let s = s.trim().strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_translation_tools_write() {
        let t = read(
            "msgid \"\"\nmsgstr \"Language: de\\n\"\n\n#: a.rs\nmsgid \"Sketch\"\nmsgstr \"Skizze\"\n\n#, fuzzy\nmsgid \"Old\"\nmsgstr \"Alt\"\n\n\
             msgid \"\"\n\"Say \\\"hi\\\"\\n\"\n\"twice\"\nmsgstr \"\"\n\"Sag \\\"hallo\\\"\\n\"\n\"zweimal\"\n\nmsgid \"Later\"\nmsgstr \"\"\n\nmsgid \"Body\"\r\nmsgstr \"Körper\"\r\n\
             msgid broken\nmsgstr \"x\"\nmsgid \"Body\"\nmsgstr \"again\"\n",
        );
        let t: Vec<_> = t.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        assert_eq!(t, [("Body", "Körper"), ("Say \"hi\"\ntwice", "Sag \"hallo\"\nzweimal"), ("Sketch", "Skizze")]);
    }
}
