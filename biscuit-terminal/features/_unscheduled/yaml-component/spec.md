# YAML Component

The goal of this feature specification is to allow rendering YAML data for the terminal and the browser/HTML.

- this feature will center around the `YamlData` struct
- this struct will ergonomically receive YAML data from a string representation, a YAML file, even a Markdown file:

    ```rust
    YamlData::new<T: Into<String>>(yaml: T): Result<Self, YamlDataError>;
    /// returns a YamlData unless the passed in Markdown is invalid. if there's no markdown then
    /// YAML is an empty object.
    YamlData::from_markdown_content<T: Into<String>>(md: T): Result<Self, YamlDataError>;
    YamlData::from_markdown_file<T: Into<String>>(ref: T): Result<Self, YamlDataError>;
    YamlData::from_yaml_file<T: Into<String>>(ref: T): Result<Self, YamlDataError>;
    ```

- the `YamlData` struct will also implement the `Renderable` and `BrowserRenderable` traits so that it can be used in both terminal and browser ergonomically

- when implementing the `Renderable` trait its important that we use "code highlighting" and the theming support that biscuit-terminal provides.
    - default "theme" used to render the YAML block will be based on whether the terminal is using light/dark mode
    - some of the
