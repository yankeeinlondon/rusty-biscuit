# Research's Show Command

The Research CLI should have a `show` command which will show a specified topic. This will be achieved by using the `open` crate to open the `deep_dive.md` file for the specified topic.

## Syntax
> **research show** \<topic\>

To start we will not need any CLI switches for "show".

## Notes

- While currently we only have the "library" type of research, more types will be added.
- The best way to identify the valid set of topics is to use a glob pattern `{root}/**/deep_dive.md` where `root` is by default the `~/.research/` this will identify every Deep Dive document we have and the directory it is in is the name of the topic.
