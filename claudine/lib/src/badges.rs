use std::sync::LazyLock;

use biscuit_terminal::prelude::{Prose, TerminalRenderable};

pub static YOLO: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-red-900><bold><red-200><bold> YOLO </bold></red-200></bold></bg-red-900>")
        .render_optimistic(None)
        .to_string()
});

pub static PROTECT: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-green-900><green-200><bold> PROTECT </bold></green-200></bg-green-900>")
        .render_optimistic(None)
        .to_string()
});

pub static WRAPPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-gray-100><bold><black><bold> Wrapped </bold></black></bg-gray-100>")
        .render_optimistic(None)
        .to_string()
});

pub static NON_INTERACTIVE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-slate-300><bold><purple-900><bold> Non-Interactive </bold></purple-900></bg-slate-300>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static USER_SCOPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-purple-300><bold><slate-900><bold> User </bold></slate-900></bg-purple-300>")
        .render_optimistic(None)
        .to_string()
});

pub static REPO_SCOPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-blue-300><bold><slate-900><bold> Repo </bold></slate-900></bg-blue-300>")
        .render_optimistic(None)
        .to_string()
});

pub static MASKED_REPO_SCOPED: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-orange><bold><slate-900><bold> Repo</bold> (<i>masked</i>) </slate-900></bg-orange>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static EXCEPTIONS: LazyLock<String> = LazyLock::new(|| {
    Prose::new("<bg-red-800><bold><red-100><bold> Exceptions </bold></red-100></bold></bg-red-800>")
        .render_optimistic(None)
        .to_string()
});

pub static REPO_FLAG: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-gray-800><bold><green-100><bold> --repo </bold></green-100></bold></bg-gray-800>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static COMPOSE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-cyan-900><bold><cyan-100><bold> Compose </bold></cyan-100></bold></bg-cyan-900>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static VERBOSE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-amber-200><bold><amber-900><bold> Verbose </bold></amber-900></bold></bg-amber-200>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static INTERACTIVE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-purple-900><bold><slate-300><bold> Interactive </bold></slate-300></bold></bg-purple-900>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static INLINE_COMPOSE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-cyan-900><bold><cyan-100><bold> Inline Compose </bold></cyan-100></bold></bg-cyan-900>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static PROMPT_FILE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-cyan-900><bold><cyan-100><bold> Prompt File </bold></cyan-100></bold></bg-cyan-900>",
    )
    .render_optimistic(None)
    .to_string()
});

pub static SEQUENCE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-yellow-900><bold><yellow-100><bold> Sequence </bold></yellow-100></bold></bg-yellow-900>",
    )
    .render_optimistic(None)
    .to_string()
});
