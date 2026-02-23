use std::sync::LazyLock;

use biscuit_terminal::prelude::{Prose, Renderable};

pub static YOLO: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-red-900><bold><red-200><bold> YOLO </bold></red-200></bold></bg-red-900>")
        .render(None)
        .to_string()
});

pub static WRAPPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-gray-100><bold><black><bold> Wrapped </bold></black></bg-gray-100>")
        .render(None)
        .to_string()
});

pub static NON_INTERACTIVE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-slate-300><bold><purple-900><bold> Non-Interactive </bold></purple-900></bg-slate-300>",
    )
    .render(None)
    .to_string()
});
