FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build

RUN cat <<'EOF' > Cargo.toml
[workspace]
resolver = "2"
members = [
    "biscuit-file/lib",
    "homelab/arcam-amp-integration",
    "homelab/sony-receiver-integration",
    "homelab/eversolo-integration",
    "homelab/unfolded-integration-helper",
    "homelab/lib",
    "schematic/define",
    "schematic/definitions",
]
exclude = ["schematic/schema"]
EOF

COPY Cargo.lock ./
COPY biscuit-file/lib/ biscuit-file/lib/
COPY homelab/arcam-amp-integration/ homelab/arcam-amp-integration/
COPY homelab/sony-receiver-integration/ homelab/sony-receiver-integration/
COPY homelab/eversolo-integration/ homelab/eversolo-integration/
COPY homelab/unfolded-integration-helper/ homelab/unfolded-integration-helper/
COPY homelab/lib/ homelab/lib/
COPY schematic/schema/ schematic/schema/
COPY schematic/define/ schematic/define/
COPY schematic/definitions/ schematic/definitions/

RUN cargo build --release -p sony-receiver-integration && \
    strip /build/target/release/sony-receiver-integration

FROM alpine:3.22

RUN apk add --no-cache ca-certificates

COPY --from=builder /build/target/release/sony-receiver-integration /usr/local/bin/sony-receiver-integration

RUN cat <<'EOF' > /entrypoint.sh
#!/bin/sh
set -eu

exec /usr/local/bin/sony-receiver-integration \
  --host "${SONY_HOST:?Set SONY_HOST to your Sony receiver IP}" \
  --port "${SONY_PORT:-10000}" \
  --listen "0.0.0.0:${LISTEN_PORT:-9091}" \
  --device-name "${DEVICE_NAME:-receiver}" \
  --timeout "${TIMEOUT:-10}" \
  --poll-interval "${POLL_INTERVAL:-5}" \
  "$@"
EOF

RUN chmod +x /entrypoint.sh

EXPOSE 9091

ENTRYPOINT ["/entrypoint.sh"]
