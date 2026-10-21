# Graphics Where Possible

The spec was originally created because I couldn't get my Wezterm multiplexed session in Windows to render graphics but I was able to reconfigure and now they display. Still, I would like to make sure that people using biscuit-terminal and biscuit-visualized get graphics "out of the box" as easily as possible and particularly in cases where someone is using a multiplexer and things might have some extra complexities.

There are three main graphics protocols used in terminals today:

- [**Kitty**](biscuit-terminal/docs/research/graphics/kitty.md)  
    - the most advanced and directionally where things are going)
- [**Iterm2**](biscuit-terminal/docs/research/graphics/iterm2.md)
    - high quality graphics but with less adoption and even iTerm2 has Kitty support now
- [**Sixel**](biscuit-terminal/docs/research/graphics/sixel.md)
    - the lowest quality graphics of the three

We currently only render using Kitty graphics and I'm not in hurry to support other formats but if there were benefits to supporting **iTerm2** that would help people get graphics delivered to their terminals then I'm completely open to adding it. 

> Note: not interested in adding Sixel graphics unless there is a REALLY strong reason to

The main focus is getting folks with terminal emulation to still see graphics (assuming their terminal app supports it); we have done some research on this: [graphics with multiplexing](biscuit-terminal/docs/research/graphics/multiplexing.md).
