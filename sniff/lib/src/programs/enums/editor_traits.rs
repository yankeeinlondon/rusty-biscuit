//! Editor facts a caller needs before launching one: where it draws, how to
//! make it block until the file is closed, and how to recognize it from a
//! configured command such as `$EDITOR`.
//!
//! These facts live in [`EDITOR_TRAITS`], a table beside `EDITOR_INFO`
//! indexed the same way (`*editor as usize`), rather than in `ProgramInfo`:
//! `ProgramInfo` is shared by every program category and is serialized, so
//! editor-only facts there would change every category's shape.

use std::path::Path;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount, IntoEnumIterator};

use super::categories::Editor;
use crate::programs::schema::ProgramMetadata;

/// Where an editor draws its interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EditorSurface {
    /// Runs inside the terminal it was started from (Vim, Nano, Helix).
    Terminal,
    /// Opens its own window and needs a display session (VS Code, Zed).
    Gui,
    /// Either, depending on how it is started and whether a display exists
    /// (Emacs).
    Both,
}

/// The launch facts for one [`Editor`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorTraits {
    /// Where the editor draws.
    pub surface: EditorSurface,
    /// Arguments that make the command block until the file is closed. Empty
    /// for a terminal editor (it blocks by nature) and for a GUI editor with
    /// no verified blocking flag (it cannot be waited on).
    pub wait_args: &'static [&'static str],
    /// Extra command names that identify this editor without being probed
    /// for during discovery (`code-insiders` is VS Code, `helix` is Helix).
    pub identify_also: &'static [&'static str],
}

const fn traits(
    surface: EditorSurface,
    wait_args: &'static [&'static str],
    identify_also: &'static [&'static str],
) -> EditorTraits {
    EditorTraits { surface, wait_args, identify_also }
}

const TERMINAL: EditorTraits = traits(EditorSurface::Terminal, &[], &[]);
const GUI_WAIT: EditorTraits = traits(EditorSurface::Gui, &["--wait"], &[]);
const GUI_NO_WAIT: EditorTraits = traits(EditorSurface::Gui, &[], &[]);
const BOTH: EditorTraits = traits(EditorSurface::Both, &[], &[]);

/// Launch facts for every [`Editor`] variant, indexed by `*editor as usize`.
///
/// Wait flags are each vendor's documented "wait for the file to close"
/// option. Lapce and Geany have no verified blocking flag and are therefore
/// recorded as non-blocking GUI editors.
pub(crate) static EDITOR_TRAITS: [EditorTraits; Editor::COUNT] = [
    TERMINAL,                                                   // Vi
    TERMINAL,                                                   // Vim
    TERMINAL,                                                   // Neovim
    BOTH,                                                       // Emacs
    BOTH,                                                       // XEmacs
    TERMINAL,                                                   // Nano
    traits(EditorSurface::Terminal, &[], &["helix"]),           // Helix
    traits(EditorSurface::Gui, &["--wait"], &["code-insiders"]), // VSCode
    GUI_WAIT,                                                   // VSCodium
    GUI_WAIT,                                                   // Sublime
    GUI_WAIT,                                                   // Zed
    TERMINAL,                                                   // Micro
    TERMINAL,                                                   // Kakoune
    TERMINAL,                                                   // Amp
    GUI_NO_WAIT,                                                // Lapce
    GUI_WAIT,                                                   // PhpStorm
    GUI_WAIT,                                                   // IntellijIdea
    GUI_WAIT,                                                   // PyCharm
    GUI_WAIT,                                                   // WebStorm
    GUI_WAIT,                                                   // CLion
    GUI_WAIT,                                                   // GoLand
    GUI_WAIT,                                                   // Rider
    GUI_WAIT,                                                   // TextMate
    GUI_WAIT,                                                   // BBEdit
    GUI_NO_WAIT,                                                // Geany
    traits(EditorSurface::Gui, &["--block"], &[]),              // Kate
];

