use super::model::{FileAssociation, FileTypeDescriptor, FrameworkKind, ProgrammingLanguage};

fn text_descriptor(association: FileAssociation) -> FileTypeDescriptor {
    FileTypeDescriptor {
        association,
        language: None,
        language_type: None,
        framework: None,
        is_text: true,
    }
}

fn language_descriptor(language: ProgrammingLanguage) -> FileTypeDescriptor {
    FileTypeDescriptor {
        association: FileAssociation::ProgrammingLanguage,
        language: Some(language),
        language_type: Some(language.language_type()),
        framework: None,
        is_text: true,
    }
}

fn framework_descriptor(framework: FrameworkKind) -> FileTypeDescriptor {
    FileTypeDescriptor {
        association: FileAssociation::FrameworkFile,
        language: None,
        language_type: None,
        framework: Some(framework),
        is_text: true,
    }
}

fn binary_descriptor(association: FileAssociation) -> FileTypeDescriptor {
    FileTypeDescriptor {
        association,
        language: None,
        language_type: None,
        framework: None,
        is_text: false,
    }
}

pub fn is_command_runner_filename(file_name: &str) -> bool {
    matches!(
        file_name,
        "justfile"
            | "Justfile"
            | "Makefile"
            | "makefile"
            | "Taskfile.yml"
            | "Taskfile.yaml"
            | "Rakefile"
            | "Brewfile"
            | "Procfile"
    )
}

pub fn lookup_exact_filename(file_name: &str) -> Option<FileTypeDescriptor> {
    let lower = file_name.to_ascii_lowercase();

    if lower == ".editorconfig" {
        return Some(text_descriptor(FileAssociation::Configuration));
    }
    if lower == ".gitignore" || lower == ".gitattributes" || lower == ".gitmodules" {
        return Some(text_descriptor(FileAssociation::Configuration));
    }
    if lower.starts_with(".env") {
        return Some(text_descriptor(FileAssociation::Configuration));
    }
    if matches!(
        lower.as_str(),
        "cargo.toml"
            | "cargo.lock"
            | "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "pnpm-workspace.yaml"
            | "yarn.lock"
            | "bun.lock"
            | "bun.lockb"
            | "go.mod"
            | "go.sum"
            | "pyproject.toml"
            | "requirements.txt"
            | "pipfile"
            | "pipfile.lock"
            | "dockerfile"
            | "containerfile"
    ) {
        return Some(text_descriptor(FileAssociation::Configuration));
    }
    if matches!(
        lower.as_str(),
        "readme" | "readme.md" | "changelog" | "changelog.md" | "contributing" | "contributing.md"
    ) {
        return Some(text_descriptor(FileAssociation::Documentation));
    }
    if matches!(lower.as_str(), "justfile" | "makefile" | "rakefile" | "brewfile" | "procfile") {
        return Some(text_descriptor(FileAssociation::Configuration));
    }

    None
}

pub fn lookup_basename_pattern(file_name: &str) -> Option<FileTypeDescriptor> {
    let lower = file_name.to_ascii_lowercase();

    if lower.ends_with(".component.html") || lower.ends_with(".component.htm") {
        return Some(framework_descriptor(FrameworkKind::AngularTemplate));
    }

    None
}

