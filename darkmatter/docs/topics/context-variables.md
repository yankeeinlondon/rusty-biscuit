# Context Variables

Context variables are variables which Darkmatter provides to the **Interpolation** process as a key/value dictionary under the name of `ctx`.

- it is recommended that document authors not use the `ctx` frontmatter variable because of the namespace collision it causes
- However, when composing a document with `md compose` if the document DOES have a `ctx` property defined then we will merge the two dictionaries; the Darkmatter's values will take president over the page's when `ctx` keys overlap
- we will report to STDERR this event using the `Status` struct (from `biscuit-terminal`) using the "warning" state and a message of:
    - `the document <a href={absolute-filepath}><blue-500>{relative-filepath}</blue-500></a> is using the <inverse>ctx</inverse> property; results were merged with <b>Darkmatter</b>'s <inverse>ctx</inverse> information (<dim><i>all keys were preserved</i></dim>).`
    - `the document <a href={absolute-filepath}><blue-500>{relative-filepath}</blue-500></a> is using the <inverse>ctx</inverse> property; results were merged with <b>Darkmatter</b>'s <inverse>ctx</inverse> information (<dim><i>document keys were overwritten</i></dim>).`

    The status message shown (from above) depends on whether there was a key collision or not.
- if there is a `ctx` property defined on the page that is _not_ a dictionary then we will:
    - by default we'll return an error to STDOUT using the `Status` struct and stop composition returning an error exit code:
        - ``
    - if the user uses the `--allow-ctx-override` CLI switch when composing a document we will change the error above to a warning and proceed with the composition


## Timing in Compose

When composing a document graph, we should only calculate the context once and use it across the full graph of documents. 

- this is more efficient
- it also ensures that we have the same date/time info throughout the composed document

## Information Provided

We will now provide a grouped overview of all the information stored in **Darkmatter**'s `ctx` variable:

> **Note:** all date and time related information is reporting using _local_ time but there will be a `_utc` variant that provides the same utility only uses UTC time to resolve.

### Date

- `today` 
    - provides an ISO Date string for the date when this was rendered (`YYYY-MM-DD` format); uses local time to calculate the date
    - has `today_utc` variant
- `yesterday`
    - provides an ISO Date string for yesterday's date (`YYYY-MM-DD` format); uses local time to calculate the date
    - has `yesterday_utc` variant
- `tomorrow`
    - provides an ISO Date string for tomorrow's date (`YYYY-MM-DD` format); uses local time to calculate the date
    - has `tomorrow_utc` variant
- `start_of_week_sun`
    - provides the date (`YYYY-MM-DD`) for the start of the given week (based on the week starting on Sunday)
    - has `start_of_week_sun_utc` variant
- `start_of_week_mon`
    - provides the date (`YYYY-MM-DD`) for the start of the given week (based on the week starting on Monday)
    - has `start_of_week_mon_utc` variant
- `end_of_week_sun`
    - provides the date (`YYYY-MM-DD`) for the end of the given week (based on the week ending on Sunday)
    - has `end_of_week_sun_utc` variant
- `end_of_week_mon`
    - provides the date (`YYYY-MM-DD`) for the end of the given week (based on the week ending on Monday)
    - has `end_of_week_mon_utc` variant

### Date and Time

- `now` - provides an ISO Datetime string for the host's locale (`YYYY-MM-DD hh:mm:ss.xxxT...`)
- `now_utc` - provides an ISO Datetime string for the UTC time when this was rendered (`YYYY-MM-DD hh:mm:ss.xxxTZ`)

## Time

- `time` - provides time in a `hh:mm a` format (e.g., `12:43 am`, `1:15 pm`, etc.)
- `time_military` - provides time based on a 24 hour clock format (e.g., `22:30`, `9:45`, etc.)

### Calendar

- `day` (NOTE: this used to be `dow`) 
    - the day of the week (e.g., Monday, Tuesday, etc.)
    - has `day_utc` variant
- `day_abbr`
    - an abbreviation for the day of the week (e.g., Mon, Tue, etc.)
    - has `day_abbr_utc` variant
- `year`
    - Shows the current year, based on local time
    - has `year_utc` variant
