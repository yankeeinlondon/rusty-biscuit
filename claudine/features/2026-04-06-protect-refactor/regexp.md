## Dangerous RegExp Patterns for Tool Calling

### 1. Mass File Deletion
These patterns target recursive deletions that could wipe a filesystem or critical directories.

* `rm -rf /` — The classic attempt to delete the entire root directory.
* `rm -rf (.*)?/\*` — Deleting everything inside a directory recursively (e.g., `rm -rf ./*` or `rm -rf /var/*`).
* `find .* -delete` — Using the `find` command to bypass simple `rm` aliases.

### 2. Disk and Partition Manipulation
Commands that bypass the filesystem to interact directly with hardware or partition tables.

* `mkfs\..*` — Formatting a partition (e.g., `mkfs.ext4`, `mkfs.vfat`).
* `dd if=.* of=/dev/sd[a-z]` — Overwriting a physical drive with data (often used for "zeroing" a disk).
* `fdisk .*` or `parted .*` — Commands used to delete or re-label disk partitions.

### 3. Volume and Storage Destruction
Targeting logical volumes and modern storage management.

* `lvremove .*` — Removing Logical Volumes in LVM.
* `zfs destroy .*` — Permanently deleting ZFS datasets or snapshots.

### 4. System Reconfiguration and Shakedowns
Commands that render a system unbootable or wipe out the user environment.

* `mv /.* /dev/null` — Moving critical system directories into the "black hole."
* `chown -R .* /` — Recursively changing ownership of the root directory (breaks permissions/sudo).
* `chmod -R 777 /` — Making every file world-writable (a massive security breach).

### 5. Remote and Indirect Execution
Methods to hide destructive payloads or execute them from the web.

* `curl .* | bash` — Piping a remote script directly into a shell (extremely high risk).
* `wget .* -O- | sh` — The `wget` equivalent of the piping risk.

### 6. Resource Exhaustion (Fork Bombs)
Commands designed to crash the system by consuming all available processes.

* `:\(\)\{ :|:& \};:` — The classic Bash fork bomb.
* `perl -e 'fork while 1'` — A Perl-based process exhaustion attack.

### 7. Device and Memory Corruption
These commands interact with the kernel's virtual filesystems to instantly crash or wipe a system.

* `echo [bcdeghimnpstuvw] > /proc/sysrq-trigger` — Manually triggering SysRq commands (like immediate reboot or memory dumps).
* `> /dev/sd[a-z]` — Using redirection to truncate/wipe an entire block device.
* `cat /dev/zero > /dev/mem` — Attempting to overwrite physical memory (usually requires high privileges but is catastrophic).

### 8. Obfuscated and Encoded Execution
Attackers often hide "cleartext" commands using encoding to bypass simple string-matching filters.

* `echo .* | base64 -d | sh` — Decoding a Base64 string and piping it directly into a shell.
* `printf .* | xxd -r -p | bash` — Converting hex strings back into binary/scripts for execution.
* `eval \$(echo .*)` — Using `eval` to execute a string that has been manipulated or reversed.

### 9. Network and Firewall Sabotage
Patterns that isolate the machine or open it up to the public internet by destroying safety configurations.

* `iptables -F` — Flushing all firewall rules, potentially leaving the system wide open.
* `ufw disable` — Disabling the Uncomplicated Firewall.
* `ip link set .* down` — Shutting down critical network interfaces (e.g., `eth0` or `lo`).

### 10. Database and Package Manager Nukes
If the Agent has access to specific environments, these commands can wipe years of data in seconds.

* `mysql -e "DROP DATABASE .*"` — Dropping a SQL database via the command line.
* `redis-cli flushall` — Wiping all keys from a Redis data store.
* `npm un.* -g` — Globally uninstalling critical packages/dependencies.

### 11. Log and Audit Trail Destruction
These commands are used to "clean up" after a malicious action, making forensic recovery nearly impossible.

* `shred -u .*` — Using `shred` to overwrite a file multiple times before deleting it.
* `history -c && rm ~/.bash_history` — Clearing the command history to hide what the Agent (or attacker) did.

