# Terminal Graphics Protocols

Modern terminals support inline image display through several protocols. Each has different capabilities, compatibility, and use cases.

## Protocol Comparison

| Protocol | Terminals | Color Depth | Animation | Transmission |
|----------|-----------|-------------|-----------|--------------|
| Sixel | xterm, mlterm, WezTerm, foot, iTerm2 | 256 colors | Yes | Base64 escape |
| Kitty | Kitty, WezTerm | Full RGB | Yes | Chunked base64 |
| iTerm2 | iTerm2, WezTerm, mintty | Full RGB | Yes | Base64 escape |

## Sixel Graphics

Sixel is a bitmap graphics format from DEC terminals (1984). Uses escape sequences to transmit pixel data as vertical six-pixel columns.

### Format

```
DCS q [parameters] SIXEL-DATA ST
DCS = \x1bP (Device Control String)
ST = \x1b\\ (String Terminator)
```

### Basic Structure

```
\x1bPq           Start Sixel
#0;2;R;G;B       Define color 0 as RGB (0-100 scale)
#0              Select color 0
!Nc              Repeat character c N times
-                Graphics newline (next sixel row)
\x1b\\           End Sixel
```

### Sixel Character Encoding

Each Sixel character represents a column of 6 pixels. Character value = 63 + (binary pattern of 6 pixels).

```
Bit 0 (1)  = top pixel
Bit 1 (2)  = second pixel
Bit 2 (4)  = third pixel
Bit 3 (8)  = fourth pixel
Bit 4 (16) = fifth pixel
Bit 5 (32) = bottom pixel

Character '?' (63) = all pixels off
Character '~' (126) = all pixels on
```

### Sixel Example

Draw a red 2x6 pixel rectangle:
```
\x1bPq#0;2;100;0;0#0~~\x1b\\
```

### Converting Images to Sixel

Using ImageMagick:
```bash
convert image.png -colors 256 sixel:-
```

Using libsixel:
```bash
img2sixel image.png
```

### Detection

Query Device Attributes (DA1):
```
\x1b[c
```

Response containing `4` indicates Sixel support.

## Kitty Graphics Protocol

Modern protocol designed for high-performance image display with full RGB support.

### Basic Format

```
\x1b_Gkey=value,key=value;payload\x1b\\
```

### Control Keys

| Key | Meaning | Values |
|-----|---------|--------|
| a | Action | t=transmit, T=transmit+display, p=put, d=delete |
| f | Format | 24=RGB, 32=RGBA, 100=PNG |
| t | Transmission | d=direct, f=file, t=temp file, s=shared memory |
| s | Source width | pixels |
| v | Source height | pixels |
| c | Columns | cell columns to display |
| r | Rows | cell rows to display |
| m | More data | 0=last chunk, 1=more chunks |
| i | Image ID | unique identifier |
| q | Quiet | 1=suppress OK response, 2=suppress errors |

### Transmit PNG Directly

```typescript
const displayImage = (pngData: Buffer) => {
  const b64 = pngData.toString('base64');
  const chunks = b64.match(/.{1,4096}/g) || [];

  chunks.forEach((chunk, i) => {
    const isLast = i === chunks.length - 1;
    const ctrl = i === 0
      ? `a=T,f=100,q=2,m=${isLast ? 0 : 1}`
      : `m=${isLast ? 0 : 1}`;
    process.stdout.write(`\x1b_G${ctrl};${chunk}\x1b\\`);
  });
};
```

### Display from File

```
\x1b_Ga=T,f=100,t=f;/path/to/image.png\x1b\\
```

### Delete Images

```
\x1b_Ga=d\x1b\\           Delete all images
\x1b_Ga=d,i=123\x1b\\     Delete image with ID 123
\x1b_Ga=d,d=c\x1b\\       Delete images in current cell
```

### Rust Example

