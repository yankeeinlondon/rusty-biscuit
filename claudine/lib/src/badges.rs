use std::sync::LazyLock;

use biscuit_terminal::prelude::{Prose, Renderable};

pub static YOLO: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-red-900><bold><red-200><bold> YOLO </bold></red-200></bold></bg-red-900>")
        .render(None)
        .to_string()
});

pub static PROTECT: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-green-900><green-200><bold> PROTECT </bold></green-200></bg-green-900>")
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


pub static USER_SCOPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-purple-300><bold><slate-900><bold> User </bold></slate-900></bg-purple-300>",
    )
    .render(None)
    .to_string()
});

pub static REPO_SCOPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-blue-300><bold><slate-900><bold> Repo </bold></slate-900></bg-blue-300>",
    )
    .render(None)
    .to_string()
});

pub static MASKED_REPO_SCOPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-orange><bold><slate-900><bold> Repo</bold> (<i>masked</i>) </slate-900></bg-orange>",
    )
    .render(None)
    .to_string()
});

