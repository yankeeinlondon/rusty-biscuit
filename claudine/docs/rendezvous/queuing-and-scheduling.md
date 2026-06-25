# Queuing and Scheduling Work

Both _queuing_ and _scheduling_ of work map to the goal of doing some specified work at some time OTHER than **now**.

- **Queuing** tends to be useful for more immediate things:
    - In 15 minutes, run a task
    - After PID 12345 completes, run a task
    - After Session sz_2342ks454 completes, run a task
- **Scheduling** is good for scheduling things further into the future as well as creating recurring tasks on a specified interval:
    - At 9pm tomorrow, run a task
    - On Oct 3rd, run a task
    - Every Monday at 5:00am, run a task

## The Importance of Isolation

- imagine a situation where we have 5 specifications along with accompanying implementation plans
- all of these specs/plans target the same package in a monorepo you're working on
- you kick off one of these implementation plans (implement -> review -> implement, etc.) which you expect to take 1-2 hours to complete
- Now what? 
    - Before AI we would be actively involved in the implementation, review, etc. so our attention was already consumed
    - in the era of AI, the developer offloads lots of the work which fundamentally changing the rhythm/flow of development
    - to be efficient in this new flow, a developer must learn to do a lot more multi-tasking across work
    - while working on 10 completely unrelated things _could_ be done in a concurrent fashion the "switching costs" of moving from one semantic environment to another would be too high to really consider this an effective strategy
    - instead the typical effort is into finding ways to leverage concurrency across 1-2 focus areas and find ways to do that without creating too much operational risk
- One of the first things which became popular once AI became capable enough to own most of development and testing was **git worktrees**
    - a worktree allows a developer to have multiple streams of work run concurrently on the same code base but with programmatic **isolation** being provided by this handy git feature
    - how many parallel streams can you run at once? it depends on the overlap in code and functionality across the set of specifications but there is a very real limit
    - in general most AI developers will have no more than 2-3 worktrees doing work at the same time (on a particular package); this might extend to 4-5 worktrees in cases where you are doing A/B testing and the various threads are just variants of the same thing.
- Although far more rare than git worktrees, developers are starting to run compute across remote machines (physically remote or at least running in a virtual machine). This is done again to achieve **isolation** and because this form of isolation is done at the host level it has far greater isolation properties but because the tooling hasn't made this easy yet it's still under-leveraged.
- Another form of isolation that has a lot of potential is **time isolation**
- If a developer can find an efficient way to run a sequence of tasks in a way where the human and AI interaction is efficient, human involvement is steered toward the start and end of the process versus in the intervening tasks, and notifications are able to get the attention of the human when it is warranted.
- **Claudine**'s sequences and loops are a first stab at giving developers the tools to
