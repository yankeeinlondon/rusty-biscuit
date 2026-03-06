# Stage 1: Install the musl target and cache the toolchain layer
FROM rust:1.88-alpine AS toolchain
RUN apk add --no-cache musl-dev

# Stage 2: Build dependencies (cached unless Cargo.toml/lock change)
FROM toolchain AS deps

WORKDIR /build

# Copy workspace root manifests
COPY Cargo.toml Cargo.lock ./

# Copy real Cargo.toml files for the crates we actually compile
COPY homelab/arcam-amp-integration/Cargo.toml homelab/arcam-amp-integration/Cargo.toml
COPY homelab/lib/Cargo.toml homelab/lib/
COPY schematic/schema/Cargo.toml schematic/schema/
COPY schematic/define/Cargo.toml schematic/define/
COPY schematic/definitions/Cargo.toml schematic/definitions/
COPY biscuit-file/lib/Cargo.toml biscuit-file/lib/

# Stub out every other workspace member with the correct crate name.
# This lets Cargo resolve the workspace without copying all source.
RUN set -e && stub() { mkdir -p "$1/src" && printf '[package]\nname = "%s"\nversion = "0.0.0"\nedition = "2024"\n' "$2" > "$1/Cargo.toml" && : > "$1/src/lib.rs"; } && \
    stub biscuit-file/cli       biscuit-file-cli && \
    stub biscuit-hash/cli       biscuit-hash-cli && \
    stub biscuit-hash/lib       biscuit-hash && \
    stub biscuit-speaks/cli     biscuit-speaks-cli && \
    stub biscuit-speaks/lib     biscuit-speaks && \
    stub biscuit-terminal/cli   biscuit-terminal-cli && \
    stub biscuit-terminal/lib   biscuit-terminal && \
    stub claudine/cli           claudine-cli && \
    stub claudine/lib           claudine && \
    stub darkmatter/cli         darkmatter-cli && \
    stub darkmatter/lib         darkmatter && \
    stub homelab/cli            homelab-cli && \
    stub homelab/server         homelab-server && \
    stub model-citizen/cli      model-citizen-cli && \
    stub model-citizen/lib      model-citizen && \
    stub unchained-ai/model_id  model_id && \
    stub unchained-ai/cli       unchained-ai-cli && \
    stub unchained-ai/gen       unchained-ai-gen && \
    stub unchained-ai/lib       unchained-ai && \
    stub playa/cli              playa-cli && \
    stub playa/lib              playa && \
    stub queue/cli              queue-cli && \
    stub queue/lib              queue && \
    stub research/cli           research-cli && \
    stub research/lib           research && \
    stub schematic/gen          schematic-gen && \
    stub sniff/cli              sniff-cli && \
    stub sniff/lib              sniff && \
    stub tabby                  tabby && \
    stub tabby/ui               ui && \
    stub tui                    tui && \
    stub tree-hugger/cli        tree-hugger-cli && \
    stub tree-hugger/lib        tree-hugger && \
    stub agent-sandbox/cli      agent-sandbox-cli

# Create dummy main so cargo can build & cache all dependencies
RUN mkdir -p homelab/arcam-amp-integration/src && \
    echo 'fn main() {}' > homelab/arcam-amp-integration/src/main.rs

# Stub lib.rs for each real dependency so the dep tree compiles
RUN for d in homelab/lib schematic/schema schematic/define schematic/definitions biscuit-file/lib; do \
      mkdir -p "$d/src" && : > "$d/src/lib.rs"; \
    done

RUN cargo build --release -p arcam-amp-integration 2>/dev/null || true

# Stage 3: Build the real binary (only source changes bust this layer)
FROM deps AS builder

# Copy real source for the crates we need
COPY homelab/lib/ homelab/lib/
COPY schematic/schema/ schematic/schema/
COPY schematic/define/ schematic/define/
COPY schematic/definitions/ schematic/definitions/
COPY biscuit-file/lib/ biscuit-file/lib/
COPY homelab/arcam-amp-integration/src/ homelab/arcam-amp-integration/src/

# Touch source files so cargo sees them as newer than the cached dummy
RUN find homelab/arcam-amp-integration/src homelab/lib/src schematic/schema/src schematic/define/src schematic/definitions/src biscuit-file/lib/src -name '*.rs' -exec touch {} +

RUN cargo build --release -p arcam-amp-integration && \
    strip /build/target/release/arcam-amp-integration

# Stage 4: Minimal runtime image (~7 MB)
FROM scratch

COPY --from=builder /build/target/release/arcam-amp-integration /arcam-amp-integration

EXPOSE 9090

ENTRYPOINT ["/arcam-amp-integration"]
