# NeoNet — CLI

---

## 1. Vue d'ensemble

Le binaire `neonet` est le point d'entrée unique pour gérer le daemon, l'identité, et debugger le réseau.

```
neonet <commande> [options]
```

---

## 2. Commandes

### `neonet init`

Crée le keystore et la configuration initiale. Lancé automatiquement si aucun keystore n'existe.

```bash
neonet init
neonet init --domain mondomaine.com     # associe l'identité à un domaine
neonet init --passphrase-env            # lit NEONET_PASSPHRASE depuis l'env
```

**Séquence interactive :**
```
NeoNet — Initialisation
───────────────────────
Domaine (optionnel, ex: mondomaine.com) : mondomaine.com
Passphrase : ********
Confirmer la passphrase : ********

Génération de la paire de clés Ed25519...
Keystore créé : ~/.neonet/keystore/identity.key
Clé publique  : ed25519:BASE64URL
Adresse NeoNet: @ed25519BASE64URL:mondomaine.com

Configuration créée : ~/.neonet/config.toml
Base de données    : ~/.neonet/neonet.db

Démarrez le daemon avec : neonet start
```

---

### `neonet start`

Démarre le daemon en foreground.

```bash
neonet start
neonet start --daemon                   # fork en background
neonet start --api-port 7780
neonet start --listen-port 7777
neonet start --passphrase-env           # passphrase via NEONET_PASSPHRASE
neonet start --log-level debug
neonet start --config /chemin/config.toml
```

**Sortie au démarrage :**
```
NeoNet daemon v0.1.0
────────────────────
Identité    : @ed25519BASE64URL:mondomaine.com
API locale  : http://127.0.0.1:7780
Réseau      : 0.0.0.0:7777 (QUIC)
Token API   : écrit dans ~/.neonet/session.token

Bootstrap en cours...
  Rendezvous exemple.com:7777 ... OK (12 pairs reçus)
  DHT bootstrap ... OK (34 nœuds connus)

Daemon prêt.
```

---

### `neonet stop`

Arrête le daemon proprement.

```bash
neonet stop
```

---

### `neonet status`

Affiche l'état du daemon en cours d'exécution.

```bash
neonet status
```

**Sortie :**
```
Daemon        : en cours (PID 12345)
Identité      : @ed25519BASE64URL:mondomaine.com
API           : http://127.0.0.1:7780
Uptime        : 3h 24min
Pairs connectés: 42
Nœuds DHT    : 312
Rooms actives : 3
```

---

### `neonet identity`

Gestion de l'identité locale.

```bash
neonet identity show                    # affiche clé publique et adresse
neonet identity export                  # exporte la clé publique (PEM ou BASE64)
neonet identity sign <payload_hex>      # signe un payload (debug)
```

**Sortie de `neonet identity show` :**
```
Adresse   : @ed25519BASE64URL:mondomaine.com
Clé pub   : ed25519:BASE64URL
Créée le  : 2025-01-01 00:00:00 UTC
Domaine   : mondomaine.com
```

---

### `neonet peers`

Inspection des pairs et de la DHT.

```bash
neonet peers list                       # liste les pairs connectés
neonet peers list --json                # sortie JSON
neonet peers ping <addr>                # ping un nœud NeoNet
neonet peers find <node_id_hex>         # lookup DHT
neonet peers rendezvous                 # état des rendezvous
```

**Sortie de `neonet peers list` :**
```
NODE ID          ADRESSE                  TYPE    RTT     VU IL Y A
a1b2c3d4...      1.2.3.4:7777             full    24ms    2s
e5f6a7b8...      5.6.7.8:7777             relay   88ms    15s
...
42 pairs connectés, 312 nœuds DHT connus
```

---

### `neonet rooms`

Gestion des rooms depuis la CLI.

```bash
neonet rooms list                       # liste les rooms
neonet rooms create --name "Mon groupe" --members @pubkey1:d1.com,@pubkey2:d2.com
neonet rooms join <neonet://join/...>   # rejoindre via lien d'invitation
neonet rooms info <room_id>             # détails d'une room
neonet rooms leave <room_id>
```

---

### `neonet rendezvous`

Gestion des listes de rendezvous.

```bash
neonet rendezvous list                  # listes configurées + statut
neonet rendezvous add <url>             # ajouter une liste
neonet rendezvous remove <url>          # retirer une liste
neonet rendezvous verify <url>          # vérifier signature d'une liste
neonet rendezvous serve                 # démarrer le serveur web /.neonet/
```