pub fn lookup_extension(extension: &str) -> Option<FileTypeDescriptor> {
    let ext = extension.to_ascii_lowercase();
    let descriptor = match ext.as_str() {
        "rs" => language_descriptor(ProgrammingLanguage::Rust),
        "ts" | "tsx" | "mts" | "cts" => language_descriptor(ProgrammingLanguage::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => language_descriptor(ProgrammingLanguage::JavaScript),
        "py" | "pyw" => language_descriptor(ProgrammingLanguage::Python),
        "go" => language_descriptor(ProgrammingLanguage::Go),
        "sh" => language_descriptor(ProgrammingLanguage::Shell),
        "bash" => language_descriptor(ProgrammingLanguage::Bash),
        "zsh" => language_descriptor(ProgrammingLanguage::Zsh),
        "fish" => language_descriptor(ProgrammingLanguage::Fish),
        "ps1" | "psm1" | "psd1" => language_descriptor(ProgrammingLanguage::PowerShell),
        "bat" | "cmd" => language_descriptor(ProgrammingLanguage::Batch),
        "rb" => language_descriptor(ProgrammingLanguage::Ruby),
        "php" => language_descriptor(ProgrammingLanguage::Php),
        "lua" => language_descriptor(ProgrammingLanguage::Lua),
        "swift" => language_descriptor(ProgrammingLanguage::Swift),
        "zig" => language_descriptor(ProgrammingLanguage::Zig),
        "c" | "h" => language_descriptor(ProgrammingLanguage::C),
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => {
            language_descriptor(ProgrammingLanguage::Cpp)
        }
        "java" => language_descriptor(ProgrammingLanguage::Java),
        "kt" | "kts" => language_descriptor(ProgrammingLanguage::Kotlin),
        "scala" | "sc" => language_descriptor(ProgrammingLanguage::Scala),
        "clj" | "cljs" | "cljc" | "edn" => language_descriptor(ProgrammingLanguage::Clojure),
        "cs" => language_descriptor(ProgrammingLanguage::CSharp),
        "fs" | "fsi" | "fsx" => language_descriptor(ProgrammingLanguage::FSharp),
        "jsonnet" | "libsonnet" => language_descriptor(ProgrammingLanguage::Jsonnet),
        "wat" | "wast" => language_descriptor(ProgrammingLanguage::Wat),
        "vue" => framework_descriptor(FrameworkKind::Vue),
        "svelte" => framework_descriptor(FrameworkKind::Svelte),
        "astro" => framework_descriptor(FrameworkKind::Astro),
        "html" | "htm" => text_descriptor(FileAssociation::Documentation),
        "css" | "scss" | "sass" | "less" | "pcss" => text_descriptor(FileAssociation::Styling),
        "json" | "json5" | "jsonl" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "lock" => {
            text_descriptor(FileAssociation::Configuration)
        }
        "md" | "mdx" | "txt" | "rst" | "adoc" | "org" | "tex" => {
            text_descriptor(FileAssociation::Documentation)
        }
        "csv" | "tsv" | "xml" | "graphql" | "gql" | "sql" => text_descriptor(FileAssociation::Data),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "avif" | "ico" | "bmp" | "tif"
        | "tiff" => text_descriptor(FileAssociation::Image),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" => {
            binary_descriptor(FileAssociation::Archive)
        }
        "ttf" | "otf" | "woff" | "woff2" => binary_descriptor(FileAssociation::Font),
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" => {
            binary_descriptor(FileAssociation::Audio)
        }
        "mp4" | "mov" | "mkv" | "webm" | "avi" => binary_descriptor(FileAssociation::Video),
        "pdf" => binary_descriptor(FileAssociation::Binary),
        "so" | "dylib" | "dll" | "a" | "o" | "wasm" => binary_descriptor(FileAssociation::Binary),
        "exe" => binary_descriptor(FileAssociation::BinaryExecutable),
        _ => return None,
    };

    Some(descriptor)
}

pub fn shebang_descriptor(interpreter: &str) -> Option<FileTypeDescriptor> {
    let normalized = interpreter
        .rsplit('/')
        .next()
        .unwrap_or(interpreter)
        .split_whitespace()
        .next()
        .unwrap_or(interpreter)
        .to_ascii_lowercase();

    let language = match normalized.as_str() {
        "sh" => ProgrammingLanguage::Shell,
        "bash" => ProgrammingLanguage::Bash,
        "zsh" => ProgrammingLanguage::Zsh,
        "fish" => ProgrammingLanguage::Fish,
        "python" | "python3" | "python2" => ProgrammingLanguage::Python,
        "node" | "nodejs" | "deno" | "bun" => ProgrammingLanguage::JavaScript,
        "ruby" => ProgrammingLanguage::Ruby,
        "php" => ProgrammingLanguage::Php,
        "lua" => ProgrammingLanguage::Lua,
        "pwsh" | "powershell" => ProgrammingLanguage::PowerShell,
        _ => return None,
    };

    Some(language_descriptor(language))
}
