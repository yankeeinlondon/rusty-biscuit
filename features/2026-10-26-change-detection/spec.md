# Better Evidence

Today we build "evidence" during our CI test runs to explicitly show that code passed and that evidence is used in the testing(ci), fuzz, nightly, and release processes in CI/CD.

The evidence is stored in git commit _notes_ and they are queried -- awkwardly -- in today's process. This spec is about defining an explicit schema and data model in Cloudflare's [D1](https://developers.cloudflare.com/d1/) database.

This D1 based solution will:

- provide a much cleaner query interface for the business logic of the jobs which interact with evidence
- allow more durable state (github notes last 90 days)
- cleaner separation of the git repo from operational state

This D1 solution, will need to consider security as an important aspect and ensure that only authorized jobs are able to interact with this D1 instance.
