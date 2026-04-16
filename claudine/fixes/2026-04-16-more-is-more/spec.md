# More is More feature

We recently unified the rendering across `compose` and `inline-compose` and hopefully our code is much DRY'er now but in the process we've noticed a series of regressions and just in general "less information" being presented than was done before (very negative).

It is critically important that when a user starts a non-interactive session with Claudine that we provide as much information as possible to keep the caller informed on the progress.

## Regressions

### Hanging More often

We doo appear to be hanging more now. The output below is a common problem now. Note that it would probably have gone on forever but when I hit CTRL+C it immediately showed text that you'd have expected to be displayed right away and then exited. 

```sh
 60s · 3 done
 90s · 3 done
 120s · 3 done
 150s · 3 done
 180s · 3 done
 210s · 3 done
 240s · 3 done
 270s · 3 done
 300s · 3 done
 330s · 3 done
 360s · 3 done
 390s · 3 done
 420s · 3 done
 450s · 3 done
 480s · 3 done
 510s · 3 done
 540s · 3 done
 570s · 3 done
 600s · 3 done
 630s · 3 done
 660s · 3 done
 690s · 3 done
 720s · 3 done
 750s · 3 done
 780s · 3 done
 810s · 3 done
 840s · 3 done
 870s · 3 done
 900s · 3 done
 930s · 3 done
 960s · 3 done
 990s · 3 done
 1020s · 3 done
 1050s · 3 done
 1080s · 3 done
^CTwo files are staged, both in claudine-cli, related to the wrap command's composition module. These are refactoring changes
(replacing emit_stream_summary_no_separator_with_context with inline Prose rendering). I'll spawn one subagent to handle this
group.



✓ 1081s · 2K input tokens · 296 output tokens · 70K cached tokens · $0.02 cost basis · 3 tool calls
error: Recipe `commit` failed on line 55 with exit code 130
```

### 
