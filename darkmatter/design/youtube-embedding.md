# Youtube Embeddings

## Functional Goal

It is common to want to refer to a video on YouTube within a document but just providing a link looses a lot of context and is very non-visual. In a DarkMatter document we would like to convert references to YouTube video into an embedded player.

## Syntax

To embed a video on YouTube in a document we will use the syntax:

```md
::youtube <ref> [width]
```

Where:

- `<ref>` is either the full share link to the video or it is just the `id` of the video, this will be resolved into an embedded player inside the rendered document.
- the second parameter `[width]` is optional but allows a user to express in pixels, rems, or percentages what the width of the video embedding should be
    - when no width is specified the width will be set to 512px

## UI

- we should start by using the YouTube iframe Embeds
- however it would be nice to have a maximize button which can be pressed to animate the embed to become a modal dialog that takes up 95% of the available viewport width (and the appropriate amount of height based on aspect ratio)
    - the rest of the page behind the modal dialog should be subtly blurred
    - pressing escape or clicking anywhere outside the dialog should return the embedded player to it's previous position and size
    - this returning to normal size should also be animated to help the user understand the interaction better
    - the play/pause setting of the video should not change when zooming in or out.

## Technical Design

### Parsing Phase

When the markdown parser encounters a `::youtube` directive:

1. **Pattern Recognition**: Detect `::youtube` as a block-level directive during the pulldown-cmark event stream processing
2. **Parameter Extraction**:
   - First parameter: video reference (URL or ID)
   - Second parameter (optional): width specification
3. **Video ID Resolution**: Extract the video ID from various YouTube URL formats:
   - `https://www.youtube.com/watch?v={ID}`
   - `https://youtu.be/{ID}`
   - `https://www.youtube.com/embed/{ID}`
   - `https://www.youtube.com/v/{ID}`
   - Raw ID: `{ID}` (11-character alphanumeric string with hyphens/underscores)
4. **Width Parsing**: Parse width parameter supporting:
   - Pixels: `512px`, `800px`
   - Rems: `32rem`, `40rem`
   - Percentages: `80%`, `100%`
   - Default to `512px` if not specified

### Rendering Phase

The renderer generates self-contained HTML with inline CSS and JavaScript:

#### HTML Structure

```html
<div class="dm-youtube-container" data-video-id="{ID}" data-width="{WIDTH}">
  <div class="dm-youtube-wrapper">
    <iframe
      class="dm-youtube-player"
      src="https://www.youtube.com/embed/{ID}?enablejsapi=1"
      frameborder="0"
      allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
      allowfullscreen>
    </iframe>
    <button class="dm-youtube-maximize" aria-label="Maximize video">
      <svg><!-- maximize icon --></svg>
    </button>
  </div>
</div>
<div class="dm-youtube-backdrop" style="display: none;"></div>
```

#### CSS Implementation

**Default State:**

- Container width: user-specified or 512px default
- Aspect ratio: 16:9 maintained via padding-bottom technique
- Maximize button: positioned absolute in top-right corner
- z-index layering for button overlay

**Modal State:**

- Container: fixed positioning, 95vw width, centered viewport
- Height: calculated from width to maintain 16:9 aspect ratio
- Backdrop: full viewport with `backdrop-filter: blur(8px)` and semi-transparent overlay
- Transitions: `transform 300ms ease-in-out` for smooth animation

**Transition Details:**

- Store original position/size in data attributes before maximizing
- Animate using CSS transforms for better performance
- Use `will-change: transform` optimization hint

#### JavaScript Implementation

**Event Handlers:**

1. **Maximize Button Click**:
   - Store current scroll position and container bounds
   - Add modal class to container
   - Show backdrop with fade-in
   - Focus trap within modal for accessibility

2. **Minimize Triggers**:
   - Escape key press
   - Backdrop click
   - Maximize button click (toggle behavior)
   - Actions: remove modal class, hide backdrop, restore scroll position

3. **Play State Preservation**:
   - Use YouTube IFrame API (`enablejsapi=1` parameter)
   - Query player state before transition
   - Maintain play/pause state through DOM changes

**API Integration:**

```javascript
// Load YouTube IFrame API
const tag = document.createElement('script');
tag.src = "https://www.youtube.com/iframe_api";

// Track player instances for state management
const players = new Map();
window.onYouTubeIframeAPIReady = function() {
  document.querySelectorAll('.dm-youtube-player').forEach(iframe => {
    const player = new YT.Player(iframe, {
      events: {
        'onStateChange': onPlayerStateChange
      }
    });
    players.set(iframe.dataset.videoId, player);
  });
};
```

### Implementation in Rust

**Parser Extension** (`lib/src/parser/directives/youtube.rs`):

```rust
pub struct YouTubeDirective {
    video_id: String,
    width: Option<String>,
}

impl YouTubeDirective {
    pub fn parse(params: &str) -> Result<Self, ParseError> {
        // Extract video ID using regex
        // Parse optional width parameter
        // Validate video ID format (11 chars, alphanumeric + - _)
    }

    fn extract_video_id(reference: &str) -> Result<String, ParseError> {
        // Regex patterns for various YouTube URL formats
        // Return normalized video ID
    }
}
```

**Renderer** (`lib/src/render/youtube.rs`):

```rust
pub fn render_youtube_embed(directive: &YouTubeDirective) -> String {
    // Generate HTML structure with inline CSS/JS
    // Use const strings for base CSS/JS to avoid duplication
    // Interpolate video ID and width
}
```

**CSS/JS Asset Management**:

- Base CSS/JS included once per document (deduplicated during toHTML phase)
- Per-embed HTML generated for each `::youtube` directive
- Inline everything for self-contained output

### Error Handling

| Error Condition | Behavior |
|----------------|----------|
| Invalid video ID format | Error with suggestion to check URL format |
| Empty/missing video reference | Error: "Video reference required" |
| Invalid width format | Error: "Width must be pixels, rems, or percentage" |
| Network unavailable (optional validation) | Warning: "Could not validate video ID" |

### Caching Considerations

- **No caching needed**: YouTube embeds are pure transformations (URL → HTML)
- **Optional validation**: Could check if video exists via oEmbed API, but not required for MVP
- **Asset deduplication**: CSS/JS included once per document, tracked in render state

### References

- [YouTube Player API Reference for iframe Embeds](https://developers.google.com/youtube/iframe_api_reference)
- [YouTube oEmbed API](https://oembed.com/) (optional validation)
- [MDN: aspect-ratio CSS](https://developer.mozilla.org/en-US/docs/Web/CSS/aspect-ratio)
