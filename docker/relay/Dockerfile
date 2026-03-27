# ── Stage 1: Builder ──────────────────────────────────────────────────
FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y musl-tools pkg-config && \
    rustup target add x86_64-unknown-linux-musl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --bin neonet --target x86_64-unknown-linux-musl && \
    strip target/x86_64-unknown-linux-musl/release/neonet

# ── Stage 2: Runtime (scratch — zero attack surface) ──────────────────
FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/neonet /neonet

# Pre-baked relay config.
COPY --from=builder /build/docker/relay/config.toml /etc/neonet/config.toml

EXPOSE 7777/udp
VOLUME /data

ENV NEONET_PASSPHRASE=""
ENV NEONET_DOMAIN=""
ENV NEONET_RENDEZVOUS=""
ENV NEONET_LOG="info"
ENV NEONET_LISTEN_PORT="7777"

ENTRYPOINT ["/neonet"]
CMD ["start", "--mode", "relay", "--config", "/etc/neonet/config.toml"]