/// Launcher extensions Windows adds to a command name; `code.cmd` and
/// `Code.exe` are both VS Code.
const LAUNCHER_EXTENSIONS: &[&str] = &["exe", "cmd", "bat", "com"];

impl Editor {
    /// The launch facts for this editor.
    pub fn traits(&self) -> &'static EditorTraits {
        &EDITOR_TRAITS[*self as usize]
    }

    /// Where this editor draws.
    pub fn surface(&self) -> EditorSurface {
        self.traits().surface
    }

    /// Arguments that make this editor block until the file is closed.
    pub fn wait_args(&self) -> &'static [&'static str] {
        self.traits().wait_args
    }

    /// Whether a caller can wait for this editor to close the file.
    ///
    /// A terminal or dual-surface editor blocks by nature; a GUI editor
    /// blocks only through its [wait arguments](Self::wait_args).
    pub fn can_block(&self) -> bool {
        let traits = self.traits();
        traits.surface != EditorSurface::Gui || !traits.wait_args.is_empty()
    }

    /// The editor a program names, recognized by its command name.
    ///
    /// `program` is one program path or name, such as the program token of
    /// `$EDITOR` (`/usr/local/bin/code`, `nvim`, `C:\Tools\code.cmd`). Only
    /// its final component is compared, so a directory containing spaces is
    /// fine; a whole command line (`code --wait`) is not split here. A
    /// trailing `.exe`, `.cmd`, `.bat`, or `.com` is removed before comparing,
    /// and no other extension is (`nvim.appimage` stays as written). The name
    /// is compared with each editor's binary name, alternate binary names,
    /// and extra identifying names: case-insensitively on Windows, exactly
    /// elsewhere.
    ///
    /// ## Returns
    ///
    /// `None` for a name no editor uses, such as a wrapper script; a caller
    /// treats that as an unknown command, not as "no editor".
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::programs::Editor;
    ///
    /// assert_eq!(Editor::identify("/usr/local/bin/code"), Some(Editor::VSCode));
    /// assert_eq!(Editor::identify("code.cmd"), Some(Editor::VSCode));
    /// assert_eq!(Editor::identify("my-editor-wrapper"), None);
    /// ```
    pub fn identify(program: &str) -> Option<Editor> {
        let file_name = Path::new(program.trim()).file_name()?.to_str()?;
        let name = match file_name.rsplit_once('.') {
            Some((stem, extension))
                if !stem.is_empty()
                    && LAUNCHER_EXTENSIONS
                        .iter()
                        .any(|launcher| launcher.eq_ignore_ascii_case(extension)) =>
            {
                stem
            }
            _ => file_name,
        };
        let matches = |candidate: &str| {
            if cfg!(windows) {
                candidate.eq_ignore_ascii_case(name)
            } else {
                candidate == name
            }
        };
        Editor::iter().find(|editor| {
            matches(editor.binary_name())
                || editor.alternate_binary_names().iter().any(|alt| matches(alt))
                || editor.traits().identify_also.iter().any(|alt| matches(alt))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits_table_covers_every_editor() {
        assert_eq!(EDITOR_TRAITS.len(), Editor::COUNT);
    }

    /// The expected surface and wait arguments, spelled per variant with no
    /// wildcard so a new `Editor` variant cannot compile without them.
    fn expected(editor: Editor) -> (EditorSurface, &'static [&'static str]) {
        use EditorSurface::{Both, Gui, Terminal};
        match editor {
            Editor::Vi
            | Editor::Vim
            | Editor::Neovim
            | Editor::Nano
            | Editor::Helix
            | Editor::Micro
            | Editor::Kakoune
            | Editor::Amp => (Terminal, &[]),
            Editor::Emacs | Editor::XEmacs => (Both, &[]),
            Editor::VSCode
            | Editor::VSCodium
            | Editor::Sublime
            | Editor::Zed
            | Editor::TextMate
            | Editor::BBEdit
            | Editor::PhpStorm
            | Editor::IntellijIdea
            | Editor::PyCharm
            | Editor::WebStorm
            | Editor::CLion
            | Editor::GoLand
            | Editor::Rider => (Gui, &["--wait"]),
            Editor::Kate => (Gui, &["--block"]),
            Editor::Lapce | Editor::Geany => (Gui, &[]),
        }
    }

    #[test]
    fn every_editor_has_its_documented_surface_and_wait_args() {
        for editor in Editor::iter() {
            let (surface, wait_args) = expected(editor);
            assert_eq!(editor.surface(), surface, "{editor:?} surface");
            assert_eq!(editor.wait_args(), wait_args, "{editor:?} wait args");
        }
    }

    #[test]
    fn non_blocking_gui_editors_cannot_block() {
        assert!(!Editor::Lapce.can_block());
        assert!(!Editor::Geany.can_block());
    }

    #[test]
    fn terminal_dual_and_waiting_gui_editors_can_block() {
        assert!(Editor::Emacs.can_block());
        assert!(Editor::XEmacs.can_block());
        assert!(Editor::Vim.can_block());
        assert!(Editor::VSCode.can_block());
        assert!(Editor::Kate.can_block());
        let blocking = Editor::iter().filter(Editor::can_block).count();
        assert_eq!(blocking, Editor::COUNT - 2, "only Lapce and Geany are non-blocking");
    }

    #[test]
    fn identify_reads_the_final_path_component() {
        assert_eq!(Editor::identify("/usr/local/bin/code"), Some(Editor::VSCode));
        assert_eq!(Editor::identify("code"), Some(Editor::VSCode));
        assert_eq!(Editor::identify("/opt/My Editors/bin/subl"), Some(Editor::Sublime));
        assert_eq!(Editor::identify("  nvim  "), Some(Editor::Neovim));
    }

    #[test]
    fn identify_strips_windows_launcher_extensions_only() {
        assert_eq!(Editor::identify("code.cmd"), Some(Editor::VSCode));
        assert_eq!(Editor::identify("code.CMD"), Some(Editor::VSCode));
        assert_eq!(Editor::identify("subl.exe"), Some(Editor::Sublime));
        assert_eq!(Editor::identify("hx.bat"), Some(Editor::Helix));
        assert_eq!(Editor::identify("zed.com"), Some(Editor::Zed));
        assert_eq!(Editor::identify("nvim.appimage"), None);
        assert_eq!(Editor::identify(".exe"), None);
    }

    #[test]
    fn identify_uses_extra_identifying_names() {
        assert_eq!(Editor::identify("code-insiders"), Some(Editor::VSCode));
        assert_eq!(Editor::identify("helix"), Some(Editor::Helix));
        assert_eq!(Editor::identify("hx"), Some(Editor::Helix));
    }

    #[test]
    fn identify_compares_case_by_platform() {
        if cfg!(windows) {
            assert_eq!(Editor::identify("Code.exe"), Some(Editor::VSCode));
            assert_eq!(Editor::identify("NVIM"), Some(Editor::Neovim));
        } else {
            assert_eq!(Editor::identify("Code.exe"), None);
            assert_eq!(Editor::identify("NVIM"), None);
        }
    }

    #[test]
    fn identify_does_not_recognize_unknown_commands() {
        assert_eq!(Editor::identify("/home/me/bin/edit-wrapper.sh"), None);
        assert_eq!(Editor::identify("my-editor"), None);
        assert_eq!(Editor::identify(""), None);
        assert_eq!(Editor::identify("code --wait"), None, "a command line is not split");
    }

    #[test]
    fn editor_surface_serializes_in_snake_case() {
        assert_eq!(serde_json::to_string(&EditorSurface::Gui).unwrap(), "\"gui\"");
        assert_eq!(serde_json::to_string(&EditorSurface::Both).unwrap(), "\"both\"");
        assert_eq!(EditorSurface::Terminal.to_string(), "terminal");
    }
}
