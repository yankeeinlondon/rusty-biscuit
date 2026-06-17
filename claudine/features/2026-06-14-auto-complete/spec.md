# Shell Completions and Autocomplete Feature

## Headline Features

In this feature we will add two headline features:

1. **Shell Completions**
2. **Autocomplete**

Claudine already does do _shell completions_ but it's quite sub-optimal and this feature attempts to take it to a production ready state.

### Shell Extensions

Shell completions are extremely important for Claudine:

1. the allow a user to explore the API surface and not always be referring to documentation to understand what is available
2. when we run composition features of Claudine (compose, inline-compose, sequence) we refer to a Markdown document (or sometimes a YAML file) and we need to help the user resolve this file so that no accidental spelling mistakes creep in but also so that kicking off a job can be done as quickly as possible

### Autocomplete

