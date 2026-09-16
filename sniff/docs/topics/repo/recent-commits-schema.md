## Recent Commits Schema

For each commit captured by Sniff's recent commits functionality you will get the following information:

```yaml
# Simplified Schema
$schema:
    datetime: datetime(required;eager) -> the date and time of the commit
    operation: string(suggest(fix,feat,doc,cicd,config,test)) -> if this follows the _conventional commits_ convention then the commit's operation will be listed
    scope: string -> if the commit follows the _conventional commits_ convention and uses a "scope" typically associated to a monorepo, then that will be found in the "scope" property
    hash: string(required;eager) -> the full hexadecimal git hash for this commit
    remote: boolean -> whether or not the commit has been pushed upstream to a cloud host
    commit_url: url -> the URL to the commit on the remote's site (if it's been pushed yet)
    heading: string(required;eager) ->  the first sentence of the commit (terminates on `.` character or `\n`).
    description: string(required;eager) -> the descriptive content for the commit after the first sentence (see _heading_) and before the bullet-points.
    bullet_points: string[](required;eager) -> the bullet points that detail out the commit message
    files: file[](required; eager; min(1))
    file_types:
        source_code: boolean(required;eager) -> boolean flag indicating whether any source code was touched in this commit
        web_assets: boolean(required;eager) -> boolean flag indicating whether any web assets (HTML, CSS, Font files, etc.) was touched in this commit
        images: boolean(required;eager) -> boolean flag indicating whether any images (including vectors like SVG) were touched in this commit
        documentation: boolean(required;eager) -> boolean flag indicating whether any documentation was touched in this commit
        configuration: boolean(required;eager) -> boolean flag indicating whether any configuration files were touched in this commit
        cicd: boolean(required;eager)
    
    packages: string[] -> the packages which were effected by the commit (**note:** this is only available when run in a monorepo)
    package_areas: string[](required;eager) -> the package areas which were effected by the commit (**note:** this is only available when run in a monorepo)
types:
    file: 
        kind: enum(modified,added,deleted,moved;required;eager)
        path: file(required;eager) -> the filepath to the mutated file
        original_path: file -> the original filepath in a "move" mutation
        added: number -> the number of lines added (_only set when `kind` is **modified**_)
        removed: number -> the number of lines removed (_only set when `kind` is **modified**_)
        symbols: string[] -> future reporting which list exported symbols in the file which were impacted; currently not populated
```
