//! The shell side of `md`'s Tab completion.
//!
//! `clap_complete`'s built-in shell scripts assume a candidate is a whole word
//! that ends with a space. A property path breaks both assumptions:
//!
//! - Bash splits a word at `=` and `:`, and replaces only the last piece, so
//!   `a.b=ce` arrives in pieces and must be answered with `center`.
//! - `style.` and `title=` continue the same word, so no space may follow them.
//!
//! [`SHELLS`] replaces the bash and zsh adapters and keeps the built-ins for fish,
//! elvish, and PowerShell. The registration scripts are the built-in ones from
//! `clap_complete` 4.6.2 with small edits, and that output has no stability
//! promise; `tests/l1/path_completion.rs` fails if an edit stops applying.

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;

use clap_complete::env::{Bash, Elvish, EnvCompleter, Fish, Powershell, Shells, Zsh};

/// The shells `md` registers with.
pub const SHELLS: Shells<'static> = Shells(&[&MdBash, &Elvish, &Fish, &Powershell, &MdZsh]);

/// The line being completed, as the words `md` itself would receive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredWords {
    /// Every word on the line, starting with the program name.
    pub words: Vec<String>,
    /// The position in `words` of the word under the cursor.
    pub index: usize,
}

impl RecoveredWords {
    /// The words before the cursor; words after it never affect completion.
    #[must_use]
    pub fn before_cursor(&self) -> &[String] {
        &self.words[..self.index.min(self.words.len())]
    }
}

/// How the active shell spells the words it hands over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordForm {
    /// Already what the program would receive: fish, elvish, and PowerShell.
    AsGiven,
    /// Raw source text, with its quotes and backslashes: zsh.
    Raw,
    /// Raw source text, and split at `=` and `:` from bash 4 on.
    BashPieces,
}

/// Set by the active adapter. `CompleteEnv` removes its variable before any
/// completer runs, so the variable cannot say which shell this is.
static WORD_FORM: OnceLock<WordForm> = OnceLock::new();

/// The line being completed, recovered from the process arguments.
///
/// This is the one reader of those arguments during completion. The engine and
/// the value completers both go through it, so they see the same words.
#[must_use]
pub fn recover_words() -> RecoveredWords {
    let arguments: Vec<String> = std::env::args_os()
        .skip_while(|argument| argument != "--")
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    let index = std::env::var("_CLAP_COMPLETE_INDEX").ok().and_then(|index| index.parse().ok());
    recover(arguments, index, WORD_FORM.get().copied().unwrap_or(WordForm::AsGiven))
}

fn recover(mut words: Vec<String>, index: Option<usize>, form: WordForm) -> RecoveredWords {
    let mut index = index.unwrap_or_else(|| words.len().saturating_sub(1));
    if form == WordForm::BashPieces {
        (words, index) = rejoin_pieces(&words, index);
    }
    if form != WordForm::AsGiven {
        words = words.iter().map(|word| shell_unquote(word)).collect();
    }
    if index >= words.len() {
        // Zsh leaves an empty current word out.
        words.resize(index + 1, String::new());
    }
    RecoveredWords { words, index }
}

/// Rejoins the pieces bash splits a word into, and moves `index` with them.
///
/// `x=ce` arrives as `x`, `=`, `ce`. With the cursor right after `=` the current
/// word is `=` itself, and the older form of the line has a further empty word;
/// both mean `x=`. Bash 3.2 does not split, so its words pass through unchanged.
///
/// The words alone cannot tell `x= y` from `x=y`, so a value left empty on purpose
/// joins the next word. That only changes what an earlier setter holds, and the
/// result is advisory.
fn rejoin_pieces(pieces: &[String], index: usize) -> (Vec<String>, usize) {
    let is_break_run = |piece: &str| !piece.is_empty() && piece.chars().all(|character| matches!(character, '=' | ':'));
    let mut words: Vec<String> = Vec::with_capacity(pieces.len());
    let mut joined_index = index;
    let mut joins_next = false;
    for (position, piece) in pieces.iter().enumerate() {
        let is_break = is_break_run(piece);
        // The program name is never part of an argument.
        match words.last_mut() {
            Some(word) if (is_break || joins_next) && position > 1 => word.push_str(piece),
            _ => words.push(piece.clone()),
        }
        joins_next = is_break;
        if position == index {
            joined_index = words.len() - 1;
        }
    }
    if index >= pieces.len() {
        joined_index = words.len() + (index - pieces.len());
    }
    (words, joined_index)
}

