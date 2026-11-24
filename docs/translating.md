# Translating ShapeCraft

No programming needed. Each language is one file in [`crates/ui-egui/lang`](../crates/ui-egui/lang):

| Language | File |
|---|---|
| Chinese (Simplified) | `zh_Hans.po` |
| Chinese (Traditional) | `zh_Hant.po` |
| French | `fr.po` |
| German | `de.po` |
| Italian | `it.po` |
| Japanese | `ja.po` |
| Korean | `ko.po` |
| Portuguese (Brazil) | `pt_BR.po` |
| Russian | `ru.po` |
| Slovak | `sk.po` |
| Spanish | `es.po` |

Every language has all its texts, about 1,800. Most haven't been read by a native speaker yet, so what helps most is
someone reading their own language and fixing what sounds wrong.

## Fix a word or a sentence

1. Open the file on GitHub and press the pencil (Edit this file).
2. Find the text and change its `msgstr`:

   ```
   #: crates/ui-egui/src/workspace/panels/popups.rs
   msgid "Offset Face"
   msgstr "Fläche versetzen"
   ```

3. Press **Commit changes…** and **Propose changes**, then **Create pull request**.

That's all. The checks run by themselves, and you're credited in the [CHANGELOG](../CHANGELOG.md).

## Work through a whole language

The files are gettext `.po` files, so [Poedit](https://poedit.net) (free), Lokalize, Weblate and other translation tools
open them. In Poedit: **Open** your language's file, translate, save, and send the saved file as a pull request. Don't want
to use Git? Attach the file to an [issue](https://github.com/jackharvest/shapecraft/issues/new/choose).

Small pull requests are fine, and so is a hundred texts at a time.

## Rules

- **Change only `msgstr`.** The `msgid` is the English the app looks the text up by.
- **`{}` is something the app fills in**: a name, a tool, a size. `{#}` is always a number. Keep every one. They're filled in
  the order they come; to change the order, number them from zero:

  ```
  msgid "Opened {} in {}"
  msgstr "{1}: {0} offen"
  ```

- **Keep spaces at the start and the end**, `\n` (a new line), and signs such as `▸` and `…`.
- **An empty `msgstr` shows the English**, and so does an entry marked `fuzzy`. If you aren't sure, leave it empty.
- Names stay as they are: ShapeCraft, STEP, 3MF, Blender, file endings.
- The line starting with `#:` says where the text comes from, which often tells you what it's about.

## Words to keep the same

Pick one word for each of these and use it everywhere: body, face, edge, sketch, plane, axis, extrude, revolve, loft, sweep,
fillet, chamfer, shell, pattern, union, subtract, intersect. If your language has usual CAD words, use those.

How the app speaks to people: German uses "du", Italian "tu"; French, Portuguese and Russian are formal. If that sounds wrong
in your language, open an issue first so the whole file changes together.

## See it in the app

Preferences ▸ Language switches at once. To see your own changes you need a build from source
([README](../README.md)); otherwise they come with the next release.

## Add a language

Copy `shapecraft.pot` to a file named for the language (`nl.po`, `pl.po`…), fill in what you can and send it. Anything still
empty shows in English, so it doesn't have to be finished. We hook it up.

## For developers

Draw text through `i18n::tr` / `tr_s` with the English text. A new text goes into `lang/shapecraft.pot` and, if you know it,
into the language files; `cargo test -p shapecraft-ui i18n` checks that no file has a text the app doesn't know and that
every `{}` is kept.