To hit your goal of 20 patterns, here are the final 9 destructive shell command patterns. These focus on more nuanced system destruction, such as kernel module tampering, service disruption, and recursive permission wipes that are often missed by basic filters.

### 12. Kernel and Driver Sabotage
These commands can disable hardware or crash the operating system by removing essential kernel modules.

* `rmmod .*` — Forcibly removing kernel modules (e.g., `rmmod network_driver`).
* `modprobe -r .*` — The more modern way to unload modules, which can disable disks or input devices.

### 13. Service and Process Mass-Termination
Patterns that kill every running process or disable the init system's ability to manage the machine.

* `kill -9 -1` — Sending a SIGKILL to every process the current user has permission to kill.
* `systemctl disable --now .*` — Not just stopping a service, but ensuring it never starts again on reboot.

### 14. Configuration and Bootloader Destruction
Targeting the files that allow the computer to actually start up.

* `rm -rf /boot/*` — Deleting the kernel images and Grub configuration.
* `update-grub` (when preceded by malicious config edits) — Finalizing the destruction of the boot sequence.

### 15. Recursive Permission Stripping
While `777` makes files world-writable, these patterns make files completely inaccessible, effectively "bricking" the software layer.

* `chmod -R 000 /` — Removing all read, write, and execute permissions from the entire system.
* `chattr +i .*` — Making files immutable so even root cannot delete or edit them without a complex reversal.

### 16. Swap and Memory Pressure
By disabling swap space, you can force the Linux OOM (Out Of Memory) killer to start terminating critical system processes.

* `swapoff -a` — Disabling all device and file swaps, leading to immediate system instability under load.

### 17. Remote Shell Injection (Reverse Shells)
While not "destructive" in terms of deleting files, these give a third party full control to perform the destructions listed above.

* `bash -i >& /dev/tcp/.* 0>&1` — The classic bash reverse shell pattern.
* `python -c 'import socket...os.dup2...'` — Python-based socket hijacking to open a remote back-door.

### 18. Crontab and Persistence Wipe
Wiping out the automation layer can stop critical backups or maintenance scripts from running.

* `crontab -r` — Deleting the current user's entire cron table without a confirmation prompt.

### 19. Binary Replacement (Trojanizing)
Replacing a common command with a destructive one.

* `alias ls='rm -rf'` — A common "prank" that is actually a catastrophic command injection.
* `cp /bin/sh /bin/ls` — Overwriting common utilities to break scripts and user interaction.

### 20. Mail and Spool Overload
Filling the system's communication or print buffers to cause disk exhaustion in sensitive `/var` partitions.

* `cat /dev/urandom | mail .*` — Flooding the local or remote mail spool with infinite random data.

### GIT Specific

#### 21. Forced History Rewriting
These patterns identify attempts to overwrite the remote repository’s history. This can delete weeks of work from colleagues if they haven't pulled recently.

git push .* --force — The standard "nuclear" push that overwrites the remote branch.

git push .* -f — The shorthand version of the forced push.

#### 22. Local Work Erasure
These commands wipe out any changes in the working directory that haven't been committed yet. There is no "undo" for these once executed.

git reset --hard .* — Forcefully moves the current branch pointer and wipes the working directory to match.

git clean -fdx — Recursively removes untracked files, including those ignored by .gitignore (like .env files or build artifacts).

#### 23. Destructive Branch Management
Deleting branches, especially remote ones, can lead to data loss if those branches contained unique commits.

git push .*--delete .* — Deleting a branch from the remote server.

git branch -D .* — Forcibly deleting a local branch that hasn't been merged yet.

#### 24. Reflog and Garbage Collection Purging
Git usually keeps a "safety net" called the Reflog. These commands clear that net, making "lost" commits truly unrecoverable.

git reflog expire --expire=now --all — Clearing the record of all recent actions.

git gc --prune=now --aggressive — Forcing the permanent deletion of "unreachable" objects immediately.

#### 25. Submodule and Config Sabotage
Tampering with the underlying Git configuration or submodules can break the repository structure for all users.

