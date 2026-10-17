---
# The OS most unlike this host, when a build host for it is declared
# (`BUILD_WIN`/`BUILD_LINUX`); `ctx.os` alone cannot tell WSL from Linux
cross_os_target: |-
    {{
        env.WSL_DISTRO_NAME
            ? (env.BUILD_WIN ? 'windows' : null)
            : ctx.os == 'Windows'
                ? (env.BUILD_LINUX ? 'linux' : null)
                : (ctx.os == 'Linux' || ctx.os == 'macOS') && env.BUILD_WIN
                    ? 'windows'
                    : null
    }}
# Where a cross-OS failure is recorded: the including prompt's spec, else its
# review, else its plan
os_failing_file: "{{ spec || review || plan || null }}"
---
# OS Testing

All functionality must work across Windows, macOS, Linux, and WSL2/Ubuntu. Keep this in mind while implementing, but be aware that the CI/CD pipeline WILL test cross-OS functionality, so it is NOT a strict requirement that you do so as part of your work. Your current host runs **{{ ctx.os }}**, so do most of your testing here. Where your work carries OS-level risk (for example, a lot of work around file paths, or an OS-specific crate), use the 'os' agent skill to learn about OS variance before you change that code.

::block when="cross_os_target"
## Cross-OS Check

If (and only if) the work you've done has notable cross-OS risks, run a cross-OS test at the end of your implementation cycle to make sure you haven't accidentally broken anything on another OS:

- test **{{ cross_os_target }}** by running `just cross-check <package> --os {{ cross_os_target }}` for each package you touched directly

If you hit any failing tests (regardless of whether you believe they came from your changes) you need to attempt to fix them on **{{ cross_os_target }}**. Make at most three attempts. If **{{ cross_os_target }}** is still failing after the third attempt:

::block when="os_failing_file"
- set the `os_failing` frontmatter property of {{ os_failing_file }} to `{{ cross_os_target }}`
::end-block
::block when="!os_failing_file"
- there is no spec, review, or plan for this task, so report that **{{ cross_os_target }}** is failing in your final summary
::end-block
- you may NOT finish this phase of the work with any failing tests on **{{ ctx.os }}**, but it is OK to continue to the next phase while **{{ cross_os_target }}** is still failing after your third fix attempt. Sometimes a bug can only be fixed properly by running directly on the failing OS; this will be flagged later in the process so that it gets done (this is not your concern directly)
::end-block

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

These environments are MUCH faster than CI/CD. Each of these test rigs allows SSH access, but to make your life easier, and to make sure we act as "good citizens" on these test rigs, use the `just cross-check` recipe instead; it already knows which hosts per OS are available and how to interact with them:

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
