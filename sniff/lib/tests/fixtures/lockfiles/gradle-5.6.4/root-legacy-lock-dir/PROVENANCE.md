# Provenance: gradle-5.6.4/root-legacy-lock-dir

- Tool: Gradle 5.6.4 (official bin distribution, run in docker `eclipse-temurin:11-jdk`, JVM 11.0.32.1, linux/aarch64)
- Host: docker `eclipse-temurin:11-jdk` on macOS 27.2 (Darwin 27.2.0 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# same settings.gradle, app/build.gradle, lib/build.gradle as the sibling subproject-only fixture;
# the ROOT build.gradle additionally applies `java` and declares implementation 'org.slf4j:slf4j-api:2.0.13',
# with allprojects { dependencyLocking { lockAllConfigurations() } }
tar -cf - . | docker run -i --rm eclipse-temurin:11-jdk sh -c 'apt-get -qq update; apt-get -qq install -y unzip; cd /opt && curl -sSLo g.zip https://services.gradle.org/distributions/gradle-5.6.4-bin.zip && unzip -q g.zip && mkdir /w && cd /w && tar xf - && /opt/gradle-5.6.4/bin/gradle --no-daemon -q dependencies :app:dependencies :lib:dependencies --write-locks >/dev/null && tar cf - $(find . -name "*.lockfile" -not -path "./.gradle/*")' | tar -xf -
```

## Trimmed

- `.gradle/` and `build/` never left the container; only `*.lockfile` files were streamed back.

## Expected membership

- Declared by the manifest (`settings.gradle`, root excluded, sorted):
  - `app`
  - `lib`
- Recorded by the lockfile: none. Each file is a 3-line comment header then one `group:artifact:version` per line (empty configurations have only the header).
- Root lock file(s): the 14 files directly under `gradle/dependency-locks/`: `annotationProcessor.lockfile`, `archives.lockfile`, `compile.lockfile`, `compileClasspath.lockfile`, `compileOnly.lockfile`, `default.lockfile`, `runtime.lockfile`, `runtimeClasspath.lockfile`, `testAnnotationProcessor.lockfile`, `testCompile.lockfile`, `testCompileClasspath.lockfile`, `testCompileOnly.lockfile`, `testRuntime.lockfile`, `testRuntimeClasspath.lockfile`
- Sniff expectation: `unverifiable` + `no_membership_data`, with `paths` = the root lock file(s) listed above
  (the subproject lockfiles under `app/` and `lib/` are not candidates).