**`neonet rendezvous serve` :**

Démarre un serveur Actix-web qui expose `/.neonet/rendezvous.toml` pour permettre à d'autres de bootstrapper depuis ce nœud. Port configurable, HTTPS via reverse proxy recommandé.

```bash
neonet rendezvous serve --port 8080
neonet rendezvous serve --name "Ma communauté" --port 8080
```

---

### `neonet logs`

Affiche les logs du daemon.

```bash
neonet logs                             # suit les logs en temps réel (tail -f)
neonet logs --level warn                # filtre par niveau
neonet logs --since 1h                 # depuis 1 heure
neonet logs --no-follow                 # affiche et quitte
```

---

### `neonet config`

Gestion de la configuration.

```bash
neonet config show                      # affiche config actuelle
neonet config set api.port 7781         # modifie une valeur
neonet config get network.listen_port   # lit une valeur
```

---

## 3. Options globales

Disponibles sur toutes les commandes :

| Option | Description |
|---|---|
| `--config <path>` | Chemin vers config.toml (défaut: `~/.neonet/config.toml`) |
| `--data-dir <path>` | Répertoire de données (défaut: `~/.neonet/`) |
| `--api-url <url>` | URL de l'API locale (défaut: `http://127.0.0.1:7780`) |
| `--json` | Sortie JSON sur la plupart des commandes |
| `--quiet` | Supprime la sortie non-essentielle |
| `--help` | Aide |
| `--version` | Version |

---

## 4. Codes de sortie

| Code | Signification |
|---|---|
| 0 | Succès |
| 1 | Erreur générale |
| 2 | Daemon non démarré (pour les commandes qui en dépendent) |
| 3 | Keystore non initialisé |
| 4 | Passphrase incorrecte |
| 5 | Erreur réseau |

---

## 5. Variables d'environnement

| Variable | Description |
|---|---|
| `NEONET_PASSPHRASE` | Passphrase du keystore |
| `NEONET_DATA_DIR` | Répertoire de données (override `--data-dir`) |
| `NEONET_API_URL` | URL API locale (override `--api-url`) |
| `NEONET_LOG` | Niveau de log (`error`, `warn`, `info`, `debug`, `trace`) |

---

## 6. Exemples d'usage complets

### Premier démarrage

```bash
# Initialiser l'identité
neonet init --domain mondomaine.com

# Démarrer le daemon
neonet start

# Vérifier le statut
neonet status

# Créer une room avec un ami
neonet rooms create --name "Discussion" --members @pubkey:son-domaine.com
```

### Monter un nœud rendezvous communautaire

```bash
# Démarrer le daemon
NEONET_PASSPHRASE=secret neonet start --daemon

# Exposer le rendezvous public
neonet rendezvous serve --name "Ma communauté" --port 8080

# Les autres peuvent maintenant bootstrapper avec :
# https://mondomaine.com/.neonet/rendezvous.toml
```

### Debug réseau

```bash
# Voir les pairs
neonet peers list

# Pinger un nœud spécifique
neonet peers ping rendezvous1.exemple.com:7777

# Lookup DHT
neonet peers find a1b2c3d4e5f6...

# Voir les logs en temps réel
neonet logs --level debug
```

---

## 7. Crates Rust

| Crate | Usage |
|---|---|
| `clap` | Parsing des arguments CLI (derive feature) |
| `rpassword` | Saisie passphrase sans écho |
| `indicatif` | Progress bars (bootstrap, sync) |
| `colored` | Couleurs terminal |
| `comfy-table` | Tableaux formatés (peers list, rooms list) |
| `reqwest` | Appels à l'API locale depuis la CLI |
| `tokio` | Runtime async |

---

## Récapitulatif des commandes

| Commande | Description |
|---|---|
| `neonet init` | Créer l'identité et la config |
| `neonet start` | Démarrer le daemon |
| `neonet stop` | Arrêter le daemon |
| `neonet status` | État du daemon |
| `neonet identity` | Gestion de l'identité |
| `neonet peers` | Inspection DHT et pairs |
| `neonet rooms` | Gestion des rooms |
| `neonet rendezvous` | Gestion des listes rendezvous + serving |
| `neonet logs` | Logs du daemon |
| `neonet config` | Gestion de la configuration |