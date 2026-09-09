# OS Testing

You are currently operating on a host that runs "{{ ctx.os }}" and so most of your testing should be on this local host until you are close to being done (aka, all tests pass on the local host and consideration has been given to the other OS's we need to support). 

Note that we don't _always_ need to test on other OS's because CI/CD will incorporate this for us but where we feel there are OS level risks you should do two things:

1. use the 'os' agent skill to gather more information about OS variance
2. leverage the available local OS platforms to do your testing; these environments will be MUCH faster than CI/CD and allow us to lower our OS risk prior to pushing to the remote

## Local OS Testing Rigs

The following local testing rigs are available to you:

::block when="env.BUILD_LINUX"
- Linux testing can be done on: **{{env.BUILD_LINUX}}**
::end-block
::block when="env.BUILD_WIN"
- Windows testing can be done on: **{{env.BUILD_WIN}}**
::end-block
::block when="env.BUILD_WSL"
- WSL testing can be done on: **{{env.BUILD_WSL}}**
::end-block
::block when="env.BUILD_MACOS"
- macOS testing can be done on: **{{env.BUILD_MACOS}}**
::end-block

Each of these test rigs allow for SSH access but to make your life easier and to make sure we act as "good citizens" on these test rigs you should instead use the `just cross-check` recipe that will already know which hosts per OS are available and how to interact with them:

```sh
just cross-check <package> --os [linux, windows, wsl, macos, all]
```

## Worktrees make better Neighbors

Remember that when you're using a shared resource like a test rig you may have other people and agents who are working on the same repo as you at the same time. To avoid any conflicts be sure
to create you're own worktree to work in:

- start by running `git fetch` to ensure you have the latest from the remote
- then create worktree where you will do your testing; naming should follow the pattern: `{hostname}-{branch-name}-{epoch-timestamp}`
- once you've completed your testing on the testing rig make sure that you remove the worktree or these rigs will quickly become low on disk storage!
    - you should not be modifying code on these platforms but