```rust
use base64::{Engine, engine::general_purpose::STANDARD};

fn display_kitty_image(png_data: &[u8]) {
    let b64 = STANDARD.encode(png_data);
    let chunks: Vec<&str> = b64.as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        let ctrl = if i == 0 {
            format!("a=T,f=100,q=2,m={}", if is_last { 0 } else { 1 })
        } else {
            format!("m={}", if is_last { 0 } else { 1 })
        };
        print!("\x1b_G{};{}\x1b\\", ctrl, chunk);
    }
}
```

## iTerm2 Inline Images Protocol

Uses OSC 1337 for image display with flexible sizing options.

### Basic Format

```
\x1b]1337;File=params:base64-data\x07
```

Or with ST terminator:
```
\x1b]1337;File=params:base64-data\x1b\\
```

### Parameters

| Param | Meaning | Example |
|-------|---------|---------|
| name | Filename (base64) | `name=aW1hZ2UucG5n` |
| size | File size in bytes | `size=12345` |
| width | Display width | `width=80px`, `width=50%`, `width=auto` |
| height | Display height | `height=24`, `height=auto` |
| preserveAspectRatio | Keep aspect ratio | `preserveAspectRatio=1` |
| inline | Display inline | `inline=1` |

### TypeScript Example

```typescript
const displayITerm2Image = (data: Buffer, options: {
  name?: string;
  width?: string;
  height?: string;
  preserveAspectRatio?: boolean;
  inline?: boolean;
} = {}) => {
  const params: string[] = [];

  if (options.name) {
    params.push(`name=${Buffer.from(options.name).toString('base64')}`);
  }
  params.push(`size=${data.length}`);
  if (options.width) params.push(`width=${options.width}`);
  if (options.height) params.push(`height=${options.height}`);
  if (options.preserveAspectRatio !== false) {
    params.push('preserveAspectRatio=1');
  }
  params.push(`inline=${options.inline !== false ? 1 : 0}`);

  const b64 = data.toString('base64');
  process.stdout.write(`\x1b]1337;File=${params.join(';')}:${b64}\x07`);
};
```

### Size Specifications

Width and height accept:
- `N` - N character cells
- `Npx` - N pixels
- `N%` - N percent of session width/height
- `auto` - Natural size

Examples:
```
width=80           80 character cells wide
width=200px        200 pixels wide
width=50%          50% of terminal width
height=auto        Natural height
```

## Protocol Detection

### Feature Detection Function

```typescript
interface TerminalCapabilities {
  trueColor: boolean;
  kittyGraphics: boolean;
  iterm2Images: boolean;
  sixel: boolean;
  hyperlinks: boolean;
}

function detectCapabilities(): TerminalCapabilities {
  const term = process.env.TERM || '';
  const termProgram = process.env.TERM_PROGRAM || '';
  const colorterm = process.env.COLORTERM || '';

  return {
    trueColor: colorterm === 'truecolor' || colorterm === '24bit',
    kittyGraphics: term === 'xterm-kitty' || termProgram === 'WezTerm',
    iterm2Images: termProgram === 'iTerm.app' || termProgram === 'WezTerm',
    sixel: false, // Requires DA1 query
    hyperlinks: true, // Widely supported, safe to use
  };
}
```

### Sixel Detection (Async)

```typescript
import * as readline from 'readline';

async function detectSixel(): Promise<boolean> {
  return new Promise((resolve) => {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    let response = '';
    const timeout = setTimeout(() => {
      rl.close();
      resolve(false);
    }, 1000);

    process.stdin.setRawMode(true);
    process.stdin.once('data', (data) => {
      clearTimeout(timeout);
      response = data.toString();
      process.stdin.setRawMode(false);
      rl.close();
      // Check for '4' in DA1 response
      resolve(response.includes('4'));
    });

    process.stdout.write('\x1b[c');
  });
}
```

## Best Practices

1. **Always detect terminal capabilities** before using graphics protocols
2. **Provide fallbacks** - ASCII art, descriptions, or skip images
3. **Chunk large images** to avoid buffer issues
4. **Use appropriate protocol** for target terminal
5. **Consider tmux/screen** - graphics may not work inside multiplexers
6. **Test across terminals** - behavior varies significantly
