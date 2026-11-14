//! Choosing the editor: `$EDITOR`, then `$VISUAL`, then the probe list,
//! keeping only editors a caller can wait for and, when asked, only one
//! surface.
//!
//! Every editor fact (surface, wait flags, recognizing a configured command)
//! comes from Sniff's [`Editor`] catalog; this module keeps no table of its
//! own.

use std::fmt;
use std::path::PathBuf;

use sniff::programs::{Editor, EditorSurface, ProgramMetadata, find_program};

use super::{DEFAULT_EDITOR_PRIORITY, EditorError};

/// Which surface a caller requires of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceRequest {
    /// Any editor that can block, in the ordinary precedence.
    Any,
    /// Only an editor Sniff knows draws in the terminal.
    Terminal,
    /// Only an editor Sniff knows opens its own window.
    Gui,
}

impl From<Option<bool>> for SurfaceRequest {
    /// The `edit` action's `gui` option: omitted is [`Any`](Self::Any).
    fn from(gui: Option<bool>) -> Self {
        match gui {
            None => Self::Any,
            Some(true) => Self::Gui,
            Some(false) => Self::Terminal,
        }
    }
}

impl fmt::Display for SurfaceRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Any => "any",
            Self::Terminal => "terminal",
            Self::Gui => "gui",
        })
    }
}

/// Why resolution passed over one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// A configured variable names a program that was not found.
    Missing,
    /// The editor returns before the file is closed (a GUI editor with no
    /// verified wait flag).
    CannotWait,
    /// The editor's surface is not the requested one; `None` is a command
    /// Sniff does not recognize, whose surface cannot be proven.
    SurfaceMismatch(Option<EditorSurface>),
    /// A probed editor is not installed.
    NotInstalled,
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("not found"),
            Self::CannotWait => f.write_str("cannot wait for the file to close"),
            Self::SurfaceMismatch(Some(surface)) => write!(f, "{surface} editor"),
            Self::SurfaceMismatch(None) => f.write_str("unrecognized command, surface unknown"),
            Self::NotInstalled => f.write_str("not installed"),
        }
    }
}

/// One editor chosen for launch, resolved once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEditor {
    /// The program to start, as found on `PATH` (or the configured path).
    pub program: PathBuf,
    /// Configured arguments, then each wait flag not already among them.
    pub args: Vec<String>,
    /// Where it draws; `None` for a command Sniff does not recognize, which
    /// is trusted to block and treated as a terminal editor.
    pub surface: Option<EditorSurface>,
    /// The command as the user configured it, or the probed binary name.
    pub display: String,
}

impl ResolvedEditor {
    /// Whether this editor needs the host's terminal: a terminal or
    /// dual-surface editor, or an unrecognized command.
    pub fn uses_terminal(&self) -> bool {
        self.surface != Some(EditorSurface::Gui)
    }