/// What a word means once the shell has removed its quoting. An open quote is
/// fine, since the word is still being typed.
fn shell_unquote(word: &str) -> String {
    let mut unquoted = String::with_capacity(word.len());
    let mut quote = None;
    let mut characters = word.chars().peekable();
    while let Some(character) = characters.next() {
        match (quote, character) {
            (None, '"' | '\'') => quote = Some(character),
            (Some(open), _) if character == open => quote = None,
            (None, '\\') => unquoted.extend(characters.next()),
            (Some('"'), '\\') if matches!(characters.peek(), Some('"' | '\\' | '$' | '`')) => unquoted.extend(characters.next()),
            _ => unquoted.push(character),
        }
    }
    unquoted
}

/// `script` with each `(built-in text, replacement)` applied once.
fn edited_script(script: Vec<u8>, edits: &[(&str, &str)]) -> Vec<u8> {
    let mut text = String::from_utf8_lossy(&script).into_owned();
    for (built_in, replacement) in edits {
        text = text.replacen(built_in, replacement, 1);
    }
    text.into_bytes()
}

#[derive(Debug, Clone, Copy)]
struct MdBash;

impl EnvCompleter for MdBash {
    fn name(&self) -> &'static str {
        Bash.name()
    }

    fn is(&self, name: &str) -> bool {
        Bash.is(name)
    }

    fn write_registration(&self, var: &str, name: &str, bin: &str, completer: &str, buf: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut script = Vec::new();
        Bash.write_registration(var, name, bin, completer, &mut script)?;
        buf.write_all(&edited_script(
            script,
            &[
                // `$2` holds the text up to the cursor, except right after `=`,
                // where it is empty and using it would lose the `=`.
                (
                    r#"if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then"#,
                    r#"if [[ "${BASH_VERSINFO[0]}" -ge 4 && "${COMP_WORDS[COMP_CWORD]}" != [=:]* ]]; then"#,
                ),
                // Tells `md` whether the word being replaced has an open quote.
                (
                    "    COMPREPLY=( $( \\\n",
                    "    local _MD_PIECE=\"${COMP_WORDS[COMP_CWORD]}\"\n    _MD_PIECE=\"${_MD_PIECE##*[=:]}\"\n    COMPREPLY=( $( \\\n        _MD_COMPLETE_QUOTE=\"${_MD_PIECE:0:1}\" \\\n",
                ),
                // A Tab must never print a problem into the prompt.
                (r#""${words[@]}" \"#, r#""${words[@]}" 2>/dev/null \"#),
                // `style.` continues the word, as `title=` does.
                ("[=/:]$", "[=/:.]$"),
            ],
        ))
    }

    fn write_complete(
        &self,
        cmd: &mut clap::Command,
        _args: Vec<OsString>,
        current_dir: Option<&Path>,
        buf: &mut dyn Write,
    ) -> Result<(), std::io::Error> {
        let _ = WORD_FORM.set(WordForm::BashPieces);
        let recovered = recover_words();
        let typed = recovered.words.get(recovered.index).cloned().unwrap_or_default();
        let arguments = recovered.words.into_iter().map(OsString::from).collect();
        let completions = clap_complete::engine::complete(cmd, arguments, recovered.index, current_dir)?;

        let separator = std::env::var("_CLAP_IFS").unwrap_or_else(|_| "\n".to_string());
        // The registered script sets this `true` where `compopt` is missing (bash
        // 3.2): nothing can turn `nospace` off there, so a finished word carries
        // its own space.
        let adds_space = std::env::var("_CLAP_COMPLETE_SPACE").is_ok_and(|space| space == "true");
        let quoted = std::env::var("_MD_COMPLETE_QUOTE").is_ok_and(|quote| matches!(quote.as_str(), "\"" | "'"));
        for (position, candidate) in completions.iter().enumerate() {
            if position != 0 {
                write!(buf, "{separator}")?;
            }
            let value = candidate.get_value().to_string_lossy();
            write!(buf, "{}", bash_reply(&value, &typed, quoted, adds_space))?;
        }
        Ok(())
    }
}

/// What bash should put in place of the last piece of `typed`.
fn bash_reply(candidate: &str, typed: &str, quoted: bool, adds_space: bool) -> String {
    let piece_start = typed.rfind(['=', ':']).map_or(0, |position| position + 1);
    let piece = candidate.strip_prefix(&typed[..piece_start]).unwrap_or(candidate);
    let mut reply = if quoted { piece.to_string() } else { bash_escape(piece) };
    // Inside an open quote, readline closes the quote itself, and a space here
    // would land before the closing quote.
    if adds_space && !quoted && !reply.ends_with(['=', '/', ':', '.']) {
        reply.push(' ');
    }
    reply
}

