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

### 2. Start

```bash
docker compose up -d
```

That's it. On first start, if no keystore is found in `./neonetconf/`,
the container automatically initialises a new Ed25519 identity using
`NEONET_PASSPHRASE` from the `.env` file.

### 3. Verify

```bash
docker compose logs -f
# → Auto-init : keystore créé
# → QUIC listener started on 0.0.0.0:7777
# → En écoute (rendezvous) sur 0.0.0.0:7777
# → Daemon prêt.
```

## Firewall

```bash
ufw allow 7777/udp
```

## Data

Identity and database are stored in `./neonetconf/` next to the `docker-compose.yml`.
Back up this directory to preserve your node identity.

| Path | Content |
|------|---------|
| `./neonetconf/.neonet/keystore/` | Ed25519 identity (encrypted with passphrase) |
| `./neonetconf/.neonet/neonet.db` | TOFU peer database |

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `NEONET_PASSPHRASE` | Yes | Keystore passphrase (also used for auto-init) |
| `NEONET_LOG` | No | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `NEONET_LISTEN_PORT` | No | QUIC listen port (default: 7777) |

## Useful Commands

```bash
# Logs
docker compose logs -f

# Show node identity
docker exec neonet-rendezvous /neonet identity show

# Update to latest version
docker compose pull && docker compose up -d

# Stop
docker compose down
```
