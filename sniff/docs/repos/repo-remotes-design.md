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

And we will provide a enum called `GitRemote` which points to each of the remote implementations we un
