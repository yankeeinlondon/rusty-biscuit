# Modern Popovers in the Browser

# Building a Modern Popover System: A Deep Dive

The Popover API, combined with modern CSS features like Anchor Positioning, provides a powerful, native way to build sophisticated popover systems without heavy JavaScript libraries. This guide covers everything you need to know to build production-ready popovers.

## Browser Support & Baseline Status

The Popover API reached **Baseline Widely Available** in April 2025, meaning it works in Chrome, Firefox, Safari, and Edge. CSS Anchor Positioning is also widely supported (Chrome 125+, Firefox 147+, Safari 26+).

## Core Concepts

### What is a Popover?

A popover is a non-modal overlay that displays content on top of other page content. Unlike `<dialog>` elements (which are modal), popovers allow interaction with the rest of the page while shown.

### Popover States

There are three popover states, each with different behaviors:

| State | Light Dismiss | Multiple Open | Use Case |
|-------|--------------|---------------|-----------|
| `auto` (default) | ✅ Yes | ❌ No (except nested) | Menus, teaching UI, status messages |
| `manual` | ❌ No | ✅ Yes | Independent panels, persistent UI |
| `hint` | ✅ Yes | ✅ Yes (closes other hints) | Tooltips alongside UI popovers |

## 1. Declarative Popovers (HTML-Only)

The simplest way to create a popover:

```html
<button popovertarget="mypopover">Toggle Popover</button>
<div id="mypopover" popover>
  <h3>Popover Content</h3>
  <p>This is a popover!</p>
</div>
```

### Control Actions

You can specify the action with `popovertargetaction`:

```html
<button popovertarget="mypopover" popovertargetaction="show">
  Show
</button>
<button popovertarget="mypopover" popovertargetaction="hide">
  Hide
</button>
<button popovertarget="mypopover" popovertargetaction="toggle">
  Toggle
</button>
<div id="mypopover" popover>Content</div>
```

### Using the `command` Attributes (Alternative)

```html
<button commandfor="mypopover" command="show-popover">Show</button>
<button commandfor="mypopover" command="hide-popover">Hide</button>
<div id="mypopover" popover>Content</div>
```

## 2. CSS Styling & Positioning

### Default Popover Styles

Browsers apply these default styles to popovers:

```css
[popover] {
  position: fixed;
  inset: 0;
  width: fit-content;
  height: fit-content;
  margin: auto;
  border: solid;
  padding: 0.25em;
  overflow: auto;
  color: CanvasText;
  background-color: Canvas;
}
```

### Custom Positioning

To override default positioning:

```css
:popover-open {
  width: 300px;
  height: auto;
  position: absolute;
  inset: unset;
  bottom: 20px;
  right: 20px;
  margin: 0;
  border-radius: 8px;
  box-shadow: 0 10px 40px rgba(0,0,0,0.2);
}
```

### Styling the Backdrop

The `::backdrop` pseudo-element sits behind the popover:

```css
[popover]::backdrop {
  background-color: transparent;
  backdrop-filter: blur(4px);
  transition: background-color 0.3s ease;
}

[popover]:popover-open::backdrop {
  background-color: rgba(0, 0, 0, 0.3);
}
```

## 3. CSS Anchor Positioning

Anchor positioning allows you to position popovers relative to their invoker buttons without JavaScript calculations.

### Basic Anchor Positioning

```html
<button id="anchor-btn" popovertarget="mypopover">
  Click me
</button>
<div id="mypopover" popover>
  I'm anchored to the button!
</div>
```

```css
/* The popover automatically has an implicit anchor reference */
#mypopover {
  margin: 0;
  inset: auto;
  position-area: bottom;  /* Position below the anchor */
}
```

### Position Areas

The `position-area` property creates a 3×3 grid around the anchor:

```css
/* Physical positions */
position-area: top;
position-area: bottom;
position-area: left;
position-area: right;
position-area: top right;
position-area: center;

/* Logical positions (respects writing mode) */
position-area: start;
position-area: end;
position-area: block-start;
position-area: inline-end;

/* Spanning multiple areas */
position-area: span-top left;  /* Spans top-left corner */
position-area: bottom span-all;  /* Spans entire bottom row */
```

### Fallback Positions

When a popover would overflow, you can specify fallback positions:

```css
#mypopover {
  position-area: top;
  position-try-fallbacks: bottom, left, right;
  position-try-order: most-width;  /* Choose position with most space */
}
```

Or using the shorthand:

```css
#mypopover {
  position-area: top;
  position-try: most-width bottom, left, right;
}
```

### Custom Position Fallbacks

Define complex fallbacks with `@position-try`:

