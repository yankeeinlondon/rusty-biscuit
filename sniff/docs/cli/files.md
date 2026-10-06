# File Association Reports

Use `sniff files` to count files by category, such as images, documentation,
or programming languages. Filter the displayed categories with `--association`:

```sh
sniff files
sniff files --association image
sniff files --association image -v
sniff files --association image --json
```

The percentage uses all classified files in the scan as its denominator. For
example, three images among twelve files appear as `3 (25.0%)`, including when
only the image category is displayed. `-v` adds language or framework details
where applicable; it does not change the scan scope, counts, or percentages.
Filtered JSON's `total_files` counts matching files, while each category's
percentage retains the denominator of the full scan.

The default base directory is the current directory. If it belongs to a known
package, Sniff scans that package's root and excludes nested packages. Otherwise,
it scans the base directory and its descendants. Override the base with
`--base`:

```sh
sniff --base ./biscuit-terminal files --association image
sniff --base ./biscuit-terminal/lib/src files --association image
```

The first command scans the area directory, including its library and CLI. The
second scans the owning library package. Repository membership is discovered
without classifying unrelated repository files. Git ignore rules and Sniff's
generated-directory exclusions apply to the scan.

Classification is capped at 10,000 files in the requested scope. A complete
scan has stable results for an unchanged tree. If the scope itself exceeds
the cap, parallel workers select a partial sample that can vary between runs.
Text output displays an **Incomplete scan** notice; JSON includes
`"truncated": true` and `"limit": 10000`. Counts and percentages then describe
the sample, so an empty filtered table does not prove the category is absent.
Use `--base` to select a smaller scope when you need a complete count.
