# NeoNet

**Protocole de communication décentralisé, privacy-first, écrit en Rust.**

[![Build](https://img.shields.io/github/actions/workflow/status/NeoNet-app/NeoNet/release.yml?label=build)](https://github.com/NeoNet-app/NeoNet/actions)
[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/NeoNet-app/NeoNet/releases)
[![License](https://img.shields.io/badge/license-AGPL--3.0-green)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux-lightgrey)]()

---

## C'est quoi NeoNet ?

NeoNet est un protocole de messagerie décentralisé qui remplace Matrix.org
avec une stack plus légère, plus sûre et plus respectueuse des métadonnées.

Ton identité est une clé cryptographique Ed25519 — pas un compte sur un serveur.
Si ton relay disparaît, ton identité reste valide. Si le rendezvous tombe,
le réseau continue à fonctionner.

| Problème Matrix | Solution NeoNet |
|---|---|
| Métadonnées trop exposées | Payload toujours chiffré, padding uniforme, sealed sender |
| Dépendance aux homeservers | Identité = clé Ed25519, pas un compte serveur |
| Synchro lente et peu fiable | CRDT pour l'état, DAG léger pour les messages |
| Trop complexe et lourd | Protocole minimaliste, frames binaires, pas de JSON-LD |

---

## Les 3 composants

NeoNet est composé de **3 pièces distinctes** qui tournent sur des machines différentes.

### 1. `neonet` — le daemon (ton laptop)

Le cœur du protocole. Tourne en **arrière-plan** sur ta machine.

- Gère ta clé privée (keystore chiffré Argon2id)
- Parle au réseau NeoNet via QUIC chiffré (ChaCha20-Poly1305)
- Expose une API REST locale sur `127.0.0.1:7780`
- Modes : `client` | `relay` | `rendezvous` | `full`

> **Le daemon doit tourner** pour que n'importe quelle app NeoNet fonctionne.
> C'est lui le pont entre tes apps et le réseau.

### 2. `neonet-demo-chat` — l'app de chat TUI (ton laptop)

Une interface de chat en terminal. Tourne en **foreground** dans un terminal.

- Parle **uniquement** à l'API locale du daemon (`127.0.0.1:7780`)
- Ne fait aucun réseau NeoNet directement
- Nécessite que le daemon soit démarré

### 3. relay sur VPS — ton nœud public

Un daemon `neonet` en mode `relay` sur un VPS, déployé via Docker.

- Port 7777 UDP ouvert publiquement
- Ton daemon client s'y connecte au démarrage
- Route les messages chiffrés vers les autres relays
- Un relay par personne (tu es souverain sur ta pile)

---

## Architecture

### Ce qui tourne où

```
Ton laptop                    Ton VPS relay             VPS rendezvous
┌──────────────────────┐    ┌─────────────────┐    ┌──────────────────┐
│ neonet-demo-chat     │    │  neonet-node    │    │  neonet-node     │
│ (TUI, foreground)    │    │  --mode relay   │    │  --mode          │
│         │            │    │  (docker)       │    │  rendezvous      │
│         │ localhost  │    │  port 7777/udp  │    │  (docker)        │
│         ▼ :7780      │    │                 │    │  port 7777/udp   │
│  neonet-node         │    │                 │    │                  │
│  --mode client       │───►│                 │◄──►│                  │
│  (daemon, background)│    │                 │    │                  │
└──────────────────────┘    └─────────────────┘    └──────────────────┘
```

### Flow d'un message Alice → Bob

```
[Alice tape dans neonet-demo-chat]
         │
         │  POST /v1/rooms/{id}/messages  (HTTP localhost)
         ▼
[daemon Alice — mode client]
         │
         │  QUIC + ChaCha20-Poly1305  (E2EE)
         ▼
[relay Alice — neonet.alice.com:7777]
         │
         │  lookup Bob sur le rendezvous
         │  QUIC chiffré
         ▼
[relay Bob — neonet.bob.com:7777]
         │
         │  QUIC chiffré
         ▼
[daemon Bob — mode client]
         │
         │  WebSocket push  (localhost)
         ▼
[neonet-demo-chat de Bob affiche le message]
```

Les relays ne voient que des blobs chiffrés opaques.
Seuls Alice et Bob peuvent déchiffrer le contenu.

### Stack protocolaire

```
┌─────────────────────────────────────┐
│  Applications  (chat, social, ...)  │  ← Couche 5
├─────────────────────────────────────┤
│  Sync & état distribué  (CRDT/DAG)  │  ← Couche 4
├─────────────────────────────────────┤
│  Identité & cryptographie           │  ← Couche 3
│  Ed25519 · X25519 · ChaCha20 · HKDF │
├─────────────────────────────────────┤
│  Routage hybride  (fédéré + P2P)    │  ← Couche 2
├─────────────────────────────────────┤
│  Transport  QUIC / TLS 1.3          │  ← Couche 1
└─────────────────────────────────────┘
```

---

## Prérequis

| Composant | Où | Prérequis |
|---|---|---|
| Daemon client + demo-chat | Ton laptop | Binaire natif (pas de docker) |
| Relay | Ton VPS | Docker + port 7777/udp ouvert |
| Rendezvous | VPS commun | Docker + port 7777/udp ouvert |

Pour compiler depuis les sources : Rust 1.75+

---

## Installation du daemon (laptop)

Télécharge le binaire depuis les [releases GitHub](https://github.com/NeoNet-app/NeoNet/releases).

### macOS ARM (Apple Silicon)

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-aarch64-apple-darwin \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

### Linux x86_64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-x86_64-unknown-linux-musl \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

### Linux ARM64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-aarch64-unknown-linux-musl \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

### Depuis les sources

```bash
git clone https://github.com/NeoNet-app/NeoNet
cd NeoNet
cargo build --release
sudo cp target/release/neonet /usr/local/bin/
```

---

## Installation neonet-demo-chat (laptop)

### macOS ARM

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-demo-chat-aarch64-apple-darwin \
  -o neonet-demo-chat
chmod +x neonet-demo-chat
sudo mv neonet-demo-chat /usr/local/bin/
```

### Linux x86_64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-demo-chat-x86_64-unknown-linux-musl \
  -o neonet-demo-chat
chmod +x neonet-demo-chat
sudo mv neonet-demo-chat /usr/local/bin/
```

### Linux ARM64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-demo-chat-aarch64-unknown-linux-musl \
  -o neonet-demo-chat
chmod +x neonet-demo-chat
sudo mv neonet-demo-chat /usr/local/bin/
```

---

## Setup VPS — Rendezvous

> À faire une seule fois, de préférence en commun entre tous les participants.

```bash
# Sur le VPS rendezvous
git clone https://github.com/NeoNet-app/NeoNet
cd NeoNet/docker/rendezvous

cp .env.example .env
nano .env
# → NEONET_PASSPHRASE=une_passphrase_solide

# Ouvrir le port
ufw allow 7777/udp

# Initialiser le keystore (une seule fois — génère l'identité Ed25519)
docker run --rm -it \
  -v neonet-rdv-data:/data \
  --env-file .env \
  ghcr.io/neonet-app/neonet-rendezvous:latest \
  init

# Démarrer
docker compose up -d

# Vérifier
docker compose logs -f
# → QUIC listener started on 0.0.0.0:7777
# → En écoute (rendezvous) sur 0.0.0.0:7777
# → Daemon prêt.
```

**Note l'IP de ce VPS** — tu en auras besoin pour configurer tes relays.

---

## Setup VPS — Relay

> Chaque personne fait le sien sur son propre VPS.

```bash
# Sur ton VPS relay
git clone https://github.com/NeoNet-app/NeoNet
cd NeoNet/docker/relay

cp .env.example .env
nano .env
# → NEONET_PASSPHRASE=une_passphrase_solide
# → NEONET_DOMAIN=neonet.alice.com
# → NEONET_RENDEZVOUS=<IP_VPS_RENDEZVOUS>:7777

# DNS : pointer ton domaine sur l'IP du VPS
# neonet.alice.com.  A  <IP_VPS_RELAY>

# Ouvrir le port
ufw allow 7777/udp

# Initialiser le keystore (une seule fois)
docker run --rm -it \
  -v neonet-relay-data:/data \
  --env-file .env \
  ghcr.io/neonet-app/neonet-relay:latest \
  init

# Démarrer
docker compose up -d

# Vérifier
docker compose logs -f
# → Enregistré sur <IP_RDV>:7777 comme neonet.alice.com
# → Daemon prêt.
```

---

## Variables d'environnement Docker

| Variable | Requis | Description | Défaut |
|---|---|---|---|
| `NEONET_PASSPHRASE` | Oui | Passphrase du keystore | — |
| `NEONET_DOMAIN` | Relay uniquement | Domaine public du relay | — |
| `NEONET_RENDEZVOUS` | Relay uniquement | Adresse(s) rendezvous (séparées par `,`) | — |
| `NEONET_LISTEN_PORT` | Non | Port QUIC d'écoute | `7777` |
| `NEONET_LOG` | Non | Niveau de log (`error` `warn` `info` `debug` `trace`) | `info` |

Priorité : CLI > variable d'environnement > `config.toml`

---

## Commandes Docker utiles

```bash
# Logs en temps réel
docker compose logs -f

# Voir l'identité du nœud
docker exec neonet-relay /neonet identity show
docker exec neonet-rendezvous /neonet identity show

# Pairs connectés (relay)
docker exec neonet-relay /neonet peers list

# Mise à jour vers la dernière version
docker compose pull && docker compose up -d

# Arrêter proprement
docker compose down

# Supprimer les données (⚠️ supprime le keystore !)
docker compose down -v
```

---

## Sécurité

| Couche | Technologie | Description |
|---|---|---|
| Identité | Ed25519 | Clés de signature portables, hors serveur |
| Échange de clés | X25519 | Clés éphémères par session |
| Chiffrement | ChaCha20-Poly1305 | E2EE sur tous les payloads |
| Dérivation | HKDF-SHA256 | Forward secrecy garantie |
| Hachage | Blake3 | IDs déterministes des events |
| Keystore | Argon2id + ChaCha20 | Clé privée chiffrée sur disque |
| Transport | QUIC + TLS 1.3 | Multiplexing, 0-RTT, résistant aux coupures |
| Anti-trafic | Padding 256 octets | Résiste à l'analyse de trafic |

**Forward secrecy** : chaque session génère une paire X25519 éphémère.
Compromettre une clé long terme ne déchiffre pas les sessions passées.

**Métadonnées minimisées** : les relays ne voient que des blobs chiffrés opaques.
Les timestamps sont non-autoritatifs. Le mode sealed sender masque l'expéditeur.

---

## Structure du projet

```
NeoNet/
├── crates/
│   ├── neonet-core/       Types de base (Event, Room, Identity, DHT)
│   ├── neonet-crypto/     Ed25519, X25519, Blake3, ChaCha20, AEAD
│   ├── neonet-proto/      Handshake, frames, sérialisation, rendezvous
│   └── neonet-node/       Binaire : CLI, API REST, QUIC, keystore
├── docker/
│   ├── rendezvous/        Dockerfile + compose pour rendezvous
│   └── relay/             Dockerfile + compose pour relay
├── docs/                  Spécifications du protocole
├── SETUP.md               Guide utilisateur laptop (→ lire ça pour commencer)
├── openapi.yaml           Documentation API (OpenAPI 3.1)
└── Cargo.toml             Workspace root
```

---

## Roadmap

- [x] Handshake cryptographique (X25519 + HKDF + ChaCha20)
- [x] API REST + WebSocket locale (17 endpoints)
- [x] Keystore chiffré Argon2id
- [x] CLI complète (10 commandes)
- [x] Modes relay / rendezvous / client
- [x] Bootstrap rendezvous + TOFU
- [x] Frames chiffrées post-handshake
- [x] Docker (scratch images) + CI/CD
- [x] Daemon mode (`--daemon`, fork Unix)
- [ ] DHT Kademlia complet (lookup, store, refresh)
- [ ] NAT traversal via relais
- [ ] Rotation de clés de room
- [ ] Support Tor (transport optionnel)
- [ ] Sync DAG inter-relais
- [ ] Applications tierces (SDK)

---

## Licence

[AGPL-3.0-or-later](LICENSE)
