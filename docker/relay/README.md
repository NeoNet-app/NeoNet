# Deploying a NeoNet Relay Node

A relay node federates with other relays and routes encrypted messages
for clients behind NAT. It registers on a rendezvous server so other
relays can discover it by domain name.

## Quick Start

### 1. Configure

```bash
cp .env.example .env
nano .env
# NEONET_DOMAIN      = your public domain (e.g. neonet.example.com)
# NEONET_RENDEZVOUS  = rendezvous server address (e.g. 1.2.3.4:7777)
# NEONET_PASSPHRASE  = strong passphrase for the keystore
```

### 2. Initialize the keystore

```bash
docker run --rm -it \
  -v neonet-relay-data:/.neonet \
  --env-file .env \
  ghcr.io/neonet-app/neonet-relay:latest \
  init
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
Enregistre sur <rendezvous> comme <domain>
Daemon pret.
```

## Firewall

Open UDP port 7777 (QUIC).

## DNS

Point your domain to the server's public IP:
```
neonet.example.com.  A  <YOUR_SERVER_IP>
```

## Volumes

| Path | Content |
|------|---------|
| `/data/keystore/` | Ed25519 identity (encrypted with passphrase) |
| `/data/neonet.db` | TOFU peer database |

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `NEONET_PASSPHRASE` | Yes | Keystore passphrase |
| `NEONET_DOMAIN` | Yes | Public domain for this relay |
| `NEONET_RENDEZVOUS` | Yes | Rendezvous server `host:port` (comma-separated for multiple) |
| `NEONET_LOG` | No | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `NEONET_LISTEN_PORT` | No | QUIC listen port (default: 7777) |
