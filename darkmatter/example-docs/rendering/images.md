# Image Rendering

When we look at image rendering we'll look at four main variants

- Raster Image Inline
- Raster Image Reference
- SVG Inline
- SVG Reference

Each of these variants is supposed to work in both the Terminal and the Browser.


## Raster Images

### Image Reference

This is the most common type of image you'll find in Markdown.

![raster reference](../graph-db.png)


### Inline Image

#### An Image using inline HTML

<img
  alt="A 32×32 red square"
  width="128"
  height="128"
  src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAAAKUlEQVR4nO3NMQEAAAgDoJvc6F+FJgk4mCRZkuT/sa0AAADgM+gAARXgAAGz8QmHAAAAAElFTkSuQmCC"
/>

#### An Image using Idiomatic Markdown syntax

![A visible 32×32 red square](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAAAKUlEQVR4nO3NMQEAAAgDoJvc6F+FJgk4mCRZkuT/sa0AAADgM+gAARXgAAGz8QmHAAAAAElFTkSuQmCC)
