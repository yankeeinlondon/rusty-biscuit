# DarkMatter DSL

## Functional Goals for the DarkMatter DSL

Darkmatter is a DSL/Language Specification which sits on top of [CommonMark](https://commonmark.org/) markdown (and [GFM](https://github.github.com/gfm/)) and is focused on providing a compact and understandable vocabulary for the *composition* of content together into a master document.

When a Darkmatter document is parsed it goes through a two step process:

1. Document Prep
2. Core Transforms (including recursive Transclusion)

And at the end of this process the output is a Markdown document (with all Darkmatter DSL operations/directives fully resolved).

### Document Prep DSL Operations

#### 1. Frontmatter Interpolation

Any frontmatter properties defined on a page can be rendered onto the page with the syntax `{{variable}}`.

The details of this feature can be found in the [Frontmatter Interpolation](../design/interpolation.md) spec.

#### 2. Text Replacement

When a page's frontmatter has a `replace` property in the frontmatter it is expected to have a key/value dictionary structure where:

- the *keys* represent text on the page which should be *replaced*
- the *values* represent the replacement text you would like to use

By default this property is not set which effectively turns off this feature. See [Text Replacement](text-replacement.md) specification for more details.

**Note:** **frontmatter** properties like `replace` or `list_expansion` which have a special semantic meaning in **Darkmatter** are referred to as [`darkmatter`](darkmatter-metadata.md) (non-capitalized)


### Core DSL Transforms

> **Note:** whereas the Document Prep stage is always relatively computationally simple and achievable without the cache, this section relies on good [work scheduling](../reference/work-scheduling.md) to ensure that there are as many cache hits as possible.

#### 1. Transclusion

The concept of [Transclusion](https://en.wikipedia.org/wiki/Transclusion) is both simple and equally essential for any document reuse strategy. In essence Transclusion allows a document to be *composed* of various sections which are sourced from external files. This process is meant to be a realtime-*like* relationship such that if **A** *transcludes* **B** to make up it's content then any change to B will be immediately (or near immediately) reflected in **A**.

In Darkmatter this would look something like this:

```md
## My Section

::file ./some-reusable-content.md
```

> **NOTE:** the actual syntax, and various options for it's use will be covered later in the [DSL Syntax](#dsl-syntax) section

#### 2. Summarization (future)

The ability to inject not the external *document* itself but instead a **summary** of an external document is a powerful feature. This feature will leverage an **LLM*** to produce the summarization and would look something like this in **Darkmatter**:

```md
The document **ABC**, when summarized, is a good example of _less is more_:

::summarize ./abc.md
```

#### 3. Consolidation (future)

At times you will have a set of documents which you want to consolidate into a cohesive whole. This may involve restructuring and/or supplementation of certain sections from one document into another. To achieve this we will leverage an **LLM** to integrate the documents together in a meaningful way. In **Darkmatter** leveraging this feature will look something like:

```md
## My Consolidated Content

::consolidate ./abc.md ./def.md
```

#### 4. Topic Extraction (future)

Whereas a **Consolidation** attempts to move the content in the various files provided *in it's entirety* into the master document, a **Topic Consolidation** reviews all the documents provided and looks for information on a specified "topic". That topic is then isolated and the various document's information on the topic are consolidated into prose. In **Darkmatter** this would look something like:

```md
## Linting for Fun and Profit

::topic "linting" ./abc.md ./def.md
```

You may request a review of the topic document by adding in the `--review` flag.

#### 5. Tables

When you use the [[GFM]] extensions to [[CommonMark]] you are given a means for creating tabular layouts which can be handy but often prove to be unwieldy for many use cases. **Darkmatter** provides an additional set of primitives for creating tables both from inline content as well as external data. Variants of both of the following syntaxes are supported:

##### Using Block Directives

~~~md
## Products

::table --with-heading-row
- Product, Price, Description
- Rubber Ducky, $5.99, A yellow rubber ducky for you and your family
- Plasma Lamp, $25.99, A very special light for a very special night
::end-table
```
~~~

##### Using Code Block Syntax
Alternatively you can also use code block syntax if you prefer:

~~~md
## Products

```table --with-heading-row
- Product, Price, Description
- Rubber Ducky, $5.99, A yellow rubber ducky for you and your family
- Plasma Lamp, $25.99, A very special light for a very special night
```
~~~

#### External Content

```md
## Products

::table ./data.csv --with-heading-row
```

#### 6. Charting

Charting helps people visualize data but unfortunately Markdown doesn't provide any out-of-the-box solution for it. **Darkmatter** provides a similar *inline* and *external* means of providing charts. Supported chart types include:

- bar chart
- line chart
- donut/pie chart
- area chart
- bubble chart

An example of this in DarkMatter might be:

```md
## Sales

::bar-chart ./sales-by-region.csv
```

#### 7. Popover

The popover effect -- where some part of the page when hovered over or clicked on, presents additional contextual detail for that underlying content -- is supported in **DarkMatter** as an *inline* element or a *block* element.

##### Inline Popover

~~~md
There I was, there I was, ... in the [jungle](popover:The Jungle is a dangerous place with lots of scary animals).
~~~

##### Block Popover

```md
::popover ./wooly-mammoth.md
The wooly mammoth is a fictitious creature who likes to hang out with the Loque Nes Monster.
::end-popover
```

**Note:** In these examples we provide simple examples that do not represent the full capabilities of this feature. There are several options and derivative formats for **Popovers** which will be covered in part in the [DSL Syntax](#dsl-syntax) section as well as in full detail in the [Darkmatter Popover Specification](./popover.md).

#### 8. List Expansion

In Markdown you can denote a *list item* by using the `*`, `-`, or `+` character which are semantically the same but use the character type chosen in the list's output format. When you're rendering a *flat list* with **DarkMatter** nothing changes, however, when you have a structured list of list items and sub-list items it would be nice to:

1. have the ability expand/compress the sub-items of list items
2. have some sort of semantics on whether the child elements of a list item should *expanded* to start or *compressed* to start

In **DarkMatter** we:

- assign line items with `*` to the *default state*, `-` will default to a *collapsed* state, and `+` will default to an *expanded* state when the page is rendered
- the "default state" is set by the `list_expansion` frontmatter property on a page but will default to `none` if not set; valid options are:
    - `expanded` | `collapsed` which provide dynamic expand/collapse at runtime by converting list items with sub-items into inline-HTML which supports this feature
    - `none` which turns this feature off entirely
- there are more controls and options described in the [Darkmatter List Expansion](../design/list-expansion.md) specification


#### 10. Smart Image

Markdown provides a way to add an image to a page with the `![alt text](./image.png)` based syntax but with a **Darkmatter** document you can go further with "smart images". A smart image is an image that will:

- try to use `avif` and `webp` formats on devices that support it while gracefully falling back to `jpg` or `png` if support is not found
- loads an optimally *sized* variant of the image based on the device's available width so that devices get consistent quality but don't load images that are larger then they can actually display
- can *optionally* work on metadata found in the image in *any* or *none* of the following ways:
    - stripping all metadata
    - strip specified metadata
    - add `data-{META}` tags to the image for metadata found
    - using one or more metadata properties to assign the `alt-text` property for screen readers, etc.

More details can be found in the [Smart Image](../design/smart-image.md) specification document.



### 12. Disclosure Blocks

In HTML we're given the `<details><summary>...</summary>...</details>` primitive which allows us to provide *selective disclosure* meaning that only the **summary*** section is displayed initially but clicking on it expands the disclosed text to include all of the text within the `<details>` block. Of course in normal Markdown -- being that it is a *superset* of HTML -- we can use this HTML to provide selective disclosure but writing that in a markdown document sometimes feels yucky.

In a **Darkmatter** document you can now use the following syntax to express the same disclosure functionality in a more Markdown-like manner:

~~~md
::summary
License: Duesy Deluxe
::details
This contract is a duesy. By signing you give up writes to everything and this library author is now legally
allowed to name your children.
~~~

The `details` definition is terminated/concluded when:

- two blank lines are found back-to-back,
- the end of the file is found,
- or if the user explicitly ends it with the `::end` operative.

### 13. Block Columns

Markdown is great for *notational velocity* (aka, forget the cruft just getting down to writing content). However, it is limited in it's ability to take full advantage of the horizontal space provided. In part this is because there is no way to have multi-columnar output.

**DarkMatter** fixes this limitation by providing the `::columns` and `::break` block elements. Below you'll find a representative example of how you might use it:

~~~md
::columns md: 2, xl: 3

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nulla pretium, eros et condimentum viverra, libero ex dapibus leo, ut placerat magna purus in libero.Curabitur eget eros rhoncus, laoreet enim vel, bibendum ligula. Aliquam molestie tortor nec bibendum sagittis. Suspendisse ut scelerisque eros. Phasellus auctor tellus vitae molestie aliquam. Morbi in dolor at felis accumsan facilisis. Aenean a mi est. Praesent eu dui placerat, cursus turpis vitae, porta turpis. Phasellus fringilla convallis ipsum non finibus. Vivamus nec tortor leo. Proin vehicula non erat sit amet consequat.

::break

Mauris dui lacus, dapibus eget diam nec, malesuada tincidunt purus. Proin varius, felis vel tristique pretium, mauris nibh vestibulum tellus, id volutpat quam odio vitae tortor.Sed neque mauris, placerat sed commodo eget, blandit quis massa. Suspendisse a sem sodales, imperdiet ipsum vel, scelerisque quam. Nam ex lorem, blandit sit amet rhoncus sed, varius id tellus. Duis tincidunt lorem eget risus semper, sed rhoncus erat vehicula. Ut mollis leo quis enim pellentesque imperdiet. Phasellus et rutrum lectus.Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae; Praesent mattis felis condimentum, gravida lacus id, pretium neque. Nullam vitae ante non elit elementum finibus ut ut est. Sed pulvinar nisl quis pellentesque dignissim. Ut ultricies pretium sapien sed pellentesque.

::break

In fringilla nibh metus, id rhoncus erat tincidunt ac. Etiam ullamcorper id est eu interdum. Integer sit amet dolor eget diam dapibus porta. Etiam interdum erat id dui blandit, sed dapibus lacus imperdiet. Morbi interdum diam lectus, ac sagittis mauris facilisis et. Maecenas vel rutrum est. Donec in convallis tellus, quis varius lorem. Duis tempus mattis massa a molestie. Mauris lectus purus, sodales non commodo et, venenatis at felis. Duis odio magna, viverra vestibulum vestibulum nec, lacinia ut ante.

::end
~~~

In this example:

- smaller devices (e.g., `xs` and `sm`) will display this text in a single column
- mid-sized devices (eg., `md` and `lg`) will display this text in two columns
- very large display devices (e.g., `xl`) will display this text in three columns

See the [Block Columns](./block-columns.md) specification for more details.

### 14. Audio Content

If a user wants to include audio content in a document, they can use the `::audio` directive to embed an HTML5 audio player.

**Syntax:**

```md
::audio <source> [name]
```

**Parameters:**

- `<source>` - Path to the audio file (MP3 or WAV format). Can be:
    - Relative path: `./audio/podcast.mp3`
    - Absolute path: `/Users/name/music/song.wav`
    - Path with spaces: `"./my audio/file.mp3"` (use quotes)
- `[name]` - Optional custom display name (in quotes if it contains spaces)

**Examples:**

```md
# Simple audio embed
::audio ./podcast.mp3

# With custom name
::audio ./episode-42.mp3 "Episode 42: The Answer"

# Path with spaces
::audio "./My Music/favorite-song.wav"

# Both quoted
::audio "./audio files/interview.mp3" "Interview with Jane Doe"
```

**Features:**

- **Metadata Extraction:** Automatically extracts duration, bitrate, sample rate, and ID3 tags (title, artist, album)
- **Caching:** Metadata is cached by content hash to avoid reprocessing unchanged files
- **Dual Rendering Modes:**
    - **File Reference** (default): Copies audio file to output directory with hash-based filename
    - **Inline Mode** (`--inline` flag): Encodes audio as base64 data URI for portable HTML
- **Display Priority:** Shows custom name → ID3 title → filename

**Output:**

The directive generates an HTML5 audio player:

```html
<div class="audio-player">
  <audio controls preload="metadata">
    <source src="audio/hash.mp3" type="audio/mpeg">
    Your browser does not support the audio element.
  </audio>
  <div class="audio-info">
    <span class="audio-name">Episode 42</span>
    <span class="audio-duration">12:34</span>
  </div>
</div>
```

**Styling:**

Default CSS is provided in `lib/assets/audio-player.css`. Override the `.audio-player` class to customize appearance.

**Supported Formats:**

- MP3 (audio/mpeg)
- WAV (audio/wav)

For implementation details, see [Audio Player Design](../design/audio-player.md).


### 15. YouTube Video Embedding

Embed YouTube videos directly in your documents with an enhanced player that includes maximize/modal functionality.

**Syntax:**

```md
::youtube <video-reference> [width]
```

**Parameters:**

- `<video-reference>` - YouTube video URL or video ID. Supported formats:
    - Video ID: `dQw4w9WgXcQ` (11-character alphanumeric string with `-` and `_`)
    - Watch URL: `https://www.youtube.com/watch?v=dQw4w9WgXcQ`
    - Short URL: `https://youtu.be/dQw4w9WgXcQ`
    - Embed URL: `https://www.youtube.com/embed/dQw4w9WgXcQ`
    - /v/ URL: `https://youtube.com/v/dQw4w9WgXcQ`
- `[width]` - Optional width specification (default: `512px`):
    - Pixels: `800px`
    - Rems: `32rem`, `32.5rem`
    - Percentage: `80%` (0-100 range)

**Examples:**

```md
# Simple video embed with default width (512px)
::youtube dQw4w9WgXcQ

# With custom pixel width
::youtube https://www.youtube.com/watch?v=dQw4w9WgXcQ 800px

# With rem width
::youtube https://youtu.be/jNQXAC9IVRw 40rem

# With percentage width
::youtube 9bZkp7q19f0 90%

# URL with query parameters
::youtube https://www.youtube.com/watch?v=dQw4w9WgXcQ&feature=share
```

**Features:**

- **16:9 Aspect Ratio:** Maintains proper video dimensions in default and modal states
- **Maximize Button:** Click the maximize button (top-right) to view in modal mode
- **Modal View:** 95vw width, centered viewport, with backdrop blur effect
- **Keyboard Navigation:** Press `Escape` to exit modal mode
- **Backdrop Dismiss:** Click the backdrop to exit modal mode
- **Play State Preservation:** Video continues playing when transitioning between states
- **YouTube IFrame API:** Full API integration for player control
- **Self-Contained Output:** CSS and JavaScript are inlined for portability
- **Asset Deduplication:** Multiple embeds share single CSS/JS assets
- **Accessibility:** ARIA labels and keyboard navigation support

**Output:**

The directive generates self-contained HTML with:

```html
<div class="dm-youtube-container" data-video-id="dQw4w9WgXcQ" data-width="512px">
  <div class="dm-youtube-wrapper">
    <iframe class="dm-youtube-player"
            src="https://www.youtube.com/embed/dQw4w9WgXcQ?enablejsapi=1"
            frameborder="0"
            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
            allowfullscreen
            aria-label="YouTube video player">
    </iframe>
    <button class="dm-youtube-maximize" aria-label="Maximize video">
      <svg><!-- maximize icon --></svg>
    </button>
  </div>
</div>
<div class="dm-youtube-backdrop" style="display: none;"></div>

<!-- Included once per document -->
<style id="dm-youtube">/* inline CSS */</style>
<script id="dm-youtube">/* inline JavaScript */</script>
```

**Styling:**

The default styles provide:

- Responsive container with custom width
- 16:9 aspect ratio padding
- Smooth transitions between states
- Backdrop blur effect (8px)
- Maximize button hover/focus states

Override CSS classes to customize appearance:

- `.dm-youtube-container` - Main container
- `.dm-youtube-wrapper` - 16:9 wrapper
- `.dm-youtube-player` - iframe element
- `.dm-youtube-maximize` - Maximize button
- `.dm-youtube-backdrop` - Modal backdrop
- `.dm-youtube-container.modal` - Modal state

**Error Handling:**

Invalid inputs produce user-friendly error messages:

```md
# Invalid video ID (wrong length)
::youtube invalid-id
# Error: Could not extract video ID from 'invalid-id'.
# Supported formats: youtube.com/watch?v=ID, youtu.be/ID, or 11-character ID

# Invalid width (>100%)
::youtube dQw4w9WgXcQ 150%
# Error: Invalid percentage '150'. Must be 0-100%

# Non-YouTube URL
::youtube https://vimeo.com/123456
# Error: Could not extract video ID from 'https://vimeo.com/123456'.
# Supported formats: youtube.com/watch?v=ID, youtu.be/ID, or 11-character ID
```

**Browser Compatibility:**

- Modern browsers with ES6+ support
- Backdrop blur requires browser support (graceful degradation)
- YouTube IFrame API requires JavaScript enabled

**Security:**

- Video IDs validated with strict regex (alphanumeric + `-` and `_` only)
- URL extraction limited to youtube.com and youtu.be domains
- HTTPS enforced for all YouTube embed URLs
- No dynamic code execution (all JavaScript is static)

**CSP Considerations:**

If your site uses Content Security Policy (CSP), you may need to adjust headers to allow:

- `frame-src https://www.youtube.com`
- `script-src https://www.youtube.com` (for IFrame API)

For more implementation details, see [YouTube Embedding Design](../design/youtube-embedding.md).

### 16. Vector Embeddings

To achieve a semantic search, the content must have a vector embedding. Currently we do not have any semantic search features but that will be added later.

When a page has the frontmatter property `embeddings` set to true then the page's content will be used to create an embedding vector and will be stored into the database.

## Caching Semantics

Before we jump into explicit syntax, let's discuss two related topics: **proximity** and **timing:**

- When all documents live in a shared filesystem their proximity to one another is consider to be **close** which implies that transclusion can be done very performantly (even without a cache)
- By contrast, when documents are referenced over a network the proximity increases and often becomes not only much slower but also less reliable. In most cases this environment will necessitate some form of caching to achieve reasonable performance and reliability.
- As most of you already know, if you can reduce a system's insistence on being "real-time" to "eventually consistent" then the system will scale much better and in most cases your performance goals will be much more easily met.
- Sometimes -- but not very often -- we **really** need realtime transclusion but often we've just fallen in love with the idea and making some strategic compromises toward an eventually consistent target is a wise choice
- With this in mind and to help you make wise decisions, transclusion in composition's Darkmatter syntax uses a good set of *defaults* for just how "real time" the transclusion is:
    - Real Time Defaults
        - Local markdown file references will always be real-time when the `compose()` function is called. When using `watch()`, local markdown files will be near realtime (eventually consistent within a second or two of an external reference being saved)
        - Local *synchronous* HTML files (aka, content of the page is immediately available and not dependent on an embedded Javascript rendering of the content) will also always be real-time when the `compose()` function is called. Using the `watch()` command will have a slight delay but only marginally longer than the like-for-like markdown references.
        - Locally asynchronous HTML files (aka, content which includes a Javascript function which is responsible for rendering the content body) maintains real-time `compose()` compatibility but with the chance of errors being raised rather than the content expected.
    - Eventually Consistent Defaults
        - Any non-local content over **HTTP** will have a caching layer.
            - By default it will be cached for **one day** before another attempt is made to resolve this from the actual source
                - Both inline and block syntaxes provide a way for a user to override the default interval of one day
            - A hash based check for change detection is performed as soon as the cache window has expired:
                - if the content is non-reachable for any reason (local machine is offline, the remote location returns an error code other than **404**) then the cache will be used but a warning will be issued and the cache status will remain in a "stale" state
                - if the remote server returns a **404** then an empty string will interpolated into the calling document but a warning will be provided to the caller to react to. The cache will then be reset to not check for another day.

     > **Note:** any resource which ends in `!` indicates that the resource is NOT ALLOWED to be empty and will halt execution with an error when this condition is met.

     > **Note:** any resource which ends in `?` indicates that the resource is allowed to be absent and that no warning (or error) should be raised when this condition is encountered.

                -

## DSL Syntax





## One Shot Transclusion


## Required or Optional

## Error Handling

### Missing Content

- empty space, warning, or error?


## Frontmatter Interpolation and Inheritance