    /// The command line without the file path, for messages.
    pub(crate) fn full_command(&self) -> String {
        let program = self.display.split_whitespace().next().unwrap_or(&self.display);
        std::iter::once(program)
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// What resolution reads from the host, so the order and filtering can be
/// exercised without touching the process environment or `PATH`.
pub(crate) struct ResolutionInputs<'a> {
    /// `$EDITOR` and `$VISUAL`, in that order; `None` when unset.
    pub variables: [(&'static str, Option<String>); 2],
    /// Finds a configured program token.
    pub find: &'a dyn Fn(&str) -> Option<PathBuf>,
    /// Finds an installed probe; called only when the probes run.
    pub installed: &'a dyn Fn(Editor) -> Option<PathBuf>,
    /// The probe order.
    pub probes: &'a [Editor],
}

/// Chooses the editor to launch.
///
/// `$EDITOR`, then `$VISUAL`, then [`DEFAULT_EDITOR_PRIORITY`]. A blank
/// variable counts as unset. A candidate is accepted only when it can block:
/// a known GUI editor without a wait flag is passed over, and an unknown
/// command is trusted. With [`SurfaceRequest::Any`], a set variable that is
/// rejected stops resolution with its error rather than falling to the
/// probes, so a typo never silently becomes another editor. With an explicit
/// surface, only an editor of exactly that surface is accepted (a
/// dual-surface or unrecognized command never is), and the probes run even
/// when a variable was set.
///
/// ## Errors
///
/// - [`EditorError::ConfiguredEditorMissing`] / [`EditorError::ConfiguredEditorCannotWait`]:
///   under `Any`, for the first set variable, when none was accepted.
/// - [`EditorError::NoEditorFound`]: under `Any`, nothing set and no probe
///   accepted.
/// - [`EditorError::NoMatchingEditor`]: under an explicit surface, listing
///   every candidate checked and why it was passed over.
pub fn resolve_editor(request: SurfaceRequest) -> Result<ResolvedEditor, EditorError> {
    let detected = std::sync::OnceLock::new();
    let installed = |editor: Editor| {
        detected
            .get_or_init(sniff::programs::InstalledEditors::new)
            .path(editor)
    };
    let find = |program: &str| find_program(program);
    resolve_editor_from(
        request,
        &ResolutionInputs {
            variables: [("EDITOR", std::env::var("EDITOR").ok()), ("VISUAL", std::env::var("VISUAL").ok())],
            find: &find,
            installed: &installed,
            probes: DEFAULT_EDITOR_PRIORITY,
        },
    )
}

/// Resolve the editor command using `$EDITOR`, `$VISUAL`, then fallback probes.
///
/// The command as configured, or the probed binary name; see
/// [`resolve_editor`] with [`SurfaceRequest::Any`].
pub fn resolve_editor_command() -> Result<String, EditorError> {
    resolve_editor(SurfaceRequest::Any).map(|editor| editor.display)
}

pub(crate) fn resolve_editor_from(
    request: SurfaceRequest,
    inputs: &ResolutionInputs<'_>,
) -> Result<ResolvedEditor, EditorError> {
    let mut candidates = Vec::new();
    let mut first_rejection = None;

    for &(variable, ref value) in &inputs.variables {
        let Some(value) = value.as_deref() else {
            continue;
        };
        let mut words = value.split_whitespace();
        let Some(token) = words.next() else {
            continue;
        };
        let configured_args: Vec<String> = words.map(str::to_owned).collect();
        let identified = Editor::identify(token);

        let reason = match (inputs.find)(token) {
            None => RejectReason::Missing,
            Some(_) if identified.is_some_and(|editor| !editor.can_block()) => RejectReason::CannotWait,
            Some(program) => match surface_mismatch(request, identified.map(|editor| editor.surface())) {
                Some(reason) => reason,
                None => {
                    return Ok(ResolvedEditor {
                        program,
                        args: with_wait_args(configured_args, identified),
                        surface: identified.map(|editor| editor.surface()),
                        display: value.to_string(),
                    });
                }
            },
        };
        first_rejection.get_or_insert_with(|| match reason {
            RejectReason::CannotWait => EditorError::ConfiguredEditorCannotWait {
                variable,
                value: value.to_string(),
            },
            _ => EditorError::ConfiguredEditorMissing {
                variable,
                value: value.to_string(),
            },
        });
        candidates.push((format!("${variable}={value}"), reason));
    }

    if request == SurfaceRequest::Any
        && let Some(error) = first_rejection
    {
        return Err(error);
    }

    for &editor in inputs.probes {
        let name = editor.binary_name().to_string();
        let rejection = if !editor.can_block() {
            Some(RejectReason::CannotWait)
        } else {
            surface_mismatch(request, Some(editor.surface()))
        };
        if let Some(reason) = rejection {
            candidates.push((name, reason));
            continue;
        }
        match (inputs.installed)(editor) {
            Some(program) => {
                return Ok(ResolvedEditor {
                    program,
                    args: with_wait_args(Vec::new(), Some(editor)),
                    surface: Some(editor.surface()),
                    display: name,
                });
            }
            None => candidates.push((name, RejectReason::NotInstalled)),
        }
    }

    match request {
        SurfaceRequest::Any => Err(EditorError::NoEditorFound),
        requested => Err(EditorError::NoMatchingEditor { requested, candidates }),
    }
}

/// `Some(SurfaceMismatch)` unless the request is `Any` or names exactly this
/// surface; a dual-surface or unknown command never matches an explicit one.
fn surface_mismatch(request: SurfaceRequest, surface: Option<EditorSurface>) -> Option<RejectReason> {
    let matches = match request {
        SurfaceRequest::Any => true,
        SurfaceRequest::Terminal => surface == Some(EditorSurface::Terminal),
        SurfaceRequest::Gui => surface == Some(EditorSurface::Gui),
    };
    (!matches).then_some(RejectReason::SurfaceMismatch(surface))
}

fn with_wait_args(mut args: Vec<String>, editor: Option<Editor>) -> Vec<String> {
    for flag in editor.map(|editor| editor.wait_args()).unwrap_or_default() {
        if !args.iter().any(|arg| arg == flag) {
            args.push((*flag).to_string());
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Programs "on PATH" for one test: token or binary name → path.
    struct Host {
        on_path: Vec<&'static str>,
        editor: Option<&'static str>,
        visual: Option<&'static str>,
    }

    impl Host {
        fn new(on_path: &[&'static str]) -> Self {
            Self { on_path: on_path.to_vec(), editor: None, visual: None }
        }

        fn editor(mut self, value: &'static str) -> Self {
            self.editor = Some(value);
            self
        }

        fn visual(mut self, value: &'static str) -> Self {
            self.visual = Some(value);
            self
        }

        fn resolve(&self, request: SurfaceRequest) -> Result<ResolvedEditor, EditorError> {
            self.resolve_counting(request, &std::cell::Cell::new(false))
        }

        fn resolve_counting(
            &self,
            request: SurfaceRequest,
            probed: &std::cell::Cell<bool>,
        ) -> Result<ResolvedEditor, EditorError> {
            let find = |token: &str| self.on_path.contains(&token).then(|| PathBuf::from(format!("/bin/{token}")));
            let installed = |editor: Editor| {
                probed.set(true);
                self.on_path
                    .contains(&editor.binary_name())
                    .then(|| PathBuf::from(format!("/bin/{}", editor.binary_name())))
            };
            resolve_editor_from(
                request,
                &ResolutionInputs {
                    variables: [("EDITOR", self.editor.map(str::to_owned)), ("VISUAL", self.visual.map(str::to_owned))],
                    find: &find,
                    installed: &installed,
                    probes: DEFAULT_EDITOR_PRIORITY,
                },
            )
        }
    }

    #[test]
    fn editor_takes_precedence_over_visual_and_keeps_its_arguments() {
        let resolved = Host::new(&["nvim", "code"]).editor("nvim -u NONE").visual("code").resolve(SurfaceRequest::Any).unwrap();

        assert_eq!(resolved.program, PathBuf::from("/bin/nvim"));
        assert_eq!(resolved.args, vec!["-u", "NONE"]);
        assert_eq!(resolved.surface, Some(EditorSurface::Terminal));
        assert_eq!(resolved.display, "nvim -u NONE");
    }

    #[test]
    fn a_blank_editor_falls_to_visual() {
        let resolved = Host::new(&["vim"]).editor("   ").visual("vim").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.display, "vim");
    }

    #[test]
    fn an_unset_editor_falls_to_visual() {
        let resolved = Host::new(&["vim"]).visual("vim").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.display, "vim");
    }

    #[test]
    fn a_missing_editor_falls_to_a_visual_that_exists() {
        let resolved = Host::new(&["vim"]).editor("nope").visual("vim").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.display, "vim");
    }

    #[test]
    fn a_configured_editor_that_is_missing_refuses_without_probing() {
        let probed = std::cell::Cell::new(false);
        let error = Host::new(&["nvim"]).editor("nope").resolve_counting(SurfaceRequest::Any, &probed).unwrap_err();

        assert!(matches!(&error, EditorError::ConfiguredEditorMissing { variable: "EDITOR", value } if value == "nope"), "{error}");
        assert!(!probed.get(), "a typo'd editor must not fall back to a probe");
    }

    #[test]
    fn a_configured_non_blocking_editor_is_skipped_and_named() {
        for editor in ["geany", "lapce"] {
            let probed = std::cell::Cell::new(false);
            let error = Host::new(&[editor, "nvim"])
                .editor(editor)
                .resolve_counting(SurfaceRequest::Any, &probed)
                .unwrap_err();

            assert!(
                matches!(&error, EditorError::ConfiguredEditorCannotWait { variable: "EDITOR", value } if value == editor),
                "{error}"
            );
            assert!(error.to_string().contains("cannot wait"), "{error}");
            assert!(!probed.get());
        }
    }

    #[test]
    fn a_non_blocking_editor_falls_to_the_next_variable() {
        let resolved = Host::new(&["geany", "code"]).editor("geany").visual("code").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.display, "code");
        assert_eq!(resolved.args, vec!["--wait"]);
    }

    #[test]
    fn the_first_rejected_variable_names_the_error() {
        let error = Host::new(&["lapce"]).editor("nope").visual("lapce").resolve(SurfaceRequest::Any).unwrap_err();
        assert!(matches!(error, EditorError::ConfiguredEditorMissing { variable: "EDITOR", .. }), "{error}");
    }

    #[test]
    fn probes_never_select_a_non_blocking_editor() {
        let resolved = Host::new(&["lapce", "geany", "nano"]).resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.display, "nano");
    }

    #[test]
    fn only_non_blocking_editors_installed_is_no_editor_found() {
        let error = Host::new(&["lapce", "geany"]).resolve(SurfaceRequest::Any).unwrap_err();
        assert!(matches!(error, EditorError::NoEditorFound), "{error}");
    }

    #[test]
    fn a_probe_launches_the_path_it_was_found_at() {
        let resolved = Host::new(&["hx"]).resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.program, PathBuf::from("/bin/hx"));
        assert_eq!(resolved.display, "hx");
        assert!(resolved.args.is_empty());
    }

