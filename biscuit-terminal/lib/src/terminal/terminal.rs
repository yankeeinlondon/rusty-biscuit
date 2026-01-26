
fn new_terminal() {
    return Terminal {
        app: get_terminal_app(),
        italics_support: italics_support(),
        image_support: image_support(),
        underline_support: underline_support(),
        is_tty: is_tty(),
        color_depth: color_depth(),
    }
}

pub struct Terminal {
    /// The app/vendor of the terminal
    pub app: TerminalApp,

    /// Whether the terminal supports italicizing text
    pub supports_italic: bool,
    /// The type of image support (if any) the terminal provides
    pub image_support: ImageSupport,
    /// The kind of **underlining** support the terminal provides
    pub underline_support: UnderlineSupport,
    /// Whether the terminal supports OSC8 Links
    pub osc_link_support: bool,

    pub is_tty: bool,
    pub color_depth: ColorDepth,

}

impl Default for Terminal {
    fn default() -> Terminal {
        new_terminal()
    }
}

impl Terminal {
    pub fn new() -> Terminal {
        new_terminal()
    }


    pub fn width() -> u32 {
        terminal_width()
    }

    pub fn height() -> u32 {
        terminal_height()
    }


    /// Whether the terminal is in "light" or "dark" mode
    pub fn color_mode() -> ColorMode {
        color_mode()
    }


    pub fn render<T: Into<String>>(content: T) -> () {
        todo!()
    }

}
