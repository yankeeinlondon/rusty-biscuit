# Fixing Changelog

of all the underlying research documents we produce for the `research library <pkg>` CLI command, the one which is most fragile is the `changelog.md`

In this plan we will fix that.

## Suggestions on Building a Changelog

Building a high-quality changelog is about balancing developer efficiency with reader clarity. A "good view" of a repo's history depends entirely on the audience: developers need to know what changed in the code, while users need to know how those changes affect them.

Here is a deep dive into the three primary paths for documenting and viewing a repository's changelog.

---

### 1. The "Human-First" Path: `CHANGELOG.md`
This is the gold standard for open-source and user-facing projects. It follows the [Keep a Changelog](https://keepachangelog.com/) principles.

* **The Philosophy:** A changelog is for humans, not machines. Raw commit logs are noisy; a curated file is signal.
* **The Structure:**
    * **Grouped by Type:** Instead of a chronological list, changes are grouped into categories: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, and `Security`.
    * **Reverse Chronological:** Newest versions at the top.
    * **Unreleased Section:** A section at the very top to track changes that haven't been tagged yet.
* **Pros:** Extremely readable; provides context ("Why" instead of just "What"); searchable within the repo.
* **Cons:** High maintenance; prone to "human error" (forgetting to update it).

### 2. The "Automated" Path: Conventional Commits
If you want a changelog that builds itself, you must enforce a strict commit message format. The industry standard is [Conventional Commits](https://www.conventionalcommits.org/).

* **The Mechanism:** Commits must follow a pattern: `type(scope): description`.
    * *Example:* `feat(auth): add Google OAuth2 support` or `fix(ui): resolve mobile padding issue`.
* **The Tools:**
    * **semantic-release:** Fully automates the whole package release workflow including: determining the next version number, generating the changelog, and publishing.
    * **standard-version:** A tool for projects that want to automate the `CHANGELOG.md` generation but still want to trigger the release manually.
* **Pros:** Zero manual effort after setup; 100% consistency; links commits directly to the log.
* **Cons:** Requires team discipline; commit messages can become "robotic" and lose the "why" behind a change.

### 3. The "Platform-Native" Path: GitHub/GitLab Releases
Many modern teams skip the `CHANGELOG.md` file entirely in favor of the "Releases" tab on their hosting provider.

* **GitHub Releases:** Allows you to attach binaries, tag versions, and write markdown descriptions.
* **Automated Release Notes:** GitHub now has a "Generate release notes" button that automatically aggregates pull requests merged since the last release. It categorizes them based on labels (e.g., any PR with a `bug` label goes under "Bug Fixes").
* **Pros:** Integrated into the UI where users already look; supports rich media (screenshots/videos of new features); handles "Contributors" automatically.
* **Cons:** Locks you into a specific platform; harder to search across the entire history of the project compared to a single text file.

---

### Which path should you choose?

| If your project is... | Recommended Path |
| :--- | :--- |
| **A Library/SDK** | **Automated (Conventional Commits).** Developers care about breaking changes and versioning precision. |
| **An App/SaaS** | **Platform-Native + SaaS Tool.** Use GitHub Releases for the tech side and a tool like *Beamer* or *Headway* for a "What's New" widget in the app. |
| **Open Source Tool** | **Human-First (`CHANGELOG.md`).** Community members need a clear, curated history to understand the project's evolution. |

### The "Ultimate" Setup
For a professional-grade repository, the most effective strategy is a **Hybrid Approach**:

1. **Enforce Conventional Commits** using `commitlint` to ensure data quality.
2. **Use a CI/CD action** (like `conventional-changelog`) to automatically update a `CHANGELOG.md` file on every release.
3. **Sync to GitHub Releases**, using the generated markdown as the release body.
4. **Add a "Breaking Changes" section** manually to the top of the release notes to ensure users don't miss critical migrations.

By following this, you get the efficiency of automation with the clarity of human curation.

## Plan

- Build a strategy on how to inspect a repo for information assets you should mine for changelog information. 
- This process may be a multi-step process and that is ok; we want a high quality process

If after employing our best techniques we do not feel confident in the Changelog, then we need to at a minimum create a timeline which indicates what date the 1.0.0, 2.0.0, releases were pushed. If the repo hasn't hit 1.0.0 yet then track the _minor_ releases timeline.

Always make sure to state what the latest committed version is. The frontmatter should have `updated_at` and `created_at` properties as well as a `latest_version` property.


