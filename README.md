# NeoNet

**Protocole de communication decentralise, privacy-first, ecrit en Rust.**

[![Build](https://img.shields.io/github/actions/workflow/status/NeoNet-app/NeoNet/release.yml?label=build)](https://github.com/NeoNet-app/NeoNet/actions)
[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/NeoNet-app/NeoNet/releases)
[![License](https://img.shields.io/badge/license-AGPL--3.0-green)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux-lightgrey)]()

---

## C'est quoi NeoNet ?

NeoNet est un protocole de communication decentralise qui remplace Matrix.org
avec une stack plus legere, plus sure, et plus respectueuse des metadonnees.

| Probleme Matrix | Solution NeoNet |
|---|---|
| Metadonnees trop exposees | Payload toujours chiffre, padding uniforme, sealed sender |
| Dependance aux homeservers | Identite basee sur cle Ed25519, pas sur un serveur |
| Synchro lente et peu fiable | CRDT pour l'etat, DAG leger pour les messages |
| Trop complexe et lourd | Protocole minimaliste, types binaires, pas de JSON-LD |

L'identite est une cle cryptographique. Le domaine n'est qu'un hint de
decouverte. Si votre serveur disparait, votre identite reste valide.

---

## Architecture

### Vue globale du reseau

```
              rdv.example.com
                (rendezvous)
                     |
         +-----------+-----------+
         |                       |
  neonet.alice.com         neonet.bob.com
      (relay)                  (relay)
         |                       |
   Client Alice            Client Bob
    (laptop)                (laptop)
```

Les **rendezvous** sont des annuaires ephemeres : ils ne stockent rien et ne
voient que l'IP de bootstrap. Les **relais** federent entre eux et routent les
messages chiffres. Les **clients** se connectent a leur relais de confiance.

### Stack protocolaire

```
+-----------------------------------+
|  Applications (chat, social...)   |  <- Couche 5
+-----------------------------------+
|  Sync & etat distribue (CRDT)     |  <- Couche 4
+-----------------------------------+
|  Identite & cryptographie         |  <- Couche 3
+-----------------------------------+
|  Routage hybride (Fed + P2P)      |  <- Couche 2
+-----------------------------------+
|  Transport (QUIC / TLS 1.3)       |  <- Couche 1
+-----------------------------------+
```

### Flow d'un message Alice -> Bob

```
Client Alice
     |
     |  REST API (localhost)
     v
Relay Alice (neonet.alice.com)
     |
     |  1. Lookup "bob.local" sur le rendezvous
     |  2. Connexion QUIC + handshake NeoNet
     |  3. Frame chiffree ChaCha20-Poly1305
     v
Relay Bob (neonet.bob.com)
     |
     |  Push WebSocket
     v
Client Bob
```

Les relais ne voient que des blobs chiffres opaques. Seuls Alice et Bob
peuvent dechiffrer le contenu.

---

## Prerequis

- **Rust 1.75+** (pour compiler depuis les sources)
- **Un VPS avec port UDP 7777 ouvert** (pour relay ou rendezvous)
- macOS ARM, Linux x86_64 ou Linux ARM64

---

## Installation

### Depuis les releases (recommande)

https://github.com/NeoNet-app/NeoNet/releases

```bash
chmod +x neonet && sudo mv neonet /usr/local/bin/
```

### Depuis les sources

```bash
git clone https://github.com/NeoNet-app/NeoNet
cd NeoNet
cargo build --release
sudo cp target/release/neonet /usr/local/bin/
```

### Via Docker (recommande pour les VPS)

```bash
# Rendezvous
docker pull ghcr.io/neonet-app/neonet-rendezvous:latest

# Relay
docker pull ghcr.io/neonet-app/neonet-relay:latest
```

Voir [`docker/rendezvous/README.md`](docker/rendezvous/README.md) et
[`docker/relay/README.md`](docker/relay/README.md) pour le deploiement complet.

---

## Setup complet en 3 etapes

> **Prerequis VPS** : le port UDP 7777 doit etre ouvert.
> - UFW (Ubuntu) : `ufw allow 7777/udp`
> - iptables : `iptables -A INPUT -p udp --dport 7777 -j ACCEPT`
> - OVH / Hetzner / DigitalOcean : ouvrir le port dans le panel reseau

### Option A -- Installation binaire (laptop/desktop)

#### Etape 1 -- Monter le rendezvous (VPS 1)

Le rendezvous est un annuaire leger. Il ne stocke ni messages ni identites.

```bash
neonet init --domain rdv.example.com
neonet start --mode rendezvous --listen-port 7777 --daemon
```

Sortie attendue :
```
En ecoute (rendezvous) sur 0.0.0.0:7777
Daemon pret.
```

#### Etape 2 -- Monter son relais (VPS personnel)

Le relais federe avec d'autres relais et route les messages pour vos clients.

```bash
neonet init --domain neonet.alice.com
neonet start --mode relay \
             --domain neonet.alice.com \
             --rendezvous <IP_VPS1>:7777 \
             --listen-port 7777 \
             --daemon
```

Sortie attendue :
```
Enregistre sur <IP_VPS1>:7777 comme neonet.alice.com
Daemon pret.
```

#### Etape 3 -- Demarrer son client (laptop)

```bash
neonet init
neonet start --mode client \
             --relay neonet.alice.com:7777 \
             --daemon
```

### Option B -- Docker (VPS, recommande)

#### Etape 1 -- Monter le rendezvous (VPS 1)

```bash
git clone https://github.com/NeoNet-app/NeoNet
cd NeoNet/docker/rendezvous

# Configurer
cp .env.example .env
nano .env
# -> NEONET_PASSPHRASE=une_passphrase_solide

# Initialiser le keystore (une seule fois)
docker run --rm -it \
  -v neonet-rdv-data:/data \
  -e NEONET_PASSPHRASE="$NEONET_PASSPHRASE" \
  ghcr.io/neonet-app/neonet-rendezvous:latest \
  init

# Demarrer
docker compose up -d

# Verifier
docker compose logs -f
# -> En ecoute (rendezvous) sur 0.0.0.0:7777
```

Ouvrir le port :
```bash
ufw allow 7777/udp
```

#### Etape 2 -- Monter son relais (VPS personnel)

```bash
cd NeoNet/docker/relay

cp .env.example .env
nano .env
# -> NEONET_PASSPHRASE=une_passphrase_solide
# -> NEONET_DOMAIN=neonet.alice.com
# -> NEONET_RENDEZVOUS=<IP_VPS1>:7777

# Initialiser le keystore (une seule fois)
docker run --rm -it \
  -v neonet-relay-data:/data \
  -e NEONET_PASSPHRASE="$NEONET_PASSPHRASE" \
  ghcr.io/neonet-app/neonet-relay:latest \
  init --domain neonet.alice.com

# Demarrer
docker compose up -d

# Verifier
docker compose logs -f
# -> Enregistre sur <IP_VPS1>:7777 comme neonet.alice.com
```

Ouvrir le port :
```bash
ufw allow 7777/udp
```

#### Etape 3 -- Demarrer son client (laptop)

Le client tourne en binaire natif (pas de Docker necessaire) :

```bash
# Installer le binaire (voir section Installation)
neonet init
neonet start --mode client \
             --relay neonet.alice.com:7777 \
             --daemon
```

### Schema de deploiement complet

```
VPS 1                     VPS 2 (Alice)            VPS 3 (Bob)
+-------------------+     +------------------+     +------------------+
|    Rendezvous     |     |   Relay Alice    |     |   Relay Bob      |
|  docker compose   |<--->|  docker compose  |     |  docker compose  |
|  port 7777/udp    |     |  port 7777/udp   |<--->|  port 7777/udp   |
+-------------------+     +------------------+     +------------------+
                                   ^                        ^
                                   |                        |
                            Laptop Alice             Laptop Bob
                            neonet client            neonet client
                            --mode client            --mode client
                            --relay alice            --relay bob
```

---

## Utilisation

### Verifier le statut

```bash
neonet status
```

```
Daemon en cours
Identite  : ed25519:ABC123...
API       : http://127.0.0.1:7780
```

### Creer une room

```bash
neonet rooms create \
  --name "Discussion" \
  --members @pubkey:neonet.bob.com
```

### Voir ses rooms

```bash
neonet rooms list
```

### Trouver un noeud sur le reseau

```bash
neonet peers find neonet.bob.com \
  --rendezvous <IP_VPS1>:7777
```

```
neonet.bob.com -> 5.6.7.8:7777 (pubkey: ed25519:DEF456...)
```

### Voir les pairs connectes

```bash
neonet peers list
```

---

## API REST locale

Le daemon expose une API REST sur `127.0.0.1:7780` pour les applications
clientes. Le daemon dechiffre tous les payloads -- l'app recoit du JSON clair.

```bash
# Identite
curl -H "Authorization: Bearer $(cat ~/.neonet/session.token)" \
     http://127.0.0.1:7780/v1/identity

# Envoyer un message
curl -X POST \
     -H "Authorization: Bearer $(cat ~/.neonet/session.token)" \
     -H "Content-Type: application/json" \
     -d '{"text":"Hello NeoNet!"}' \
     http://127.0.0.1:7780/v1/rooms/<room_id>/messages
```

WebSocket temps reel sur `ws://127.0.0.1:7780/v1/ws` pour les notifications
`new_message`, `presence_update`, `member_joined`, etc.

Documentation complete : [`openapi.yaml`](openapi.yaml)

---

## Identite NeoNet

Le format d'une adresse NeoNet est `@<pubkey>:<domain>` :

```bash
neonet identity show
# Cle pub: ed25519:rwLKKjurz9FhQqUkw71mdc0JdLXx_aJ6KLQ3vn3Ikpc
# Adresse: @rwLKKjurz9FhQqUkw71mdc0JdLXx_aJ6KLQ3vn3Ikpc:neonet.alice.com
```

- La cle publique est **Ed25519** (permanente, portable)
- Le domaine est un **hint de decouverte** (pas autoritatif)
- Si le domaine disparait, l'identite reste valide
- Mode anonyme possible (`author` optionnel dans les events)

Pour partager votre adresse : copiez la sortie de `neonet identity export`.

---

## Securite

| Couche | Technologie | Description |
|---|---|---|
| Identite | Ed25519 | Cles de signature portables |
| Echange de cles | X25519 | Cles ephemeres par session |
| Chiffrement | ChaCha20-Poly1305 | E2EE sur tous les payloads |
| Derivation | HKDF-SHA256 | Forward secrecy garantie |
| Hachage | Blake3 | IDs deterministes des events |
| Keystore | Argon2id + ChaCha20 | Cle privee chiffree sur disque |
| Transport | QUIC + TLS 1.3 | Multiplexing, 0-RTT |
| Anti-trafic | Padding 256 bytes | Resist a l'analyse de trafic |

**Forward secrecy** : chaque session genere une paire X25519 ephemere.
Compromettre une cle long terme ne dechiffre pas les sessions passees.

**Metadonnees minimisees** : les relais ne voient que des blobs chiffres
opaques. Les timestamps sont non-autoritatifs. Le mode sealed sender masque
l'expediteur.

---

## Structure du projet

```
NeoNet/
+-- crates/
|   +-- neonet-core/       Types de base (Event, Room, Identity, DHT)
|   +-- neonet-crypto/     Ed25519, X25519, Blake3, ChaCha20, AEAD
|   +-- neonet-proto/      Handshake, frames, serialisation, rendezvous
|   +-- neonet-node/       Binaire: CLI, API REST, QUIC, keystore
+-- docker/
|   +-- rendezvous/         Dockerfile + compose pour rendezvous
|   +-- relay/              Dockerfile + compose pour relay
+-- docs/                   Specifications du protocole
+-- openapi.yaml            Documentation API (OpenAPI 3.1)
+-- Cargo.toml              Workspace root
```

---

## Variables d'environnement

| Variable | Description |
|---|---|
| `NEONET_PASSPHRASE` | Passphrase du keystore |
| `NEONET_DOMAIN` | Domaine public du noeud (relay) |
| `NEONET_RENDEZVOUS` | Adresses rendezvous (comma-separated) |
| `NEONET_LISTEN_PORT` | Port QUIC (defaut 7777) |
| `NEONET_LOG` | Niveau de log (`error`, `warn`, `info`, `debug`, `trace`) |
| `NEONET_API_URL` | URL de l'API locale (defaut `http://127.0.0.1:7780`) |
| `NEONET_DATA_DIR` | Repertoire de donnees (defaut `~/.neonet/`) |

Priorite : CLI > env > config.toml

---

## Variables d'environnement Docker

Ces variables sont reconnues par les images Docker (`neonet-rendezvous` et `neonet-relay`) :

| Variable | Description | Defaut |
|---|---|---|
| `NEONET_PASSPHRASE` | Passphrase du keystore | -- (obligatoire) |
| `NEONET_DOMAIN` | Domaine public du relais | -- (relay uniquement) |
| `NEONET_RENDEZVOUS` | Adresse(s) rendezvous (virgule separees) | -- |
| `NEONET_LISTEN_PORT` | Port QUIC | `7777` |
| `NEONET_LOG` | Niveau de log | `info` |

---

## Commandes Docker utiles

```bash
# Voir les logs en temps reel
docker compose logs -f

# Redemarrer apres mise a jour
docker compose pull && docker compose up -d

# Voir l'identite du noeud
docker exec neonet-relay /neonet identity show
docker exec neonet-rendezvous /neonet identity show

# Voir les pairs connectes (relay)
docker exec neonet-relay /neonet peers list

# Arreter proprement
docker compose down

# Supprimer les donnees (attention -- supprime le keystore !)
docker compose down -v
```

---

## Mise a jour

```bash
# Mettre a jour vers la derniere version
docker compose pull
docker compose up -d

# Verifier la version
docker exec neonet-relay /neonet --version
```

Pour les installations binaires :

```bash
# Telecharger la derniere release
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-x86_64-unknown-linux-musl -o neonet
chmod +x neonet && sudo mv neonet /usr/local/bin/

# Redemarrer le daemon
neonet stop && neonet start --daemon
```

---

## Roadmap

- [x] Handshake cryptographique (X25519 + HKDF + ChaCha20)
- [x] API REST + WebSocket locale (17 endpoints)
- [x] Keystore chiffre Argon2id
- [x] CLI complete (10 commandes)
- [x] Modes relay / rendezvous / client
- [x] Bootstrap rendezvous + TOFU
- [x] Frames chiffrees post-handshake
- [x] Docker (scratch images) + CI/CD
- [ ] DHT Kademlia complet (lookup, store, refresh)
- [ ] NAT traversal via relais
- [ ] Rotation de cles de room
- [ ] Support Tor (transport optionnel)
- [ ] Sync DAG inter-relais
- [ ] Applications tierces (SDK)

---

## Licence

[AGPL-3.0-or-later](LICENSE)
