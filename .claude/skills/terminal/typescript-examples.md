# TypeScript Terminal Examples

Practical TypeScript/Node.js examples for terminal manipulation.

## Basic ANSI Styling

### Simple Style Function

```typescript
const ESC = '\x1b';
const CSI = `${ESC}[`;

const style = (text: string, ...codes: number[]): string =>
  `${CSI}${codes.join(';')}m${text}${CSI}0m`;

// Usage
console.log(style('Bold Red', 1, 31));
console.log(style('Underlined Blue', 4, 34));
console.log(style('Bright Cyan Background', 106));
```

### Chainable Style Builder

```typescript
class Styler {
  private codes: number[] = [];

  bold(): this { this.codes.push(1); return this; }
  dim(): this { this.codes.push(2); return this; }
  italic(): this { this.codes.push(3); return this; }
  underline(): this { this.codes.push(4); return this; }
  inverse(): this { this.codes.push(7); return this; }
  strikethrough(): this { this.codes.push(9); return this; }

  fg(color: number): this { this.codes.push(38, 5, color); return this; }
  bg(color: number): this { this.codes.push(48, 5, color); return this; }

  rgb(r: number, g: number, b: number): this {
    this.codes.push(38, 2, r, g, b);
    return this;
  }

  bgRgb(r: number, g: number, b: number): this {
    this.codes.push(48, 2, r, g, b);
    return this;
  }

  apply(text: string): string {
    if (this.codes.length === 0) return text;
    return `\x1b[${this.codes.join(';')}m${text}\x1b[0m`;
  }
}

const s = () => new Styler();

// Usage
console.log(s().bold().rgb(255, 100, 50).apply('Orange Bold'));
console.log(s().underline().fg(45).apply('Teal Underlined'));
```

## Progress Bar

### Basic Progress Bar

```typescript
interface ProgressOptions {
  width?: number;
  complete?: string;
  incomplete?: string;
  showPercent?: boolean;
  showETA?: boolean;
}

function createProgressBar(options: ProgressOptions = {}) {
  const {
    width = 40,
    complete = '\u2588',
    incomplete = '\u2591',
    showPercent = true,
    showETA = false,
  } = options;

  let startTime: number | null = null;

  return function update(current: number, total: number) {
    if (startTime === null) startTime = Date.now();

    const percent = Math.min(100, Math.round((current / total) * 100));
    const filled = Math.round(width * percent / 100);
    const bar = complete.repeat(filled) + incomplete.repeat(width - filled);

    let output = `\r${bar}`;

    if (showPercent) {
      output += ` ${percent.toString().padStart(3)}%`;
    }

    if (showETA && current > 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      const rate = current / elapsed;
      const remaining = (total - current) / rate;
      output += ` ETA: ${formatTime(remaining)}`;
    }

    process.stdout.write(output);

    if (current >= total) {
      process.stdout.write('\n');
    }
  };
}

function formatTime(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  return `${Math.round(seconds / 3600)}h`;
}

// Usage
const progress = createProgressBar({ showETA: true });
let i = 0;
const interval = setInterval(() => {
  progress(i, 100);
  if (++i > 100) clearInterval(interval);
}, 50);
```

### Colored Progress Bar

```typescript
function coloredProgressBar(
  percent: number,
  width = 50,
  label = ''
): string {
  const filled = Math.round(width * percent / 100);

  // Color gradient: red -> yellow -> green
  const r = percent < 50 ? 255 : Math.round(255 - (percent - 50) * 5.1);
  const g = percent < 50 ? Math.round(percent * 5.1) : 255;

  const bar = '\u2588'.repeat(filled);
  const empty = '\u2591'.repeat(width - filled);

  return `${label}\x1b[38;2;${r};${g};0m${bar}\x1b[90m${empty}\x1b[0m ${percent}%`;
}

// Usage
for (let p = 0; p <= 100; p += 10) {
  console.log(coloredProgressBar(p, 40, 'Download: '));
}
```

## Spinner

```typescript
const spinnerFrames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

function createSpinner(message: string) {
  let frame = 0;
  let interval: NodeJS.Timeout | null = null;

  return {
    start() {
      process.stdout.write('\x1b[?25l'); // Hide cursor
      interval = setInterval(() => {
        process.stdout.write(`\r${spinnerFrames[frame]} ${message}`);
        frame = (frame + 1) % spinnerFrames.length;
      }, 80);
    },

    stop(finalMessage?: string) {
      if (interval) clearInterval(interval);
      process.stdout.write('\x1b[?25h'); // Show cursor
      process.stdout.write(`\r\x1b[2K`); // Clear line
      if (finalMessage) console.log(finalMessage);
    },

    success(msg: string) {
      this.stop(`\x1b[32m✓\x1b[0m ${msg}`);
    },

    fail(msg: string) {
      this.stop(`\x1b[31m✗\x1b[0m ${msg}`);
    },
  };
}

// Usage
const spinner = createSpinner('Loading...');
spinner.start();
setTimeout(() => spinner.success('Done!'), 2000);
```

## Hyperlinks

```typescript
function hyperlink(text: string, url: string, id?: string): string {
  const params = id ? `id=${id}` : '';
  return `\x1b]8;${params};${url}\x1b\\${text}\x1b]8;;\x1b\\`;
}

// Usage
console.log(`Check out ${hyperlink('my website', 'https://example.com')}`);

// Multi-line link with same ID
const linkId = 'doc-link';
console.log(hyperlink('Documentation', 'https://docs.example.com', linkId));
console.log(hyperlink('(click here)', 'https://docs.example.com', linkId));
```

## Table Rendering

```typescript
interface TableOptions {
  headers?: string[];
  padding?: number;
  headerStyle?: (s: string) => string;
}

function renderTable(rows: string[][], options: TableOptions = {}): string {
  const {
    headers,
    padding = 1,
    headerStyle = (s) => `\x1b[1m${s}\x1b[0m`,
  } = options;

  const allRows = headers ? [headers, ...rows] : rows;

  // Calculate column widths
  const colWidths = allRows[0].map((_, i) =>
    Math.max(...allRows.map(row => (row[i] || '').length))
  );

  const pad = ' '.repeat(padding);
  const separator = colWidths.map(w => '-'.repeat(w + padding * 2)).join('+');

  const formatRow = (row: string[], isHeader = false): string => {
    const cells = row.map((cell, i) =>
      `${pad}${(cell || '').padEnd(colWidths[i])}${pad}`
    );
    const content = cells.join('|');
    return isHeader ? headerStyle(content) : content;
  };

  const lines: string[] = [];

  if (headers) {
    lines.push(formatRow(headers, true));
    lines.push(separator);
  }

  for (const row of rows) {
    lines.push(formatRow(row));
  }

  return lines.join('\n');
}

// Usage
console.log(renderTable(
  [
    ['Alice', '25', 'Engineer'],
    ['Bob', '30', 'Designer'],
    ['Carol', '28', 'Manager'],
  ],
  { headers: ['Name', 'Age', 'Role'] }
));
```

## Interactive Prompt

```typescript
import * as readline from 'readline';

async function prompt(question: string): Promise<string> {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  return new Promise((resolve) => {
    rl.question(`\x1b[36m?\x1b[0m ${question} `, (answer) => {
      rl.close();
      resolve(answer);
    });
  });
}

async function confirm(question: string): Promise<boolean> {
  const answer = await prompt(`${question} (y/n)`);
  return answer.toLowerCase() === 'y' || answer.toLowerCase() === 'yes';
}

async function select<T extends string>(
  question: string,
  choices: T[]
): Promise<T> {
  console.log(`\x1b[36m?\x1b[0m ${question}`);
  choices.forEach((choice, i) => {
    console.log(`  ${i + 1}) ${choice}`);
  });

  const answer = await prompt('Enter number:');
  const index = parseInt(answer, 10) - 1;

  if (index >= 0 && index < choices.length) {
    return choices[index];
  }

  console.log('\x1b[31mInvalid selection\x1b[0m');
  return select(question, choices);
}

// Usage
async function main() {
  const name = await prompt('What is your name?');
  const lang = await select('Favorite language?', ['TypeScript', 'Rust', 'Python']);
  const confirmed = await confirm('Continue?');
  console.log({ name, lang, confirmed });
}
```

## Box Drawing

```typescript
const box = {
  topLeft: '┌',
  topRight: '┐',
  bottomLeft: '└',
  bottomRight: '┘',
  horizontal: '─',
  vertical: '│',
};

function drawBox(content: string, title?: string): string {
  const lines = content.split('\n');
  const width = Math.max(...lines.map(l => l.length), (title?.length || 0) + 2);

  const top = title
    ? `${box.topLeft}${box.horizontal} ${title} ${box.horizontal.repeat(width - title.length - 2)}${box.topRight}`
    : `${box.topLeft}${box.horizontal.repeat(width + 2)}${box.topRight}`;

  const bottom = `${box.bottomLeft}${box.horizontal.repeat(width + 2)}${box.bottomRight}`;

  const body = lines.map(line =>
    `${box.vertical} ${line.padEnd(width)} ${box.vertical}`
  );

  return [top, ...body, bottom].join('\n');
}

// Usage
console.log(drawBox('Hello, World!\nThis is a box.', 'Message'));
```

## Terminal Size

```typescript
function getTerminalSize(): { columns: number; rows: number } {
  return {
    columns: process.stdout.columns || 80,
    rows: process.stdout.rows || 24,
  };
}

// Listen for resize
process.stdout.on('resize', () => {
  const { columns, rows } = getTerminalSize();
  console.log(`Terminal resized to ${columns}x${rows}`);
});
```

## Alternate Screen Buffer

```typescript
function withAlternateScreen<T>(fn: () => T): T {
  process.stdout.write('\x1b[?1049h'); // Enter alternate screen
  process.stdout.write('\x1b[H');      // Move cursor to home

  try {
    return fn();
  } finally {
    process.stdout.write('\x1b[?1049l'); // Exit alternate screen
  }
}

// Usage - fullscreen app that restores original content on exit
withAlternateScreen(() => {
  console.log('This is in the alternate screen buffer');
  // Original terminal content is preserved and restored when done
});
```
