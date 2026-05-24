# Filepath Resolution

After being _composed_ the file links -- such as that to [Darkmatter Pipeline](@darkmatter/docs/darkmatter-compose-pipeline.md) -- will be resolved to a relative path (in most cases) or an absolute path (if a portable relative path can't be found).

You can test that by running:

```sh
md compose example-docs/filepath-resolution/test.md
```

And you'll find that the magic path which had been expressed in this document `@darkmatter/docs/darkmatter-compose-pipeline.md` has been transformed to a relative path. 

Now, you often may find that where the source document you are composing is located in a directory but what you want to produce is a new "composed" document that will be located somewhere else. This can easily be adjusted by shifting the _base directory_ with the `--base <dir>` CLI switch. Doing this will still target converting _file references_ to relative paths which remain portable as Markdown content.

If instead, you want to change the file reference strategy to absolute paths, that too is possible, with the `--use-absolute-paths` CLI switch. However, the use of absolute paths is much more brittle and is not normally what you want.

> **Note:** ironically, if you pipe composed output from Darkmatter back into the Darkmatter renderer to display in the terminal then you'll need an OSC8 link which **requires** that the file reference be an absolute path:
>
> ```sh
> md compose example-docs/filepath-resolution/test.md | md
> ```
>
> The Darkmatter renderer is smart enough to be able to convert a file with a relative link into an absolute file references when rendered to the
> terminal so under normal circumstances (aka, when the Markdown content resides as a file in the filesystem) this will just work without issue
> but there is a small wrinkle when we use the compose command ... the Markdown content is just a string not a file and therefore it has no 


[test link](../../docs/darkmatter-compose-pipeline.md)