- `day_of_month` - the numeric value for the day of the month
- `day_of_month_suffixed` - the numeric value for the day of the month plus the appropriate suffix (1st, 2nd, 3rd, etc.)
- `month` - the numeric value for today's month
- `month_name` - the name for today's month (e.g., January, February, etc.)
- `month_name_abbr` - an abbreviated name for today's month (e.
- `season` - provides appropriate value in `Season` enumeration of `["Summer", "Spring", "Fall", "Winter"]

### Timestamps

- `timestamp`
    - provides an EPOCH timestamp in seconds
- `timestamp_ms`
    - provides an EPOCH timestamp in milliseconds

### Filesystem and Git

> **Note:** most if not all of the discovery in this section leverages the `sniff` library
> **Note:** rendering of markdown unordered lists will leverage `biscuit-terminal`'s `UnorderedList` component

- `repo`
    - provides the name of the current repo (based on where `md compose` is run from)
    - null if not in a repo
- `is_monorepo`
    - provides a boolean expressing whether the current repo is a monorepo
- `packages`
    - provides a list of _packages_ for the current repo 
    - null if not a monorepo
    - null if not in a repo
- `package_areas`
    - provides a list of _packages areas_ for the current repo 
    - null if not a monorepo
    - null if not in a repo
- `current_package`
    - always returns `null` if not a monorepo or not in a repo at all
    - returns `null` if in a monorepo but _not_ in a package's directory tree
    - provides the name of the current package (based on where `md compose` is run from)
- `current_package_area`
    - always returns `null` if not a monorepo or not in a repo at all
    - returns `null` if in a monorepo but _not_ in a package area's directory tree
    - provides the name of the current package area (based on where `md compose` is run from)

- `dirty_files`
    - provides a comma separated list of files that are "dirty" (aka, have changed since last commit or are untracked)
- `dirty_files_list`
    - provides a list of files that are "dirty" (aka, have changed since last commit or are untracked) as a Markdown Unordered List
- `dirty_source_code_files`
    - provides a comma separated list of source code files that are "dirty" (aka, have changed since last commit or are untracked)
- `dirty_source_code_files_list`
    - provides a list of source code files that are "dirty" (aka, have changed since last commit or are untracked) as a Markdown Unordered List
- `staged_files`
    - provides a comma separated list of files that are _staged_ to be committed
- `staged_files_list`
    - provides a list of files that are _staged_ to be committed as a Markdown unordered list
- `untracked_files`
    - provides a comma separated list of files that are _untracked_ to be committed
- `untracked_files_list`
    - provides a list of files that are _untracked_ to be committed as a Markdown unordered list

- `dirty_packages`
    - provides a comma separated list of packages that are "dirty" (aka, have changes since last commit)
- `dirty_packages_list`
    - provides a list of packages that are "dirty" (aka, have changes since last commit) as a Markdown Unordered List

- `dirty_package_areas`
    - provides a comma separated list of package areas that are "dirty" (aka, have changes since last commit)
- `dirty_package_areas_list`
    - provides a list of package areas that are "dirty" (aka, have changes since last commit) as a Markdown Unordered List

- `staged_packages`
    - provides a comma separated list of packages that are "dirty" (aka, have changes since last commit)
- `staged_packages_list`
    - provides a list of packages that are "dirty" (aka, have changes since last commit) as a Markdown Unordered List

- `staged_package_areas`
    - provides a comma separated list of package areas that have "staged" files (aka, have changes since last commit)
- `staged_package_areas_list`
    - provides a list of package areas that have "staged" files (aka, have changes since last commit) as a Markdown Unordered List

- `current_package_has_staged_files`
    - boolean flag which indicates whether the current package has staged files
    - always false if not monorepo or not in a repo
    - always false if CWD is not in a package directory tree
- `current_package_area_has_staged_files`
    - boolean flag which indicates whether the current package area has staged files
    - always false if not monorepo or not in a repo
    - always false if CWD is not in a package area directory tree

- `current_package_has_dirty_files`
    - boolean flag which indicates whether the current package has dirty files
    - always false if not monorepo or not in a repo
    - always false if CWD is not in a package directory tree
- `current_package_area_has_dirty_files`
    - boolean flag which indicates whether the current package area has dirty files
    - always false if not monorepo or not in a repo
    - always false if CWD is not in a package area directory tree
