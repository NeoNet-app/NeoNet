# Deploying a NeoNet Rendezvous Node

A rendezvous node is a lightweight directory that helps relays find each other.
It does not store messages or user data.

## Quick Start

### 1. Configure

```bash
cp .env.example .env
nano .env
# Set NEONET_PASSPHRASE to a strong passphrase
```

### 2. Initialize the keystore

```bash
docker run --rm -it -v neonet-rdv-data:/data \
  -e NEONET_PASSPHRASE="$NEONET_PASSPHRASE" \
  ghcr.io/neonet-app/neonet-rendezvous:latest \
  /neonet init
```

### 3. Start

```bash
docker compose up -d
```

### 4. Verify

```bash
docker compose logs -f
```

You should see:
```
QUIC listener started on 0.0.0.0:7777
En ecoute (rendezvous) sur 0.0.0.0:7777
Daemon pret.
```

## Firewall

Open UDP port 7777 (QUIC).

## Volumes

| Path | Content |
|------|---------|
| `/data/keystore/` | Ed25519 identity (encrypted with passphrase) |
| `/data/neonet.db` | TOFU peer database |
