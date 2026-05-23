/// **PageFeature**
///
/// - Enumerates all of the reusable features which can be added to a
///   HTML page.
/// - Each feature will be used to look the code, CSS styling, and
///   HEAD based metadata that this feature requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageFeature {
    CopyToClipboard,
    PasteFromClipboard,
    CollapseNestedLists,
    ExpandNestedLists,
    ShowDialog,
    MermaidDiagram,
    DarkMode,
}
