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
    - however, with AI the developer offloads lots of the work and fundamentally changing the rhythm/flow of development
- One of the first things which became popular once AI became capable enough to own most of development and testing was git worktrees
    - a **worktree** allows a developer to have multiple streams of work run concurrently on the same code base but with programmatic **isolation** being provided by this handy git feature
    - how many parallel streams can you run at once? it depends on the overlap in code and functionality across the set of specifications but there is a very real limit
    - in general most AI developers will have no more than 2-3 worktrees doing work at the same time; this might extend to 4-5 worktrees in cases where you are doing A/B testing and the various threads are just variants of the same thing.
- In far few cases today, developers are running the specs on remote machines (physically remote or at least running in a virtual machine). This is done again to achieve **isolation** and because this form of isolation is done at the host level it has far greater isolation properties.
    - If this 


- While you're initially blocked, one of the things which has exploded in popularity recently is using git **worktrees**
- A worktree allows you to run multiple of these plans in parallel and each plan is run in an isolated **environment**
- How many can you run in parallel, well of course it depends (mainly on how isolated the functionality of each is) but for each additional worktree and parallel stream you have, the risks and time spent merging them back together increases. Also, for many developers, if they parallelize too much they will find that instead of working really quickly, all of their parallel workstreams will get capped halfway through the plan. 
- In addition to avoiding immediate
