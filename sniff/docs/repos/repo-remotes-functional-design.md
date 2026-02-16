# Repo Remotes

The sniff library deals primarily with interrogating the host system for information but since we are already providing useful information about git repos (most of it local to the host but some remote info) we are going to extend this capability. Centering this more capable look at git remote hosts we'll focus on the following providers:

1. Github
2. Gitlab
3. Gitea
4. Bitbucket
5. AWS CodeCommit (stage 2)
6. Azure Devops (stage 2)

The Sniff Library will add a set of structs which are provider specific:

1. `GithubRepo`
2. `GiteaRepo`
3. `GiteaRepo`
4. `BitbucketRepo`
5. `CodeCommitRepo` (stage 2)
6. `AzureDevopsRepo` (stage 2)

And we will provide an enum called `GitRemoteInfo` which points to each of the remote implementations with appropriate mappings so that we can leverage features across all platforms in as similar a fashion as possible.

## API Surfaces

> **IMPORTANT:** all actual API interaction will leverage the API client's defined in `schematic/schema`

Broadly we want to be able to report on the following things:

1. `organization` - information about the organization which the repo belongs to
2. `repo_metadata`
      - high level metrics like "stars", "contributors", etc.,
      - but also `tags`, `releases`, `last_commit_date`, `last_commit_hash`, `branches`, etc.
      - also provides the `LICENSE` or `LICENSE.md` file at repo root if available and parses to identify the license information
      - also provides full text of the `README.md` (case insensitive) at the root of the repo if it exists
      - we should also be able to get the repo's "description" property and other useful meta
      - we should also be able to identify how many forks are known about (and list them if we want)
3. `documentation`
   - identifies all the Markdown and text files in the document
   - initial call should just get list of available documents
   - user should be able to as for details on the subset of these documents (or their choosing) or a _category_ of documents where the categories are:
       - readme_docs - any case-ignored variant of a `README.md` file
       - source_docs - any document contained within the `src` directory
4. `PRs`
   - we should be able to get a list of PR's on the repo, along with supporting metadata
5. `issues`
   - we should be able to get all or a filtered subset of the issues on the remote site for the repo
6. `wiki`

   - we should be able to identify whether the repo has a wiki attached or not
   - if it does we should be able to get a listing of all the pages
7. `ci-cd`

   - we should be able to get a list of the CI/CD jobs
   - we should be able to filter down to only those which are running
   - we should be able to filter down to only completed jobs (with a discrete number we want, starting from most recent)

8. `other_repos`
   - we should be able to list the other repos under the requested repo's organization
9.  `key_urls`
  - key URLs for the repo (repo homepage, issues page, wiki page, CI/CD page, insights page, etc.)

## Sniff CLI

We already have a subcommand `git` which reports on the repo in the current working directory of the host. What we will need to do is provide an optional parameter to specify the remote repo we're interested in.

- in cases where the remote parameter is specified we will switch instead to reporting the remote info
- in cases where the remote parameter is _not_ specified but the `--deep` flag is used then we will supplement the local view with information like PR's, Issues, etc.


