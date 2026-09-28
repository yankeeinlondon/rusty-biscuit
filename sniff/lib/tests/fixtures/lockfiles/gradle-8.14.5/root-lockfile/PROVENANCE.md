# Provenance: gradle-8.14.5/root-lockfile

- Tool: Gradle 8.14.5 (docker image `gradle:8-jdk17`, JVM 17.0.20.1 Temurin, linux/aarch64)
- Host: docker `gradle:8-jdk17` on macOS 27.2 (Darwin 27.2.0 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# same settings.gradle, app/build.gradle, lib/build.gradle as the sibling subproject-only fixture;
# the ROOT build.gradle additionally applies `java` and declares implementation 'org.slf4j:slf4j-api:2.0.13',
# with allprojects { dependencyLocking { lockAllConfigurations() } }
tar -cf - . | docker run -i --rm gradle:8-jdk17 sh -c 'mkdir /w && cd /w && tar xf - && gradle --no-daemon -q dependencies :app:dependencies :lib:dependencies --write-locks >/dev/null && tar cf - $(find . -name "*.lockfile" -not -path "./.gradle/*")' | tar -xf -
```

## Trimmed

- `.gradle/` and `build/` never left the container; only `*.lockfile` files were streamed back.

## Expected membership

- Declared by the manifest (`settings.gradle`, root excluded, sorted):
  - `app`
  - `lib`
- Recorded by the lockfile: none. Lines are `group:artifact:version=conf,...` plus `empty=conf,...`; subprojects and `project(':lib')` are not recorded.
- Root lock file(s): `gradle.lockfile`
- Sniff expectation: `unverifiable` + `no_membership_data`, with `paths` = the root lock file(s) listed above
  (the subproject lockfiles under `app/` and `lib/` are not candidates).