    #[test]
    fn a_configured_wait_flag_is_not_doubled() {
        let resolved = Host::new(&["code"]).editor("code --wait").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.args, vec!["--wait"]);
    }

    #[test]
    fn an_absolute_path_to_a_known_gui_editor_gets_its_wait_flag() {
        let resolved = Host::new(&["/usr/local/bin/code"]).editor("/usr/local/bin/code").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.args, vec!["--wait"]);
        assert_eq!(resolved.surface, Some(EditorSurface::Gui));
    }

    #[test]
    fn kate_gets_block() {
        let resolved = Host::new(&["kate"]).editor("kate").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.args, vec!["--block"]);
    }

    #[test]
    fn an_unknown_command_is_trusted_without_gui() {
        let resolved = Host::new(&["my-wrapper"]).editor("my-wrapper --flag").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.surface, None);
        assert_eq!(resolved.args, vec!["--flag"]);
        assert!(resolved.uses_terminal());
    }

    #[test]
    fn gui_true_skips_a_terminal_editor_for_a_gui_visual() {
        let resolved = Host::new(&["nvim", "code"]).editor("nvim").visual("code").resolve(SurfaceRequest::Gui).unwrap();
        assert_eq!(resolved.display, "code");
        assert_eq!(resolved.args, vec!["--wait"]);
        assert!(!resolved.uses_terminal());
    }

    #[test]
    fn gui_false_keeps_a_terminal_editor() {
        let resolved = Host::new(&["nvim", "code"]).editor("nvim").visual("code").resolve(SurfaceRequest::Terminal).unwrap();
        assert_eq!(resolved.display, "nvim");
    }

    #[test]
    fn gui_omitted_keeps_ordinary_precedence() {
        let resolved = Host::new(&["nvim", "code"]).editor("nvim").visual("code").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.display, "nvim");
    }

    #[test]
    fn gui_false_with_a_dual_surface_editor_probes_for_a_terminal_editor() {
        let resolved = Host::new(&["emacs", "nvim"]).editor("emacs").resolve(SurfaceRequest::Terminal).unwrap();
        assert_eq!(resolved.display, "nvim");
    }

    #[test]
    fn an_explicit_surface_never_matches_a_dual_or_unknown_editor() {
        for request in [SurfaceRequest::Gui, SurfaceRequest::Terminal] {
            let error = Host::new(&["emacs", "my-wrapper"]).editor("emacs").visual("my-wrapper").resolve(request).unwrap_err();
            let EditorError::NoMatchingEditor { requested, candidates } = &error else {
                panic!("{error}");
            };
            assert_eq!(*requested, request);
            assert_eq!(candidates[0], ("$EDITOR=emacs".to_string(), RejectReason::SurfaceMismatch(Some(EditorSurface::Both))));
            assert_eq!(candidates[1], ("$VISUAL=my-wrapper".to_string(), RejectReason::SurfaceMismatch(None)));
        }
    }

    #[test]
    fn an_explicit_surface_probes_although_a_variable_is_set() {
        let probed = std::cell::Cell::new(false);
        let resolved = Host::new(&["nvim", "zed"]).editor("nvim").resolve_counting(SurfaceRequest::Gui, &probed).unwrap();
        assert!(probed.get());
        assert_eq!(resolved.display, "zed");
        assert_eq!(resolved.args, vec!["--wait"]);
    }

    #[test]
    fn no_matching_editor_lists_every_candidate_with_its_reason() {
        let error = Host::new(&["nvim", "lapce"]).editor("nope").visual("nvim").resolve(SurfaceRequest::Gui).unwrap_err();
        let EditorError::NoMatchingEditor { requested, candidates } = &error else {
            panic!("{error}");
        };
        assert_eq!(*requested, SurfaceRequest::Gui);
        assert_eq!(candidates.len(), 2 + DEFAULT_EDITOR_PRIORITY.len());
        assert_eq!(candidates[0], ("$EDITOR=nope".to_string(), RejectReason::Missing));
        assert_eq!(candidates[1], ("$VISUAL=nvim".to_string(), RejectReason::SurfaceMismatch(Some(EditorSurface::Terminal))));
        let reason_of = |name: &str| candidates.iter().find(|(candidate, _)| candidate == name).map(|(_, reason)| *reason);
        assert_eq!(reason_of("lapce"), Some(RejectReason::CannotWait));
        assert_eq!(reason_of("code"), Some(RejectReason::NotInstalled));
        assert_eq!(reason_of("nano"), Some(RejectReason::SurfaceMismatch(Some(EditorSurface::Terminal))));

        let message = error.to_string();
        for expected in ["gui", "$EDITOR=nope", "not found", "lapce", "cannot wait", "code"] {
            assert!(message.contains(expected), "{expected:?} missing from {message}");
        }
    }

    #[test]
    fn surface_request_follows_the_gui_option() {
        assert_eq!(SurfaceRequest::from(None), SurfaceRequest::Any);
        assert_eq!(SurfaceRequest::from(Some(true)), SurfaceRequest::Gui);
        assert_eq!(SurfaceRequest::from(Some(false)), SurfaceRequest::Terminal);
    }

    #[test]
    fn full_command_omits_the_configured_arguments_duplicated_in_display() {
        let resolved = Host::new(&["code"]).editor("code -n").resolve(SurfaceRequest::Any).unwrap();
        assert_eq!(resolved.full_command(), "code -n --wait");
    }
}
