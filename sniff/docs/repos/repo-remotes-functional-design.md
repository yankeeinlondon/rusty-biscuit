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
3. `documentation`
   - identifies all the Markdown and text files in the document
   - initial call should just get list of available documents
   - user should be able to as for details on the subset of these documents or a category of documents where the categories are:
       - readme_docs - any case-ignored variant of a `README.md` file
       -
