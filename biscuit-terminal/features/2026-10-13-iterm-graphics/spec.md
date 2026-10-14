# ITerm Graphics

There are three main graphics protocols used in terminals today:

- **Kitty**  
    - the most advanced and directionally where things are going)
- **Iterm2** 
    - high quality graphics but with less adoption and even iTerm2 has Kitty support now
- **Sixel**
    - the lowest quality graphics of the three

Currently biscuit-terminal supports Kitty graphics but NOT iTerm2 or Sixel. We will likely never support Sixel but there are benefits to supporting iTerm2:

- more terminals get high quality graphics
- Windows SSH sessions kill all Kitty graphics but let iTerm2
