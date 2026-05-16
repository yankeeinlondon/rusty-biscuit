use crate::html::attribute::DomId;

/// A value that can be `true`, `false`, or `mixed`. Used by ARIA states
/// such as `aria-checked` and `aria-pressed` that have an indeterminate
/// "mixed" state in addition to the standard boolean values.
pub enum AriaTriState {
    True,
    False,
    Mixed,
}

/// Token values accepted by `aria-autocomplete`.
pub enum AriaAutoComplete {
    /// No input completion suggestions are provided.
    None,
    /// Text suggesting one way to complete the provided input is dynamically inserted after the caret.
    Inline,
    /// A list of choices appears from which the user can choose.
    List,
    /// Both an inline completion and a list of choices appear.
    Both,
}

/// Token values accepted by `aria-haspopup`.
pub enum AriaHasPopup {
    /// Indicates the element does not have a popup.
    False,
    /// Indicates the popup is a menu (default `true` behavior).
    True,
    /// Indicates the popup is a menu.
    Menu,
    /// Indicates the popup is a listbox.
    ListBox,
    /// Indicates the popup is a tree.
    Tree,
    /// Indicates the popup is a grid.
    Grid,
    /// Indicates the popup is a dialog.
    Dialog,
}

/// Token values accepted by `aria-invalid`.
pub enum AriaInvalid {
    /// There are no detected errors in the value.
    False,
    /// The value entered by the user has failed validation.
    True,
    /// A grammatical error was detected.
    Grammar,
    /// A spelling error was detected.
    Spelling,
}

/// Token values accepted by `aria-orientation`.
pub enum AriaOrientation {
    /// The element is oriented horizontally.
    Horizontal,
    /// The element is oriented vertically.
    Vertical,
}

/// Token values accepted by `aria-live`.
pub enum AriaLive {
    /// Updates to the region have the highest priority and should be presented immediately.
    Assertive,
    /// Updates to the region are not presented to the user unless focus is on the region.
    Off,
    /// Updates to the region should be presented at the next graceful opportunity.
    Polite,
}

/// Token values accepted by `aria-relevant`. The attribute accepts a
/// space-separated list of these tokens.
pub enum AriaRelevant {
    /// Element nodes are added to the accessibility tree within the live region.
    Additions,
    /// Equivalent to combining all other tokens.
    All,
    /// Text or element nodes within the live region are removed from the accessibility tree.
    Removals,
    /// Text content is added to any descendant of the live region.
    Text,
}

/// Token values accepted by `aria-sort`.
pub enum AriaSort {
    /// Items are sorted in ascending order.
    Ascending,
    /// Items are sorted in descending order.
    Descending,
    /// There is no defined sort applied.
    None,
    /// A sort algorithm other than ascending or descending has been applied.
    Other,
}

/// Token values accepted by `aria-current`.
pub enum AriaCurrent {
    /// Does not represent the current item within a set.
    False,
    /// Represents the current item within a set, without a more specific designation.
    True,
    /// Represents the current page within a set of pages.
    Page,
    /// Represents the current step within a process.
    Step,
    /// Represents the current location within an environment or context.
    Location,
    /// Represents the current date within a collection of dates.
    Date,
    /// Represents the current time within a set of times.
    Time,
}

/// Token values accepted by `aria-dropeffect`. The attribute accepts a
/// space-separated list of these tokens.
///
/// > **Note:** `aria-dropeffect` is deprecated in ARIA 1.1 and later.
pub enum AriaDropEffect {
    /// A duplicate of the source object will be dropped into the target.
    Copy,
    /// A function supported by the drop target is executed using the dragged source object as input.
    Execute,
    /// A reference or shortcut to the dragged object will be created in the target.
    Link,
    /// The source object will be removed from its current location and dropped into the target.
    Move,
    /// No operation can be performed; equivalent to dismissing the drag operation.
    None,
    /// There is a popup menu or dialog that allows the user to choose one of the drag operations.
    Popup,
}

