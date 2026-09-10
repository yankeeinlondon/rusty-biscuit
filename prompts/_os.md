# OS Testing

You are currently operating on a host that runs "{{ ctx.os }}" and so most of your testing should be on this local host until you are close to being done (aka, all tests pass on the local host and consideration has been given to the other OS's we need to support). 

Note that we don't _always_ need to test on other OSes because CI/CD will incorporate this for us but where we feel there are OS level risks you should do two things:

1. use the 'os' agent skill to gather more information about OS variance
2. leverage the available local OS platforms to do your testing; these environments will be MUCH faster than CI/CD and allow us to lower our OS risk prior to pushing to the remote

## Local OS Testing Rigs

::block when="env.BUILD_LINUX || env.BUILD_WIN || env.BUILD_WSL || env.BUILD_MACOS"
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
::end-block
::block when="!env.BUILD_LINUX && !env.BUILD_WIN && !env.BUILD_WSL && !env.BUILD_MACOS"
Currently there are no local testing rigs for you to use. You should report to the caller that this is the case and tell them which ENV variables they should set to do this: 

- BUILD_LINUX - set to the configured SSH host that will run Linux based testing
- BUILD_WIN - set to the configured SSH host that will run native Windows based testing
- BUILD_WSL - set to the configured SSH host that will run WSL based testing on Windows
- BUILD_MACOS - set to the configured SSH host that will run macOS based testing

The benefits to having these environments available should be seen as important so you should not only report the lack of these environments at the point of discovery but also at the end of any task as part of your summary.
::end-block
