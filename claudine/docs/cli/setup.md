---
kind: canonical-documentation
blast_radius:
  - claudine/cli/src/commands/setup.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/lib/src/config/setup.rs
---
# `claudine setup`

`claudine setup` sets Claudine up with Darkmatter: it installs the schema
files `md` and your editor use to understand Claudine documents, points
Darkmatter's permissions at Claudine's configuration, and registers those
files in `~/.darkmatter.yaml`. A first `claudine` command does the same
automatically; run `claudine setup` yourself when:

- `md` or your editor warns that `~/.claudine/darkmatter-host.yaml` is a
  **format 1** `darkmatter-host` manifest. That file is the registration an
  earlier Claudine release installed; setup replaces its entry and removes it.
- `~/.darkmatter.yaml` lost Claudine's `register` entry.

```sh
claudine setup
```

It takes no arguments and never asks a question, so a script or an upgrade
can run it. Running it again changes nothing.

## What it changes

With an existing Claudine configuration, every setting and comment you wrote
is kept. Setup runs four steps, in order:

1. installs the registration under `~/.claudine/registrations/darkmatter/`
   (unchanged files are not rewritten); if this fails, it stops with an error
   and changes no configuration;
2. completes the `permissions` handshake between `~/.claudine/config.yaml`
   and `~/.darkmatter.yaml`. Two different blocks are a warning, and both are
   left as they are;
3. writes `~/.claudine/registrations/darkmatter/manifest.yaml` into the
   `register` list of `~/.darkmatter.yaml`, in place of any entry naming
   `~/.claudine/darkmatter-host.yaml`, even when that file is already gone;
4. removes `~/.claudine/darkmatter-host.yaml` and the
   `~/.claudine/darkmatter-host/` folder.

```yaml
# ~/.darkmatter.yaml before
register:
    - ~/hosts/mine.yaml
    - ~/.claudine/darkmatter-host.yaml
# after claudine setup
register:
    - ~/hosts/mine.yaml
    - ~/.claudine/registrations/darkmatter/manifest.yaml
```

A repository's `.darkmatter.yaml` is never changed. If it lists the old
manifest, `md` warns at that file once setup removed it; edit the entry
there (or remove it: the user entry already registers Claudine).

Without a Claudine configuration, setup first writes the same defaults a
first run without a terminal writes.

Changes are reported on stderr, followed by one line naming Claudine's
configuration file; stdout stays empty. The exit status is nonzero only when
a configuration file does not load, publishing fails, or Claudine's own
write fails; a `permissions` conflict or a registration that could not be
written is a warning.

The full rules, with a diagram of each step's failure path, are in
[Configuration → Darkmatter's permissions](../topics/configuration.md#darkmatters-permissions)
and [Schemas → When Claudine publishes and registers](../topics/schemas.md#when-claudine-publishes-and-registers).
To reinstall only the schema files, without touching either configuration
file, use [`claudine registrations publish`](./registrations.md).