rm -rf .git — The ultimate "delete the repo" command.

git config --global --unset-all .* — Wiping out user configurations, which can break authentication or signing.


### Containers

To round out a truly robust protection system, you should look into **Cloud/Container orchestration**, **Package Manager sabotage**, and **In-Memory execution**. These are the "modern" ways an Agent or attacker might cause irreparable damage or hide their tracks.

Here are 5 final domains to bring your total well over 30 patterns:

#### 26. Container & Orchestration Nukes
If your Agent has access to `docker` or `kubectl`, it can wipe out entire production environments with a single line.

* `docker system prune -a --volumes` — Deletes every stopped container, unused network, and, most destructively, all persistent volumes.
* `kubectl delete namespaces --all` — The "delete everything" command for Kubernetes, wiping out every service, pod, and config.
* `docker rm -f $(docker ps -aq)` — Forcibly kills and removes every container on the host.

#### 27. Cloud Provider "Account Wipes"
If the Agent has AWS, GCP, or Azure CLI tools installed, these patterns are the equivalent of `rm -rf /` for your infrastructure.

* `aws ec2 terminate-instances --instance-ids .*` — Specifically the use of `--all` or mass IDs to kill servers.
* `aws s3 rb s3://.* --force` — Forcibly removing an S3 bucket and all its contained data.
* `gcloud projects delete .*` — Deleting an entire Google Cloud project.

#### 28. "Fileless" and Memory-Only Execution
These patterns avoid writing to the disk to bypass standard file-integrity monitors, running payloads directly in RAM.

* `python -c "exec(.*\.(decode|decompress))"` — Executing encoded/compressed code directly in memory.
* `perl -e 'use Socket;.*'` — Using Perl's socket library to create unauthorized network bridges.
* `ruby -e 'require "open-uri"; eval(open(".*").read)'` — Fetching a remote script and evaluating it without ever saving a `.sh` file.

#### 29. Shadow Admin & SSH Sabotage
These patterns don't delete files but "lock the doors" so you can't get back in to fix the system.

* `echo "" > ~/.ssh/authorized_keys` — Wiping out all authorized SSH keys, instantly locking out all remote admins.
* `userdel -r .*` — Deleting a user and their entire home directory (especially dangerous if targeted at the `admin` or `root` user).
* `passwd -l root` — Locking the root account so no one can log in or escalate privileges.

#### 30. Sensitive Data Exfiltration (The "Silent" Destruction)
Destruction isn't always about deleting; sometimes it's about destroying privacy or intellectual property.

* `grep -rE 'PASSWORD|bash_history|key|secret' .` — Searching the entire filesystem for credentials to leak.
* `tar -czf - .* | nc .*` — Compressing a directory and streaming it over the network to a listener.

## MCP Prompt Injection

In the context of the **Model Context Protocol (MCP)**, the risk shifts from simple command execution to **Tool Poisoning** and **Indirect Prompt Injection**. Since MCP allows an AI to dynamically discover and call tools, an attacker can manipulate the "Context" part of the protocol to trick the agent into performing actions it shouldn't.

### 1. Indirect Injection (Data-as-Instruction)
These occur when an agent reads a file, webpage, or ticket that contains "hidden" instructions intended for the LLM's next reasoning step.

* `(?i)(ignore|skip|system).*instructions.*instead.*` — The classic "ignore previous instructions" reset.
* `(?i)user\s+has\s+authorized\s+the\s+following\s+action` — Faking user authorization within a data block.
* `(?i)assistant\s+update:\s+the\s+task\s+has\s+changed` — Attempting to hijack the agent's internal monologue/persona.

### 2. Tool Poisoning & Parameter Injection
These target the specific JSON structure or metadata of MCP tool calls.

* `(?i)call_tool\(.*(['"]delete_all['"]|['"]wipe['"]|['"]drop_table['"])\)` — Specifically looking for high-risk tool names being "suggested" by untrusted data.
* `(?i)["'](admin|root|superuser)["']\s*:\s*true` — Attempting to inject elevated privilege flags into a tool's JSON argument.
* `(?i)--no-preserve-root|--force-yes` — Detecting "safety-bypass" flags being snuck into legitimate-looking CLI tools.