```css
@position-try --compact-top {
  position-area: top;
  width: 200px;  /* Smaller width in fallback */
}

#mypopover {
  position-area: top right;
  width: 300px;
  position-try: most-width, --compact-top, bottom left;
}
```

### Using the `anchor()` Function

For more precise control:

```css
#mypopover {
  position: absolute;
  position-anchor: --anchor-btn;
  top: anchor(bottom);
  left: anchor(center);
  transform: translateX(-50%);
}
```

### Multiple Anchors

You can anchor different sides to different elements:

```css
.anchor-1 { anchor-name: --anchor-1; }
.anchor-2 { anchor-name: --anchor-2; }

#mypopover {
  position: absolute;
  inset-block-start: anchor(--anchor-1 bottom);
  inset-inline-end: anchor(--anchor-2 left);
}
```

## 4. Animating Popovers

### CSS Transitions

```css
[popover]:popover-open {
  opacity: 1;
  transform: scale(1);
}

[popover] {
  opacity: 0;
  transform: scale(0.95);
  transition:
    opacity 0.3s ease,
    transform 0.3s ease,
    overlay 0.3s allow-discrete,
    display 0.3s allow-discrete;
}

@starting-style {
  [popover]:popover-open {
    opacity: 0;
    transform: scale(0.95);
  }
}
```

### CSS Keyframe Animations

```css
[popover]:popover-open {
  animation: popover-in 0.3s ease-out forwards;
}

[popover] {
  animation: popover-out 0.3s ease-in forwards;
}

@keyframes popover-in {
  0% {
    opacity: 0;
    transform: translateY(-10px) scale(0.95);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

@keyframes popover-out {
  0% {
    opacity: 1;
    transform: translateY(0) scale(1);
    display: block;
  }
  100% {
    opacity: 0;
    transform: translateY(-10px) scale(0.95);
    display: none;
  }
}
```

## 5. Interest Invokers (Hover/Focus Popovers)

Interest invokers allow popovers to show on hover or focus without JavaScript.

### Basic Interest Invoker

```html
<a href="#" interestfor="tooltip">Hover me</a>
<div id="tooltip" popover="hint">I'm a tooltip!</div>
```

```css
#tooltip {
  position-area: bottom;
}
```

### Combining Interest and Click

```html
<button
  interestfor="tooltip"
  commandfor="menu"
  command="toggle-popover">
  Button
</button>
<div id="tooltip" popover="hint">Hover tooltip</div>
<div id="menu" popover="auto">
  <button>Action 1</button>
  <button>Action 2</button>
</div>
```

### Interest Delays

```css
a[interestfor] {
  interest-delay: 0.5s 1s;  /* 0.5s to show, 1s to hide */
}

/* Once interest is shown on any button, remove delay for others */
p:has(a:interest-source) a {
  interest-delay-start: 0s;
}
```

### Styling Based on Interest

```css
button:interest-source {
  background-color: orange;
}

#tooltip:interest-target {
  border-style: dashed;
}
```

## 6. JavaScript API

### Feature Detection

```javascript
function supportsPopover() {
  return Object.hasOwn(HTMLElement.prototype, 'popover');
}

function supportsInterestInvokers() {
  return Object.hasOwn(HTMLButtonElement.prototype, 'interestForElement');
}
```

### Programmatic Control

```javascript
const popover = document.getElementById('mypopover');

// Show popover
popover.showPopover();

// Hide popover
popover.hidePopover();

// Toggle popover
popover.togglePopover();

// Check if open
if (popover.matches(':popover-open')) {
  console.log('Popover is open');
}
```

### Setting Up Popovers Programmatically

```javascript
const popover = document.getElementById('mypopover');
const button = document.getElementById('toggleBtn');

if (supportsPopover()) {
  popover.popover = 'auto';
  button.popoverTargetElement = popover;
  button.popoverTargetAction = 'toggle';
} else {
  button.style.display = 'none';
}
```

### Event Handling

```javascript
popover.addEventListener('beforetoggle', (e) => {
  console.log(`About to toggle: ${e.oldState} → ${e.newState}`);
  console.log(`Triggered by:`, e.source);

  // Prevent opening
  if (e.newState === 'open' && someCondition) {
    e.preventDefault();
  }
});

popover.addEventListener('toggle', (e) => {
  console.log(`Toggled: ${e.oldState} → ${e.newState}`);
});
```

### Interest Events

```javascript
const tooltip = document.getElementById('tooltip');

tooltip.addEventListener('interest', (e) => {
  console.log('Interest shown on:', e.source);
  // Update content based on source
  tooltip.textContent = `Info about ${e.source.textContent}`;
});

tooltip.addEventListener('loseinterest', (e) => {
  console.log('Interest lost on:', e.source);
});
```

### Progressive Enhancement for Hover