/// Enumerates the HTML attributes that follow the `aria-{name}="{value}"`
/// pattern, with each variant carrying the value type required by that
/// attribute per the WAI-ARIA 1.2 specification.
///
/// Distinct from [`AriaRole`], which models the values of the `role`
/// attribute itself.
pub enum AriaAttribute {
    // --- Widget attributes ---
    /// Indicates whether inputting text could trigger display of one or more predictions of the user's intended value.
    AutoComplete(AriaAutoComplete),
    /// Indicates the current "checked" state of checkboxes, radio buttons, and other widgets.
    Checked(AriaTriState),
    /// Indicates that the element is perceivable but disabled, so it is not editable or otherwise operable.
    Disabled(bool),
    /// Identifies the element that provides an error message for an object.
    ErrorMessage(DomId),
    /// Indicates whether a grouping element owned or controlled by this element is expanded or collapsed.
    Expanded(bool),
    /// Indicates the availability and type of interactive popup element that can be triggered by the element.
    HasPopup(AriaHasPopup),
    /// Indicates whether the element is exposed to an accessibility API.
    Hidden(bool),
    /// Indicates the entered value does not conform to the format expected by the application.
    Invalid(AriaInvalid),
    /// Indicates keyboard shortcuts the author has implemented to activate or give focus to an element.
    KeyShortcuts(String),
    /// Defines a string value that labels the current element.
    Label(String),
    /// Defines the hierarchical level of an element within a structure.
    Level(u32),
    /// Indicates whether an element is modal when displayed.
    Modal(bool),
    /// Indicates whether a text box accepts multiple lines of input or only a single line.
    MultiLine(bool),
    /// Indicates that the user may select more than one item from the current selectable descendants.
    MultiSelectable(bool),
    /// Indicates whether the element's orientation is horizontal, vertical, or unknown/ambiguous.
    Orientation(AriaOrientation),
    /// Defines a short hint intended to aid the user with data entry when the control has no value.
    Placeholder(String),
    /// Indicates the current "pressed" state of toggle buttons.
    Pressed(AriaTriState),
    /// Indicates that the element is not editable, but is otherwise operable.
    ReadOnly(bool),
    /// Indicates that user input is required on the element before a form may be submitted.
    Required(bool),
    /// Indicates the current "selected" state of various widgets.
    Selected(bool),
    /// Indicates if items in a table or grid are sorted in ascending or descending order.
    Sort(AriaSort),
    /// Defines the maximum allowed value for a range widget.
    ValueMax(f64),
    /// Defines the minimum allowed value for a range widget.
    ValueMin(f64),
    /// Defines the current value for a range widget.
    ValueNow(f64),
    /// Defines the human-readable text alternative of `aria-valuenow` for a range widget.
    ValueText(String),

    // --- Live region attributes ---
    /// Indicates whether assistive technologies will present all, or only parts of, the changed region.
    Atomic(bool),
    /// Indicates an element is being modified and that assistive technologies may want to wait until completion.
    Busy(bool),
    /// Indicates that an element will be updated, and describes the types of updates the user agents, AT, and user can expect.
    Live(AriaLive),
    /// Indicates what notifications the user agent will trigger when the accessibility tree within a live region is modified.
    Relevant(Vec<AriaRelevant>),

    // --- Drag-and-drop attributes (deprecated in ARIA 1.1) ---
    /// Indicates what functions can be performed when a dragged object is released on the drop target.
    DropEffect(Vec<AriaDropEffect>),
    /// Indicates an element's "grabbed" state in a drag-and-drop operation.
    Grabbed(bool),

