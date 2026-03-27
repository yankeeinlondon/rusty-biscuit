# Claudine Sequences

## Overview

The command `claudine sequence <file>` starts a state machine we call a **sequence** and builds off the composition features seen in `compose` and `inline-compose`:

### Sequence Frontmatter

- the sequence's **steps** are defined in the `sequence` frontmatter property of the file reference
    - if the sequence property doesn't exist then we will exit with an error
    - a valid `sequence` definition take the form of a YAML _list_ of **objects**
- a sequence is also _allowed_ but not _required_ to define **parameters** it will need to be run
    - the `parameters` property is used to define properties
    - the `parameters` property is a YAML list of objects when defined
- a log file to track the progress of the sequence is not specifically required but considered a good practice
    - the Markdown file defining the sequence can define a `log` frontmatter property
    - if this is done then this suggested filename will be used unless the file already exists
    - if this file already exist then an simple `-{#}.md` will be added to the filename's ending until a unique value is found.
    - a good default log name uses dynamic properties like `{{ctx.today}}/{{topic}}/log.md`


- a Markdown which defines a sequence acts as a "template" and when executed with `claudine sequence` it's first action is to create a log file which will guide the sequence through it's defined steps.
- this log file 




- each item in the list represents a "stage" in the state machine
- each stage is defined by an object of the shape:

    ```ts
    type ShellCommand = {
        /** the shell command along with params */
        command: string;

    }

    type Agent = {
        agent: "claude" | "codex" | "gemini" | "opencode" | "qwen" | "etc.";

    }

    type FileReference = {
        /** 
         * the file reference to a Markdown file to be used as the prompt for this stage in the sequence 
         */
        file: string;

        /** 
         * - set key/values for the initial Frontmatter state when on a `File` operation.
         * - passed as a dictionary parameter to the 
         */
        set?: Record<string, any>;

        /**
         * - provides default key/values for the frontmatter but if document 
         */
        defaults?: Record<string, any>;
    }

    type Stage = {
        /** the name of the state; every state must have a name */
        name: string;
        /**
         * either a file reference or shell command (based on `kind`)
         */
        action: FileReference | ShellCommand;

        /**
         * You may optionally specify a short name (usually a single word)
         * to represent the _operation_ being performed at this stage
         */
        operation?: string;

        /**
         * Allows the sequence to express that this stage is an _optional_
         * stage. By default, stages are required.
         */
        optional?: boolean;
    }
    ```

