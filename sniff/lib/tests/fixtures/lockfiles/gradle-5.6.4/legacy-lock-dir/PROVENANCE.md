# Provenance: gradle-5.6.4/legacy-lock-dir

- Tool: Gradle 5.6.4 (official bin distribution, run in docker `eclipse-temurin:11-jdk`, JVM 11.0.32.1, linux/aarch64)
- Host: docker `eclipse-temurin:11-jdk` on the macOS host above
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# same build scripts as gradle-8.14.5/multi-project
docker run --platform linux/amd64 gradle:5.6-jdk8 ...   # FAILED: JVM SIGSEGV under amd64 emulation (no arm64 image exists)
tar -cf - . | docker run -i --rm eclipse-temurin:11-jdk sh -c 'apt-get -qq update; apt-get -qq install -y unzip; cd /opt && curl -sSLo g.zip https://services.gradle.org/distributions/gradle-5.6.4-bin.zip && unzip -q g.zip && mkdir /w && cd /w && tar xf - && /opt/gradle-5.6.4/bin/gradle --no-daemon -q dependencies :app:dependencies :lib:dependencies --write-locks >/dev/null && tar cf - $(find . -name "*.lockfile" -not -path "./.gradle/*")' | tar -xf -
```

## Trimmed

- `.gradle/` and `build/` never left the container; only `*.lockfile` files were streamed back.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `app`
  - `lib`
- Recorded by the lockfile:
- Legacy layout: `<project>/gradle/dependency-locks/<configuration>.lockfile`, one file per configuration
    (14 per project), each a 3-line comment header then one `group:artifact:version` per line (empty
    configurations have only the header). No membership, no version field.
