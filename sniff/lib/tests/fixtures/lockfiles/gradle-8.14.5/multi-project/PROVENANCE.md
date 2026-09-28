# Provenance: gradle-8.14.5/multi-project

- Tool: Gradle 8.14.5 (docker image `gradle:8-jdk17`, JVM 17.0.20.1 Temurin, linux/aarch64)
- Host: docker `gradle:8-jdk17` on the macOS host above
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write settings.gradle (include ':app', ':lib'), root build.gradle (allprojects { dependencyLocking { lockAllConfigurations() } }),
# lib/build.gradle (java-library, api 'org.slf4j:slf4j-api:2.0.13'), app/build.gradle (java, implementation project(':lib'))
tar -cf - . | docker run -i --rm gradle:8-jdk17 sh -c 'mkdir /w && cd /w && tar xf - && gradle --no-daemon -q dependencies :app:dependencies :lib:dependencies --write-locks >/dev/null && tar cf - $(find . -name "*.lockfile" -not -path "./.gradle/*")' | tar -xf -
```

## Trimmed

- `.gradle/` and `build/` never left the container; only `*.lockfile` files were streamed back.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `app`
  - `lib`
- Recorded by the lockfile:
- Per-project `app/gradle.lockfile` and `lib/gradle.lockfile`; NO root `gradle.lockfile` (the root project
    has no configurations, so Gradle writes none). Lines are `group:artifact:version=conf1,conf2,...` plus
    `empty=conf,...`. Project dependencies (`project(':lib')`) are not recorded, so no membership data.

## Notes

A bare `gradle dependencies --write-locks` only ran the root project's `dependencies` task and wrote no
lockfile; each subproject's task had to be named. No version field exists in the format.