/// Bash inserts a reply as source text, so a member with a space or `$` must be
/// escaped to arrive at `md` as one value.
fn bash_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_whitespace() || "\\\"'$`&|;<>()*?[]{}#~!".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[derive(Debug, Clone, Copy)]
struct MdZsh;

impl EnvCompleter for MdZsh {
    fn name(&self) -> &'static str {
        Zsh.name()
    }

    fn is(&self, name: &str) -> bool {
        Zsh.is(name)
    }

    fn write_registration(&self, var: &str, name: &str, bin: &str, completer: &str, buf: &mut dyn Write) -> Result<(), std::io::Error> {
        let mut script = Vec::new();
        Zsh.write_registration(var, name, bin, completer, &mut script)?;
        buf.write_all(&edited_script(
            script,
            &[
                ("local -a other=()", "local -a other=()\n        local -a continues=()"),
                (
                    "            else\n                other+=(\"$completion\")",
                    "            elif [[ \"$value\" == *[.=] ]]; then\n                continues+=(\"$completion\")\n            else\n                other+=(\"$completion\")",
                ),
                // `-V` keeps the order `md` chose; `continues` takes no space.
                (
                    "[[ -n $other ]] && _describe 'values' other",
                    "[[ -n $continues ]] && _describe -V 'values' continues -S ''\n        [[ -n $other ]] && _describe -V 'values' other",
                ),
            ],
        ))
    }

    fn write_complete(
        &self,
        cmd: &mut clap::Command,
        _args: Vec<OsString>,
        current_dir: Option<&Path>,
        buf: &mut dyn Write,
    ) -> Result<(), std::io::Error> {
        let _ = WORD_FORM.set(WordForm::Raw);
        let arguments = recover_words().words.into_iter().map(OsString::from).collect();
        Zsh.write_complete(cmd, arguments, current_dir, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &[&str]) -> Vec<String> {
        line.iter().map(ToString::to_string).collect()
    }

    fn recovered(line: &[&str], index: usize, form: WordForm) -> (Vec<String>, usize) {
        let recovered = recover(words(line), Some(index), form);
        (recovered.words, recovered.index)
    }

    #[test]
    fn bash_pieces_rejoin_mid_value() {
        assert_eq!(
            recovered(&["md", "compose", "a.md", "x", "=", "ce"], 5, WordForm::BashPieces),
            (words(&["md", "compose", "a.md", "x=ce"]), 3)
        );
    }

    #[test]
    fn the_cursor_right_after_equals_means_an_empty_value() {
        let expected = (words(&["md", "compose", "a.md", "x="]), 3);
        assert_eq!(recovered(&["md", "compose", "a.md", "x", "="], 4, WordForm::BashPieces), expected);
        assert_eq!(recovered(&["md", "compose", "a.md", "x", "=", ""], 5, WordForm::BashPieces), expected);
    }

    #[test]
    fn bash_3_passes_a_whole_word_through() {
        assert_eq!(
            recovered(&["md", "compose", "a.md", "x=ce"], 3, WordForm::BashPieces),
            (words(&["md", "compose", "a.md", "x=ce"]), 3)
        );
    }

    #[test]
    fn earlier_setters_rejoin_and_the_index_follows() {
        assert_eq!(
            recovered(&["md", "compose", "a", "=", "1", "b", "=", "2", "c", "d"], 8, WordForm::BashPieces),
            (words(&["md", "compose", "a=1", "b=2", "c", "d"]), 4),
        );
        assert_eq!(
            recovered(&["md", "compose", "a", "=", "1", "", ""], 5, WordForm::BashPieces).0,
            words(&["md", "compose", "a=1", "", ""]),
        );
    }

    #[test]
    fn every_break_in_a_value_rejoins() {
        assert_eq!(
            recovered(&["md", "x", "=", "a", ":", "b", "=", "c"], 7, WordForm::BashPieces),
            (words(&["md", "x=a:b=c"]), 1)
        );
        assert_eq!(recovered(&["md", "x", "==", "a"], 3, WordForm::BashPieces), (words(&["md", "x==a"]), 1));
    }

    #[test]
    fn the_program_name_is_never_joined() {
        assert_eq!(recovered(&["md", "=", "a"], 2, WordForm::BashPieces), (words(&["md", "=a"]), 1));
    }

    #[test]
    fn other_shells_keep_a_lone_equals_word() {
        // `md set doc.md a =` sets `a` to `=` in a shell that does not split.
        let line = ["md", "set", "doc.md", "a", "="];
        assert_eq!(recovered(&line, 4, WordForm::AsGiven), (words(&line), 4));
        assert_eq!(recovered(&line, 4, WordForm::Raw), (words(&line), 4));
    }

    #[test]
    fn no_index_means_the_last_word() {
        let recovered = recover(words(&["md", "get", "a.md", "ti"]), None, WordForm::AsGiven);
        assert_eq!(recovered.index, 3);
        assert_eq!(recovered.before_cursor(), words(&["md", "get", "a.md"]));
    }

    #[test]
    fn a_missing_current_word_is_empty() {
        assert_eq!(recovered(&["md", "get"], 2, WordForm::Raw), (words(&["md", "get", ""]), 2));
    }

    #[test]
    fn words_after_the_cursor_stay_out_of_the_earlier_words() {
        let line = recover(words(&["md", "compose", "a=", "later=1"]), Some(2), WordForm::AsGiven);
        assert_eq!(line.before_cursor(), words(&["md", "compose"]));
    }

    #[test]
    fn raw_words_lose_their_shell_quoting() {
        for (raw, unquoted) in [
            (r"a\ b", "a b"),
            (r"a\\.b", r"a\.b"),
            (r#""a b"#, "a b"),
            (r#""a b""#, "a b"),
            (r#"x="c"#, "x=c"),
            ("'a\\b'", r"a\b"),
            (r#""a\"b""#, r#"a"b"#),
            (r#""a\b""#, r"a\b"),
            ("plain", "plain"),
            ("", ""),
        ] {
            assert_eq!(shell_unquote(raw), unquoted, "{raw}");
        }
        assert_eq!(recovered(&["md", r"a\\.b"], 1, WordForm::Raw).0[1], r"a\.b");
        assert_eq!(recovered(&["md", r"a\\.b"], 1, WordForm::AsGiven).0[1], r"a\\.b");
    }

    #[test]
    fn a_reply_replaces_only_the_last_piece() {
        assert_eq!(bash_reply("x=center", "x=ce", false, false), "center");
        assert_eq!(bash_reply("x=center", "x=", false, false), "center");
        assert_eq!(bash_reply("x=a:b", "x=a:", false, false), "b");
        assert_eq!(bash_reply("x=a=b", "x=a=", false, false), "b");
        assert_eq!(bash_reply("style.page.", "style.pa", false, false), "style.page.");
        assert_eq!(bash_reply("--output=json", "--output=", false, false), "json");
    }

    #[test]
    fn a_reply_is_escaped_unless_a_quote_is_open() {
        assert_eq!(bash_reply("x=two words", "x=", false, false), r"two\ words");
        assert_eq!(bash_reply("x=two words", "x=", true, false), "two words");
        assert_eq!(bash_reply("x=$HOME", "x=", false, false), r"\$HOME");
        assert_eq!(bash_reply(r"a\.b", "a", false, false), r"a\\.b");
        assert_eq!(bash_reply(r#"x="null""#, "x=", false, false), r#"\"null\""#);
    }

    #[test]
    fn a_finished_reply_carries_its_own_space_when_bash_cannot_add_one() {
        assert_eq!(bash_reply("x=center", "x=", false, true), "center ");
        for continuing in ["style.", "title=", "docs/", "a:"] {
            assert_eq!(bash_reply(continuing, "", false, true), continuing);
        }
        assert_eq!(bash_reply("x=center", "x=", true, true), "center");
    }

    #[test]
    fn every_registration_edit_applies() {
        let registration = |shell: &dyn EnvCompleter| {
            let mut script = Vec::new();
            shell.write_registration("COMPLETE", "md", "md", "md", &mut script).unwrap();
            String::from_utf8(script).unwrap()
        };
        let bash = registration(&MdBash);
        for edited in [r#"!= [=:]* ]]"#, "_MD_COMPLETE_QUOTE=", "2>/dev/null", "[=/:.]$"] {
            assert!(bash.contains(edited), "`{edited}` in {bash}");
        }
        assert!(!bash.contains("[=/:]$"), "{bash}");
        let zsh = registration(&MdZsh);
        for edited in ["local -a continues=()", "*[.=]", "continues -S ''", "_describe -V 'values' other"] {
            assert!(zsh.contains(edited), "`{edited}` in {zsh}");
        }
    }

    #[test]
    fn the_shells_keep_their_names() {
        for name in ["bash", "elvish", "fish", "powershell", "zsh"] {
            assert!(SHELLS.0.iter().any(|shell| shell.is(name)), "{name}");
        }
    }
}