```javascript
const popover = document.querySelector('[popover]');
const anchor = document.querySelector('button');

// Add hover behavior as enhancement
if (anchor && popover) {
  anchor.addEventListener('mouseenter', () => {
    popover.showPopover({ source: anchor });
  });

  anchor.addEventListener('mouseleave', () => {
    popover.hidePopover();
  });
}
```

## 7. Complete Examples

### Example 1: Dropdown Menu with Anchor Positioning

```html
<nav>
  <button id="menu-btn" popovertarget="dropdown-menu">
    Menu ▾
  </button>
  <div id="dropdown-menu" popover="auto">
    <ul>
      <li><button>Profile</button></li>
      <li><button>Settings</button></li>
      <li><button>Logout</button></li>
    </ul>
  </div>
</nav>
```

```css
#dropdown-menu {
  margin: 0;
  inset: auto;
  position-area: bottom;
  min-width: 200px;
  padding: 8px 0;
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.15);
}

#dropdown-menu ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

#dropdown-menu li button {
  width: 100%;
  padding: 8px 16px;
  border: none;
  background: none;
  text-align: left;
  cursor: pointer;
}

#dropdown-menu li button:hover {
  background-color: #f0f0f0;
}
```

### Example 2: Tooltip with Tether Arrow

```html
<button id="anchor-btn" popovertarget="tooltip">
  Hover for info
</button>
<div id="tooltip" popover="hint">
  This is a helpful tooltip!
</div>
```

```css
#tooltip {
  --tether-offset: 1px;
  --tether-size: 8px;

  position-anchor: --anchor-btn;
  position: absolute;
  position-area: top;
  position-try: --bottom, --left, --right;

  margin: 0 0 var(--tether-size) 0;
  padding: 8px 12px;
  border-radius: 6px;
  background: #333;
  color: white;
  font-size: 14px;

  clip-path: inset(var(--tether-offset)) margin-box;
}

/* Top/bottom arrows */
#tooltip::before {
  content: "";
  position: absolute;
  z-index: -1;
  inset: calc(-1 * var(--tether-size)) calc(50% - var(--tether-size));
  background: inherit;
  clip-path: polygon(
    0 var(--tether-size),
    50% 0,
    100% var(--tether-size),
    100% calc(100% - var(--tether-size)),
    50% 100%,
    0 calc(100% - var(--tether-size))
  );
}

/* Left/right arrows */
#tooltip::after {
  content: "";
  position: absolute;
  z-index: -1;
  inset: calc(50% - var(--tether-size)) calc(-1 * var(--tether-size));
  background: inherit;
  clip-path: polygon(
    0 var(--tether-size),
    var(--tether-size) 0,
    calc(100% - var(--tether-size)) 0,
    100% 50%,
    calc(100% - var(--tether-size)) 100%,
    var(--tether-size) 100%
  );
}

@position-try --bottom {
  margin: var(--tether-size) 0 0 0;
}

@position-try --left {
  margin: 0 var(--tether-size) 0 0;
}

@position-try --right {
  margin: 0 0 0 var(--tether-size);
}
```

### Example 3: User Profile Preview Card

```html
<p>
  Check out
  <a href="/users/john" interestfor="profile-card" target="_blank">
    @john
  </a>
  's profile!
</p>

<div id="profile-card" popover="hint">
  <div class="profile-wrapper">
    <img src="avatar.jpg" alt="John Doe" />
    <div class="profile-info">
      <h3>John Doe</h3>
      <p>Frontend Developer at TechCorp</p>
      <p>📍 San Francisco, CA</p>
      <button>Follow</button>
    </div>
  </div>
</div>
```

```css
#profile-card {
  position-area: bottom right;
  width: 320px;
  border: 1px solid #e0e0e0;
  border-radius: 12px;
  padding: 16px;
  background: white;
  box-shadow: 0 8px 30px rgba(0,0,0,0.12);

  opacity: 0;
  transition:
    opacity 0.3s ease,
    overlay 0.3s allow-discrete,
    display 0.3s allow-discrete;
}

#profile-card:interest-target {
  opacity: 1;
}

@starting-style {
  #profile-card:interest-target {
    opacity: 0;
  }
}

.profile-wrapper {
  display: flex;
  gap: 12px;
}

.profile-wrapper img {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  object-fit: cover;
}

.profile-info h3 {
  margin: 0 0 4px 0;
  font-size: 16px;
}

.profile-info p {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: #666;
}

.profile-info button {
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  background: #007bff;
  color: white;
  cursor: pointer;
  font-weight: 500;
}
```

### Example 4: Nested Menu System