    // --- Relationship attributes ---
    /// Identifies the currently active element when DOM focus is on a composite widget, combobox, textbox, group, or application.
    ActiveDescendant(DomId),
    /// Defines the total number of columns in a table, grid, or treegrid.
    ColCount(i32),
    /// Defines an element's column index or position with respect to the total number of columns.
    ColIndex(u32),
    /// Defines a human readable text alternative of `aria-colindex`.
    ColIndexText(String),
    /// Defines the number of columns spanned by a cell or gridcell within a table, grid, or treegrid.
    ColSpan(u32),
    /// Identifies the element (or elements) whose contents or presence are controlled by the current element.
    Controls(Vec<DomId>),
    /// Identifies the element (or elements) that describes the object.
    DescribedBy(Vec<DomId>),
    /// Defines a string value that describes or annotates the current element.
    Description(String),
    /// Identifies the element (or elements) that provide additional information related to the object.
    Details(Vec<DomId>),
    /// Identifies the next element (or elements) in an alternate reading order of content.
    FlowTo(Vec<DomId>),
    /// Identifies the element (or elements) that labels the current element.
    LabelledBy(Vec<DomId>),
    /// Identifies an element (or elements) in order to define a visual, functional, or contextual parent/child relationship.
    Owns(Vec<DomId>),
    /// Defines an element's number or position in the current set of listitems or treeitems.
    PosInSet(u32),
    /// Defines the total number of rows in a table, grid, or treegrid.
    RowCount(i32),
    /// Defines an element's row index or position with respect to the total number of rows.
    RowIndex(u32),
    /// Defines a human readable text alternative of `aria-rowindex`.
    RowIndexText(String),
    /// Defines the number of rows spanned by a cell or gridcell within a table, grid, or treegrid.
    RowSpan(u32),
    /// Defines the number of items in the current set of listitems or treeitems.
    SetSize(i32),

    // --- Other global attributes ---
    /// Indicates the element that represents the current item within a container or set of related elements.
    Current(AriaCurrent),
    /// Defines a human-readable, author-localized description for the role of an element.
    RoleDescription(String),
    /// Defines a string value that labels the current element for users of assistive technologies using a braille display.
    BrailleLabel(String),
    /// Defines a human-readable, author-localized description for the role of an element, for braille users.
    BrailleRoleDescription(String),
}

impl AriaTriState {
    /// The string token for this tri-state value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaTriState::True => "true",
            AriaTriState::False => "false",
            AriaTriState::Mixed => "mixed",
        }
    }
}

impl AriaAutoComplete {
    /// The `aria-autocomplete` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaAutoComplete::None => "none",
            AriaAutoComplete::Inline => "inline",
            AriaAutoComplete::List => "list",
            AriaAutoComplete::Both => "both",
        }
    }
}

impl AriaHasPopup {
    /// The `aria-haspopup` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaHasPopup::False => "false",
            AriaHasPopup::True => "true",
            AriaHasPopup::Menu => "menu",
            AriaHasPopup::ListBox => "listbox",
            AriaHasPopup::Tree => "tree",
            AriaHasPopup::Grid => "grid",
            AriaHasPopup::Dialog => "dialog",
        }
    }
}

impl AriaInvalid {
    /// The `aria-invalid` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaInvalid::False => "false",
            AriaInvalid::True => "true",
            AriaInvalid::Grammar => "grammar",
            AriaInvalid::Spelling => "spelling",
        }
    }
}

impl AriaOrientation {
    /// The `aria-orientation` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaOrientation::Horizontal => "horizontal",
            AriaOrientation::Vertical => "vertical",
        }
    }
}

impl AriaLive {
    /// The `aria-live` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaLive::Assertive => "assertive",
            AriaLive::Off => "off",
            AriaLive::Polite => "polite",
        }
    }
}

impl AriaRelevant {
    /// The `aria-relevant` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaRelevant::Additions => "additions",
            AriaRelevant::All => "all",
            AriaRelevant::Removals => "removals",
            AriaRelevant::Text => "text",
        }
    }
}

impl AriaSort {
    /// The `aria-sort` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaSort::Ascending => "ascending",
            AriaSort::Descending => "descending",
            AriaSort::None => "none",
            AriaSort::Other => "other",
        }
    }
}

impl AriaCurrent {
    /// The `aria-current` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaCurrent::False => "false",
            AriaCurrent::True => "true",
            AriaCurrent::Page => "page",
            AriaCurrent::Step => "step",
            AriaCurrent::Location => "location",
            AriaCurrent::Date => "date",
            AriaCurrent::Time => "time",
        }
    }
}

