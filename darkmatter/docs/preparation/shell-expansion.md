# Shell Expansion

We allow the output of shell commands to be injected into a Markdown page using the syntax:

```md
::shell <command> <params>
```

This is a powerful feature but if left unguarded it could be a very damaging one too. To prevent malicious or accidentally harmful commands from being run we have a two stage security design:

1. there are a set of [Blacklisted Commands and Syntax](#blacklisted-commands-and-syntax) which will NEVER be allowed to be run
2. we maintain a list of approved commands in a namespaced "whitelist" file:
     - located in `[repo root]/.darkmatter-shell-whitelist` if CWD is a git repo
     - otherwise located in `${HOME}/.darkmatter-shell-whitelist`

We also maintain a companion namespaced blacklist file for user-denied commands:

- located in `[repo root]/.darkmatter-shell-blacklist` if CWD is a git repo
- otherwise located in `${HOME}/.darkmatter-shell-blacklist`

When the Darkmatter compose pipeline reaches the Shell Expansion stage, it will iterate over all `::shell` lines and:

- if the command does not exist on the host system we exit the pipeline in error
    - `<red><b>ERROR:</b></red> the shell command '{command}' does not exist on this host but was referenced in a shell expansion operation during the <b>compose</b> pipeline in <blue>{file}</blue>!`
- if the command matches the blacklisted commands and syntax we exit in error
    - `<red><b>ERROR:</b></red> the shell command '{command}' is not allowed as a shell expansion command in Darkmatter's compose pipeline! This command is considered a globally blacklisted command.`
- if the command does not exist in the repo's (or user's) whitelist file the the user will be asked to approve this command. For more details see [Approvals and the Whitelist](#approvals-and-the-whitelist).
- if the command _does_ exist in the whitelist then we execute the command and both STDOUT and STDERR are captured and added to the Markdown document in place of the `::shell` instruction.

    - If a command does not complete in 10 seconds (by default) then we will exit with an error
        - `<red><b>ERROR:</b></red> the shell command '{command}' in {file} took too long to complete (10 seconds) and was terminated. The Darkmatter pipeline has exited.`
    - If the command outputs nothing in STDOUT or STDERR while returning a 0 exit code (aka, no error) then we simply remove the `::shell` instruction line.
    - If the shell command's exit code is _not_ 0 (aka, there was an error when running the command) then we will exit the pipeline with an error:
        - `<red><b>ERROR:</b></red> the shell command '{command}' in {file} exited with an error code of {error_code}. The Darkmatter pipeline has exited.\n\n<b>STDOUT:</b> {stdout}\n\n<b>STDERR:</b>{stderr}`

## Blacklisted Commands and Syntax

The following commands will never be allowed as they are part of the global blacklist:

- `rm`
- `rimraf`
- `find*-delete`
- `unlink`
- `shred`
- `wipe`
- `echo* >>*`, `echo* >*`
- `* >*`
- `install`
- `brew`, `apt`, `nala`, `pacman`, `dnf`, `yum`
- `npm uninstall`, `pnpm uninstall`, `bun uninstall`, `yarn uninstall`
- `npm install`, `pnpm install`, `bun install`, `yarn install`
- `npm add`, `pnpm add`, `bun add`, `yarn add`
- `mv`
- `dd`
- `zfs`, `zpool`
- `wipefs`
- `mkfs*`
- `parted`
- `mparted`
- `sgdisk`
- `pvcreate`
- `lvremove`
- `vgremove`
- `mdadm`
- `cryptsetup`
- `chmod`, `chgrp`, `chown`, `setfacl`
- `tar`, `unzip`, `rsync`, `cp`,
- `kill`, `pkill`, `killall`, 
- `systemctl`
- `shutdown`, `reboot`, `poweroff`, `halt`
- `init`
- `git reset`, `git clean`, `git checkout`, `git restore`, `git rebase`, `git branch`, `git push`, `git reflog`, `git gc`, `git reset`
- `psql -c`
- `mysql -e`
- `redis-cli FLUSH*`
- `mongosh --eval`
- `ssh`
- `scp`
- `rsync`
- `ansible`
- `curl`
- `wget`
- `http`
- `source`
- `eval`
- `sudo`
- `doas`
- `su`
- `docker rm*`, `docker system prune*`, `docker volume rm*`, `docker volume prune*`
- `kubectl delete*`
- `helm uninstall*`
- `terraform destroy*`

## Approvals and the Whitelist

When a command that does NOT match the Blacklist and is not registered in the Whitelist either then we must ask the user if they want to:

- allow exact command (add command with all params to `.darkmatter-shell-whitelist`)
- allow command with any parameters (add command with wildcard signature for params to `.darkmatter-shell-whitelist`)
- allow once (all the current execution but do not add to `.darkmatter-shell-whitelist`)
- deny (exit the pipeline process with an error but don't add to `.darkmatter-shell-blacklist`)
- blacklist (exits the pipeline with an error and adds this command to `.darkmatter-shell-blacklist`)


--- 

> Return to [Darkmatter Pipeline](../darkmatter-pipeline.md)
