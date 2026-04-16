# More is More feature

We recently unified the rendering across `compose` and `inline-compose` and hopefully our code is much DRY'er now but in the process we've noticed a series of regressions and just in general "less information" being presented than was done before (very negative).

- It is critically important that when a user starts a non-interactive session with Claudine that we provide as much information as possible to keep the caller informed on the progress. 
- This information should be as consistent as is possible across Agent providers 
    - but because there is inconsistency in how much information each provider provides then we accept that this can't be 100% consistent but it should be consistent when it can
- The only information we _don't_ want to repeat is repetitive information

## Things Which are Missing

Things we would always expect to see:

- not just `Bash` or `Zsh` but the parameter which were used in the tool call!
    - I propose in this case we format this information like `Bash(<dim><i>bash {params}<i></dim>)`, `Zsh(<dim><i>zsh {params}</i></dim>)`, etc.
- similarly when we see `Task` we should see the task information too not just that we created a task
- **thinking text** used to be displayed but appears to be missing (at least from OpenCode)
    - we had agreed that all thinking text would be rendered with a `BlockQuote` with a gray vertical bar (and wider block character equivalent to what we use for the block quotes in System Prompt and Agent Prompt) to make this thinking text clearly distinguishable from the final output.
- **warnings and errors**
    - any warning or error should be rendered
    - warnings should use the `Status` struct in WARNING state
    - errors should use the BlockQuote style rendering with a red vertical bar. there should be examples of this already in the code base (unless they were possible removed in the last few commits)
        - We also need to be able to clearly identify different types of errors so that our "handling" logic can appropriately respond to errors

## Other Regressions

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