impl AriaDropEffect {
    /// The `aria-dropeffect` token for this value.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaDropEffect::Copy => "copy",
            AriaDropEffect::Execute => "execute",
            AriaDropEffect::Link => "link",
            AriaDropEffect::Move => "move",
            AriaDropEffect::None => "none",
            AriaDropEffect::Popup => "popup",
        }
    }
}

impl AriaAttribute {
    /// Renders this attribute as a leading-space `aria-{name}="{value}"`
    /// fragment, with the value escaped for a double-quoted attribute.
    ///
    /// ARIA booleans are string-valued (`"true"`/`"false"`), not HTML
    /// boolean attributes, so they always emit an explicit value.
    pub fn render(&self) -> String {
        fn join_ids(ids: &[DomId]) -> String {
            ids.iter()
                .map(DomId::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        }
        let bool_str = |value: bool| if value { "true" } else { "false" }.to_string();

        let (name, value): (&str, String) = match self {
            AriaAttribute::AutoComplete(v) => ("autocomplete", v.as_str().to_string()),
            AriaAttribute::Checked(v) => ("checked", v.as_str().to_string()),
            AriaAttribute::Disabled(v) => ("disabled", bool_str(*v)),
            AriaAttribute::ErrorMessage(v) => ("errormessage", v.as_str().to_string()),
            AriaAttribute::Expanded(v) => ("expanded", bool_str(*v)),
            AriaAttribute::HasPopup(v) => ("haspopup", v.as_str().to_string()),
            AriaAttribute::Hidden(v) => ("hidden", bool_str(*v)),
            AriaAttribute::Invalid(v) => ("invalid", v.as_str().to_string()),
            AriaAttribute::KeyShortcuts(v) => ("keyshortcuts", v.clone()),
            AriaAttribute::Label(v) => ("label", v.clone()),
            AriaAttribute::Level(v) => ("level", v.to_string()),
            AriaAttribute::Modal(v) => ("modal", bool_str(*v)),
            AriaAttribute::MultiLine(v) => ("multiline", bool_str(*v)),
            AriaAttribute::MultiSelectable(v) => ("multiselectable", bool_str(*v)),
            AriaAttribute::Orientation(v) => ("orientation", v.as_str().to_string()),
            AriaAttribute::Placeholder(v) => ("placeholder", v.clone()),
            AriaAttribute::Pressed(v) => ("pressed", v.as_str().to_string()),
            AriaAttribute::ReadOnly(v) => ("readonly", bool_str(*v)),
            AriaAttribute::Required(v) => ("required", bool_str(*v)),
            AriaAttribute::Selected(v) => ("selected", bool_str(*v)),
            AriaAttribute::Sort(v) => ("sort", v.as_str().to_string()),
            AriaAttribute::ValueMax(v) => ("valuemax", v.to_string()),
            AriaAttribute::ValueMin(v) => ("valuemin", v.to_string()),
            AriaAttribute::ValueNow(v) => ("valuenow", v.to_string()),
            AriaAttribute::ValueText(v) => ("valuetext", v.clone()),
            AriaAttribute::Atomic(v) => ("atomic", bool_str(*v)),
            AriaAttribute::Busy(v) => ("busy", bool_str(*v)),
            AriaAttribute::Live(v) => ("live", v.as_str().to_string()),
            AriaAttribute::Relevant(v) => (
                "relevant",
                v.iter()
                    .map(AriaRelevant::as_str)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            AriaAttribute::DropEffect(v) => (
                "dropeffect",
                v.iter()
                    .map(AriaDropEffect::as_str)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            AriaAttribute::Grabbed(v) => ("grabbed", bool_str(*v)),
            AriaAttribute::ActiveDescendant(v) => {
                ("activedescendant", v.as_str().to_string())
            }
            AriaAttribute::ColCount(v) => ("colcount", v.to_string()),
            AriaAttribute::ColIndex(v) => ("colindex", v.to_string()),
            AriaAttribute::ColIndexText(v) => ("colindextext", v.clone()),
            AriaAttribute::ColSpan(v) => ("colspan", v.to_string()),
            AriaAttribute::Controls(v) => ("controls", join_ids(v)),
            AriaAttribute::DescribedBy(v) => ("describedby", join_ids(v)),
            AriaAttribute::Description(v) => ("description", v.clone()),
            AriaAttribute::Details(v) => ("details", join_ids(v)),
            AriaAttribute::FlowTo(v) => ("flowto", join_ids(v)),
            AriaAttribute::LabelledBy(v) => ("labelledby", join_ids(v)),
            AriaAttribute::Owns(v) => ("owns", join_ids(v)),
            AriaAttribute::PosInSet(v) => ("posinset", v.to_string()),
            AriaAttribute::RowCount(v) => ("rowcount", v.to_string()),
            AriaAttribute::RowIndex(v) => ("rowindex", v.to_string()),
            AriaAttribute::RowIndexText(v) => ("rowindextext", v.clone()),
            AriaAttribute::RowSpan(v) => ("rowspan", v.to_string()),
            AriaAttribute::SetSize(v) => ("setsize", v.to_string()),
            AriaAttribute::Current(v) => ("current", v.as_str().to_string()),
            AriaAttribute::RoleDescription(v) => ("roledescription", v.clone()),
            AriaAttribute::BrailleLabel(v) => ("braillelabel", v.clone()),
            AriaAttribute::BrailleRoleDescription(v) => {
                ("brailleroledescription", v.clone())
            }
        };
        format!(
            r#" aria-{name}="{}""#,
            crate::browser::utils::escape_attribute(&value)
        )
    }
}

/// Enumerates the WAI-ARIA roles that may be assigned to an element via the
/// `role` attribute. Roles are grouped into widget, document structure,
/// landmark, live region, and window categories.
pub enum AriaRole {
    // --- Widget roles ---
    /// A message with important, and usually time-sensitive, information.
    Alert,
    /// A type of dialog that contains an alert message.
    AlertDialog,
    /// A clickable input that triggers a response when activated.
    Button,
    /// A checkable input with three possible values: `true`, `false`, or `mixed`.
    Checkbox,
    /// An input that controls another element (such as a listbox or grid) that
    /// can dynamically pop up to help the user set the value of that input.
    Combobox,
    /// A cell in a grid or treegrid.
    GridCell,
    /// An interactive reference to an internal or external resource.
    Link,
    /// An option in a set of choices contained by a menu or menubar.
    MenuItem,
    /// A menu item with a checkable state whose values are `true`, `false`, or `mixed`.
    MenuItemCheckbox,
    /// A checkable menu item in a set of menu items, only one of which can be checked at a time.
    MenuItemRadio,
    /// A selectable item in a `listbox`.
    Option,
    /// An element that displays the progress status for tasks that take a long time.
    ProgressBar,
    /// A checkable input in a group of radio inputs, only one of which can be checked at a time.
    Radio,
    /// A graphical object that controls the scrolling of content within a viewing area.
    ScrollBar,
    /// A type of textbox intended for specifying search criteria.
    SearchBox,
    /// A divider that separates and distinguishes sections of content or groups of menu items.
    Separator,
    /// A user input where the user selects a value from within a given range.
    Slider,
    /// A form of `range` that expects the user to select from among discrete choices.
    SpinButton,
    /// A type of checkbox that represents on/off values.
    Switch,
    /// A grouping label providing a mechanism for selecting the tab content to be rendered.
    Tab,
    /// A container for the resources associated with a `tab`.
    TabPanel,
    /// A type of input that allows free-form text as its value.
    TextBox,
    /// An option item of a `tree`.
    TreeItem,

    // --- Composite widget roles ---
    /// A composite widget containing a collection of one or more rows with one
    /// or more cells where some or all cells in the grid are focusable.
    Grid,
    /// A widget that allows the user to select one or more items from a list of choices.
    ListBox,
    /// A type of widget that offers a list of choices to the user.
    Menu,
    /// A presentation of `menu` that usually remains visible and is usually horizontal.
    MenuBar,
    /// A group of radio buttons.
    RadioGroup,
    /// A list of `tab` elements that reference `tabpanel` elements.
    TabList,
    /// A widget that allows the user to select one or more items from a hierarchically organized collection.
    Tree,
    /// A grid whose rows can be expanded and collapsed in the same manner as for a tree.
    TreeGrid,

    // --- Document structure roles ---
    /// A structure that is a logical equivalent of a piece of writing.
    Article,
    /// A section of content that is quoted from another source.
    Blockquote,
    /// A visible caption for a `figure`, `table`, or `grid`.
    Caption,
    /// A cell in a tabular container.
    Cell,
    /// A section whose content represents a fragment of computer code.
    Code,
    /// A cell containing header information for a column.
    ColumnHeader,
    /// A comment contains content expressing reaction to other content.
    Comment,
    /// A definition of a term or concept.
    Definition,
    /// A deletion represents content that is marked as removed.
    Deletion,
    /// An element containing content that assistive technology users may want to browse in a reading mode.
    Document,
    /// An element representing emphatic stress in textual content.
    Emphasis,
    /// A scrollable list of articles where scrolling may cause articles to be added to or removed.
    Feed,
    /// A perceivable section of content that typically contains a graphical document, images, code snippets, or example text.
    Figure,
    /// A nameless container that has no semantic meaning on its own.
    Generic,
    /// A set of user interface objects which are not intended to be included in a page summary.
    Group,
    /// A heading for a section of the page.
    Heading,
    /// A container for a collection of elements that form an image.
    Image,
    /// A change to the document where content has been added.
    Insertion,
    /// A section containing listitem elements.
    List,
    /// A single item in a list or directory.
    ListItem,
    /// Content that is marked or highlighted for reference or notation purposes.
    Mark,
    /// Content that represents a mathematical expression.
    Math,
    /// An element that represents a scalar measurement within a known range, or a fractional value.
    Meter,
    /// An element whose implicit native role semantics will not be mapped to the accessibility API.
    None,
    /// A section whose content is parenthetic or ancillary to the main content of the resource.
    Note,
    /// A paragraph of content.
    Paragraph,
    /// An element whose implicit native role semantics will not be mapped to the accessibility API. (Synonym for `None`.)
    Presentation,
    /// A row of cells in a tabular container.
    Row,
    /// A structure containing one or more `row` elements in a tabular container.
    RowGroup,
    /// A cell containing header information for a row.
    RowHeader,
    /// Content that represents strong importance, seriousness, or urgency.
    Strong,
    /// A subscript to a base.
    Subscript,
    /// A content change suggestion containing an `insertion` and/or `deletion`.
    Suggestion,
    /// A superscript to a base.
    Superscript,
    /// A section containing data arranged in rows and columns.
    Table,
    /// A word or phrase with an optional corresponding definition.
    Term,
    /// An element that represents a specific point in time.
    Time,
    /// A collection of commonly used function buttons or controls represented in compact visual form.
    Toolbar,
    /// A contextual popup that displays a description for an element.
    Tooltip,

    // --- Landmark roles ---
    /// A region that contains mostly site-oriented content, rather than page-specific content.
    Banner,
    /// A supporting section of the document, designed to be complementary to the main content.
    Complementary,
    /// A large perceivable region that contains information about the parent document.
    ContentInfo,
    /// A landmark region that contains a collection of items and objects that combine to create a form.
    Form,
    /// The main content of a document.
    Main,
    /// A collection of navigational elements (usually links) for navigating the document or related documents.
    Navigation,
    /// A perceivable section containing content that is relevant to a specific, author-specified purpose.
    Region,
    /// A landmark region that contains a collection of items and objects that, as a whole, combine to create a search facility.
    Search,

    // --- Live region roles ---
    /// A type of live region with important, and usually time-sensitive, information. (Synonymous with `Alert` semantics.)
    Log,
    /// A type of live region where non-essential information changes frequently.
    Marquee,
    /// A container whose content is advisory information for the user but is not important enough to justify an alert.
    Status,
    /// A type of live region containing a numerical counter which indicates an amount of elapsed time or remaining time.
    Timer,

    // --- Window roles ---
    /// A dialog is a descendant window of the primary window of a web application.
    Dialog,

    // --- Application role ---
    /// A structure containing one or more focusable elements requiring user input, such as keyboard or gesture events.
    Application,
}

impl AriaRole {
    /// The lowercase `role` attribute token for this WAI-ARIA role.
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaRole::Alert => "alert",
            AriaRole::AlertDialog => "alertdialog",
            AriaRole::Button => "button",
            AriaRole::Checkbox => "checkbox",
            AriaRole::Combobox => "combobox",
            AriaRole::GridCell => "gridcell",
            AriaRole::Link => "link",
            AriaRole::MenuItem => "menuitem",
            AriaRole::MenuItemCheckbox => "menuitemcheckbox",
            AriaRole::MenuItemRadio => "menuitemradio",
            AriaRole::Option => "option",
            AriaRole::ProgressBar => "progressbar",
            AriaRole::Radio => "radio",
            AriaRole::ScrollBar => "scrollbar",
            AriaRole::SearchBox => "searchbox",
            AriaRole::Separator => "separator",
            AriaRole::Slider => "slider",
            AriaRole::SpinButton => "spinbutton",
            AriaRole::Switch => "switch",
            AriaRole::Tab => "tab",
            AriaRole::TabPanel => "tabpanel",
            AriaRole::TextBox => "textbox",
            AriaRole::TreeItem => "treeitem",
            AriaRole::Grid => "grid",
            AriaRole::ListBox => "listbox",
            AriaRole::Menu => "menu",
            AriaRole::MenuBar => "menubar",
            AriaRole::RadioGroup => "radiogroup",
            AriaRole::TabList => "tablist",
            AriaRole::Tree => "tree",
            AriaRole::TreeGrid => "treegrid",
            AriaRole::Article => "article",
            AriaRole::Blockquote => "blockquote",
            AriaRole::Caption => "caption",
            AriaRole::Cell => "cell",
            AriaRole::Code => "code",
            AriaRole::ColumnHeader => "columnheader",
            AriaRole::Comment => "comment",
            AriaRole::Definition => "definition",
            AriaRole::Deletion => "deletion",
            AriaRole::Document => "document",
            AriaRole::Emphasis => "emphasis",
            AriaRole::Feed => "feed",
            AriaRole::Figure => "figure",
            AriaRole::Generic => "generic",
            AriaRole::Group => "group",
            AriaRole::Heading => "heading",
            AriaRole::Image => "image",
            AriaRole::Insertion => "insertion",
            AriaRole::List => "list",
            AriaRole::ListItem => "listitem",
            AriaRole::Mark => "mark",
            AriaRole::Math => "math",
            AriaRole::Meter => "meter",
            AriaRole::None => "none",
            AriaRole::Note => "note",
            AriaRole::Paragraph => "paragraph",
            AriaRole::Presentation => "presentation",
            AriaRole::Row => "row",
            AriaRole::RowGroup => "rowgroup",
            AriaRole::RowHeader => "rowheader",
            AriaRole::Strong => "strong",
            AriaRole::Subscript => "subscript",
            AriaRole::Suggestion => "suggestion",
            AriaRole::Superscript => "superscript",
            AriaRole::Table => "table",
            AriaRole::Term => "term",
            AriaRole::Time => "time",
            AriaRole::Toolbar => "toolbar",
            AriaRole::Tooltip => "tooltip",
            AriaRole::Banner => "banner",
            AriaRole::Complementary => "complementary",
            AriaRole::ContentInfo => "contentinfo",
            AriaRole::Form => "form",
            AriaRole::Main => "main",
            AriaRole::Navigation => "navigation",
            AriaRole::Region => "region",
            AriaRole::Search => "search",
            AriaRole::Log => "log",
            AriaRole::Marquee => "marquee",
            AriaRole::Status => "status",
            AriaRole::Timer => "timer",
            AriaRole::Dialog => "dialog",
            AriaRole::Application => "application",
        }
    }
}
