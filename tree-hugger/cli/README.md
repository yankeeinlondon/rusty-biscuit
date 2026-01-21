# Tree Hugger CLI

Tree Hugger provides the `hug` CLI for exploring symbols, imports, and exports across multiple
languages.

## Usage

```bash
hug symbols "src/**/*.rs"
hug functions --language rust "src/**/*.rs"
hug imports --format json "tests/fixtures/**/*.js"
```

## Commands

- `functions`
- `types`
- `symbols`
- `exports`
- `imports`

## Options

- `--language <LANG>` - override language detection
- `--ignore <GLOB>` - exclude files
- `--format <pretty|json>` - output formatting