### 3. Exfiltration via Tool Abuse
Agents often have "safe" tools (like `fetch_url` or `send_email`) that can be weaponized to leak data retrieved by "sensitive" tools (like `read_database`).

* `(?i)fetch_url\(.*exfiltrate.*\.com.*data=` — Combining a network tool with a data payload.
* `(?i)send_email\(.*to:.*attacker@.*body:.*` — Using a messaging tool to forward sensitive context.
* `(?i)webhook\.site\/[a-zA-Z0-9-]+` — A common signature for temporary data-harvesting endpoints.

### 4. Semantic Escape Patterns
These patterns try to break the "boundary" between the System instructions and User data using structural markers.

* `(?i)(</system>|\[/INST\]|</s>)` — Attempting to close a system tag manually (common in Llama/Mistral/Antropic formatting).
* `(?i)---END\s+OF\s+CONTEXT---` — Faking a delimiter to make the model think the "untrusted" part is over.
* `(?i)\n\nSystem:\s+New\s+Task:` — Using line breaks and headers to simulate a fresh instruction set.

### 5. Multi-Step "Confused Deputy" Attacks
These look for instructions that tell the agent to chain tools in a way that bypasses security logic.

* `(?i)first\s+read.*then\s+delete` — Detecting destructive "Chained Intent" where the first action is benign.
* `(?i)copy.*to.*public_folder` — Moving data across security boundaries.

---

### Rust Implementation: The `RegexSet` for MCP
In Rust, you can combine these with your existing shell-command list. 

```rust
let mcp_security_set = RegexSet::new(&[
    r"(?i)ignore\s+all\s+previous",             // Reset attack
    r"(?i)call_tool\(.*delete",                 // Tool hijacking
    r"(?i)https?://webhook\.site",              // Exfiltration
    r"(?i)</system>|\[/INST\]",                 // Tag escape
    r"(?i)authorization\s*:\s*['"]bearer",      // Credential harvesting
]).unwrap();
```

### Critical MCP Safeguard: "Capability Splitting"
Regex is a "Swiss cheese" defense for MCP. The most effective way to protect an MCP agent in Rust is **Capability Splitting**:

1. **Read-Only Agent:** One agent has the `list_files` and `read_file` tools but **no** network access and **no** write access.
2. **Writer Agent:** A second, isolated agent receives the *processed summary* from the first and only has permission to call `write_file` or `git_commit`.
3. **Human-In-The-Loop (HITL):** For any tool call involving a "Write" or "Delete" operation, your Rust hook should pause execution and wait for a `stdin` confirmation or a Signal.



Would you like a sample of how to implement a **Human-in-the-loop** gate for your Rust CLI agent?


## Contextual Awareness

If you are building this for an AI Agent, **Regex alone will eventually fail** because an Agent can write a Python script that performs these actions via a library (like `shutil.rmtree('/')`) instead of a shell command. 

To be truly safe, your "hook" system should:

1. **Monitor syscalls** (like `unlink`, `rmdir`, `mount`).
2. **Use a Sandbox:** Run the Agent in a non-persistent Docker container with no network access unless explicitly required.
3. **Human-in-the-loop:** Any command matching these patterns should require a manual "Yes/No" from a human supervisor.

Are you planning to implement these in a specific language, or would you like to see how to wrap these into a Python-based execution guard?



## Implementation Tips for your RegEx
When converting these to actual code, consider the following:

* **Case Sensitivity:** Ensure your regex engine is set to case-insensitive (`/i`).
* **Whitespace:** Use `\s+` instead of literal spaces to catch commands like `rm    -rf`.
* **Sudo:** Prepend `(sudo\s+)?` to your patterns, as most of these require root privileges to succeed.
* **False Positives:** `rm -rf` is a common developer command for cleaning `node_modules` or `build` folders. You might want to scope your "hook" to only trigger if the path is a system directory (like `/etc`, `/var`, or `/usr`).

