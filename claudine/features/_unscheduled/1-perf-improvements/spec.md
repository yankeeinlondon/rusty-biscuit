Leveraging claudine `--perf` flag we can start to analyze where things are taking longer than they should and start to get closer to the desired performance of Claudine. In addition to "true performance" we will also address "perceived performance" too by addressing the long delays we are seeing between when a user runs `claudine` and when the "Claudine Execution Line" shows up.


There are a few important Just recipes that help me "project manage" features that are being considered. 

1. At a macro level, a feature should start in the `_unscheduled` directory and once ready to be implemented we use `just schedule` to move it into a direct subdirectory of the "features" directory of the current "package area". When doing this we will prefix today's date (in YYYY-MM-DD format) to the feature's directory name. 
2. continuing at the macro level, when a feature has been implemented we will move it to the `_completed` directory. Doing this reduces the clutter and aids other just recipes by only showing features that are active (or as a fallback are _unscheduled). We move things into the `_unscheduled` directory with the `just complete` command.