```html
<div id="main-menu" popover="auto">
  <button popovertarget="submenu-1">Products ▸</button>
  <button popovertarget="submenu-2">Services ▸</button>
  <button>About</button>
</div>

<div id="submenu-1" popover="auto">
  <button>Product A</button>
  <button>Product B</button>
  <button>Product C</button>
</div>

<div id="submenu-2" popover="auto">
  <button>Consulting</button>
  <button>Development</button>
  <button>Support</button>
</div>
```

```css
#main-menu {
  position-area: bottom;
  min-width: 200px;
}

#submenu-1, #submenu-2 {
  position-area: right;
  min-width: 180px;
}
```

```javascript
// Handle keyboard navigation and closing
const mainMenu = document.getElementById('main-menu');
const submenus = document.querySelectorAll('#submenu-1, #submenu-2');

// Close submenus when main menu closes
mainMenu.addEventListener('toggle', (e) => {
  if (e.newState === 'closed') {
    submenus.forEach(sm => sm.hidePopover());
  }
});

// Close all menus when an option is selected
document.querySelectorAll('[popover] button').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('[popover]').forEach(p => p.hidePopover());
  });
});
```

## 8. Accessibility Best Practices

### Automatic ARIA

The Popover API automatically sets up:

- `aria-expanded` on invoker buttons
- `aria-details` relationship between invoker and popover
- Focus management (focus moves into popover when shown, returns to invoker when closed)

### Manual ARIA (When Needed)

```html
<button
  popovertarget="mypopover"
  aria-haspopup="true"
  aria-expanded="false">
  Open Menu
</button>
<div id="mypopover" popover role="menu">
  <button role="menuitem">Option 1</button>
  <button role="menuitem">Option 2</button>
</div>
```

### Focus Management

```javascript
popover.addEventListener('beforetoggle', (e) => {
  if (e.newState === 'open') {
    // Focus first interactive element
    const firstFocusable = popover.querySelector('button, [href], input');
    if (firstFocusable) {
      setTimeout(() => firstFocusable.focus(), 0);
    }
  }
});
```

## 9. Advanced Patterns

### Multiple Invokers, One Popover

```html
<button interestfor="shared-tooltip">Button 1</button>
<button interestfor="shared-tooltip">Button 2</button>
<button interestfor="shared-tooltip">Button 3</button>
<div id="shared-tooltip" popover="hint">Tooltip content</div>
```

```javascript
const tooltip = document.getElementById('shared-tooltip');
const buttons = document.querySelectorAll('button[interestfor]');

tooltip.addEventListener('interest', (e) => {
  tooltip.textContent = `Info about ${e.source.textContent}`;
});
```

### Dynamic Content Loading

```javascript
popover.addEventListener('beforetoggle', async (e) => {
  if (e.newState === 'open' && !popover.dataset.loaded) {
    const content = await fetch('/api/content').then(r => r.json());
    popover.innerHTML = content.html;
    popover.dataset.loaded = 'true';
  }
});
```

### Position-Aware Styling

```css
/* Different styles based on position */
@position-try --top-position {
  #mypopover {
    border-radius: 8px 8px 0 0;
  }
}

@position-try --bottom-position {
  #mypopover {
    border-radius: 0 0 8px 8px;
  }
}
```

## 10. Browser Compatibility & Polyfills

### Feature Detection

```javascript
const popoverSupported = 'popover' in HTMLElement.prototype;
const anchorSupported = CSS.supports('anchor-name', '--test');
const interestSupported = 'interestForElement' in HTMLButtonElement.prototype;
```

### Progressive Enhancement Strategy

```html
<!-- Fallback for non-supporting browsers -->
<noscript>
  <style>
    [popover] { display: block !important; }
  </style>
</noscript>

<script>
  if (!('popover' in HTMLElement.prototype)) {
    // Load polyfill or provide alternative
    document.documentElement.classList.add('no-popover');
  }
</script>
```

```css
.no-popover [popover] {
  display: none;
}

.no-popover .popover-fallback {
  display: block;
}
```

## Summary

The modern Popover API, combined with CSS Anchor Positioning, provides:

✅ **Declarative HTML** - No JavaScript required for basic functionality
✅ **Automatic accessibility** - Built-in ARIA and focus management
✅ **Smart positioning** - Automatic fallback positioning with CSS
✅ **Smooth animations** - Native CSS transitions and keyframes
✅ **Light dismiss** - Click outside to close (auto state)
✅ **Keyboard support** - Esc to close, Tab navigation
✅ **Interest invokers** - Hover/focus popovers without JS
✅ **Nested popovers** - Support for complex menu systems

This native approach eliminates the need for heavy libraries while providing better performance, accessibility, and maintainability. The small amount of JavaScript shown here is purely for progressive enhancement and advanced patterns—not required for core functionality.
