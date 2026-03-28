# Deploying a NeoNet Relay Node

A relay node federates with other relays and routes encrypted messages
for clients behind NAT. It registers on a rendezvous server so other
relays can discover it by domain name.

## Quick Start

### 1. Configure

```bash
cp .env.example .env
nano .env
# NEONET_PASSPHRASE  = strong passphrase for the keystore
# NEONET_DOMAIN      = your public domain (e.g. neonet.example.com)
# NEONET_RENDEZVOUS  = rendezvous server address (e.g. 1.2.3.4:7777)
```

### 2. DNS

Point your domain to this server's public IP:
```
neonet.example.com.  A  <YOUR_SERVER_IP>
```

### 3. Start

```bash
docker compose up -d
```

That's it. On first start, if no keystore is found in `./neonetconf/`,
the container automatically initialises a new Ed25519 identity using
`NEONET_PASSPHRASE` and `NEONET_DOMAIN` from the `.env` file.

### 4. Verify

```bash
docker compose logs -f
# → Auto-init : keystore créé
# → Enregistré sur <rendezvous> comme <domain>
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
| `NEONET_DOMAIN` | Yes | Public domain for this relay |
| `NEONET_RENDEZVOUS` | Yes | Rendezvous server `host:port` (comma-separated for multiple) |
| `NEONET_LOG` | No | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `NEONET_LISTEN_PORT` | No | QUIC listen port (default: 7777) |

## Useful Commands

```bash
# Logs
docker compose logs -f

# Show node identity
docker exec neonet-relay /neonet identity show

# List connected peers
docker exec neonet-relay /neonet peers list

# Update to latest version
docker compose pull && docker compose up -d

# Stop
docker compose down
```
