# Using Iconify Icons in Mermaid

## Icon Sets

- [View Font Awesome 7 Brands](https://icon-sets.iconify.design/fa7-brands/), [Download](https://unpkg.com/@iconify-json/fa7-brands/icons.json)
- [View Lucide Icons](https://icon-sets.iconify.design/lucide/), [Download](https://unpkg.com/@iconify-json/lucide/icons.json)
- [View Carbon Icons](https://icon-sets.iconify.design/carbon/), [Download](https://unpkg.com/@iconify-json/carbon/icons.json)
- [View SystemUI Icons](https://icon-sets.iconify.design/system-uicons/), [Download](https://unpkg.com/@iconify-json/system-uicons/icons.json)


## Integration

### Lazy Loading from CDN

```js
import mermaid from 'CDN/mermaid.esm.mjs';
mermaid.registerIconPacks([
  {
    name: 'logos',
    loader: () =>
      fetch('https://unpkg.com/@iconify-json/logos@1/icons.json').then((res) => res.json()),
  },
]);
```
