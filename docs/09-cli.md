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

Crée le keystore et la configuration initiale.

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
Adresse NeoNet: @BASE64URL:mondomaine.com

Configuration créée : ~/.neonet/config.toml

Démarrez le daemon avec : neonet start
```

**Auto-init (Docker / CI) :**

Si `NEONET_PASSPHRASE` est définie dans l'environnement et qu'aucun keystore n'existe,
`neonet start` initialise automatiquement le keystore au premier démarrage.
`NEONET_DOMAIN` est utilisé comme domaine si présent.
Aucune interaction utilisateur requise.

---

### `neonet start`

Démarre le daemon.

```bash
neonet start                            # foreground
neonet start --daemon                   # fork en background (Unix)
neonet start --mode client              # mode réseau : client | relay | rendezvous | full
neonet start --relay neonet.alice.com:7777
neonet start --rendezvous 1.2.3.4:7777
neonet start --domain neonet.alice.com
neonet start --api-port 7780
neonet start --listen-port 7777
neonet start --log-level debug
neonet start --config /chemin/config.toml
```

**Sortie au démarrage :**
```
NeoNet daemon v0.1.0
────────────────────
Mode        : client
Identité    : @BASE64URL:neonet.alice.com
API locale  : http://127.0.0.1:7780
Réseau      : 0.0.0.0:7777 (QUIC)
Token API   : écrit dans ~/.neonet/session.token
Relay configuré : neonet.alice.com:7777

Daemon démarré (PID 84521)          ← avec --daemon
Logs    : ~/.neonet/daemon.log      ← avec --daemon
```

**Notes importantes :**
- Avec `--daemon` : le process se fork en background, le terminal est immédiatement rendu.
  Le PID est écrit dans `~/.neonet/daemon.pid`.
  Stdout + stderr sont redirigés vers `~/.neonet/daemon.log`.
- Sans `--daemon` : le daemon tourne en foreground et bloque le terminal.
- La passphrase peut être passée via `NEONET_PASSPHRASE` pour éviter le prompt interactif.

---

### `neonet stop`

Arrête le daemon proprement (SIGTERM).

```bash
neonet stop
# → Daemon arrêté (PID 84521)
```

Lit `~/.neonet/daemon.pid`, envoie SIGTERM au process, puis supprime le PID file
et le session token. Si le process n'existe plus (crash, etc.), nettoie les fichiers
sans erreur.

---

### `neonet status`

Affiche l'état du daemon en cours d'exécution.

```bash
neonet status
neonet status --json
```

**Sortie (daemon actif) :**
```
Daemon en cours
Identité : @BASE64URL:neonet.alice.com
API      : http://127.0.0.1:7780
Logs     : ~/.neonet/daemon.log
```

**Codes d'erreur spécifiques :**
- Connexion refusée → `Daemon non joignable (connexion refusée) — Lancer : neonet start ...`
- Token invalide (401) → `Token périmé — relancer le daemon pour régénérer le token`
- Aucun token file → `Daemon not running`

---

### `neonet identity`

Gestion de l'identité locale.

```bash
neonet identity show                    # affiche clé publique et adresse
neonet identity export                  # exporte la clé publique (BASE64URL)
neonet identity sign <payload_hex>      # signe un payload (debug)
```

**Sortie de `neonet identity show` :**
```
Clé pub : ed25519:BASE64URL
```

---

### `neonet peers`

Inspection des pairs connectés.

```bash
neonet peers list                       # liste les pairs connectés
neonet peers list --json                # sortie JSON
neonet peers connect <addr>:7777        # forcer une connexion QUIC+handshake
neonet peers ping <addr>                # ping un nœud NeoNet (non implémenté)
neonet peers find <node_id>             # lookup DHT (non implémenté)
neonet peers rendezvous                 # état des rendezvous (non implémenté)
```

**`neonet peers connect` :**

Ouvre une connexion QUIC vers le pair cible, exécute le handshake NeoNet
(X25519 + HKDF + ChaCha20), et affiche le résultat TOFU.

```bash
neonet peers connect neonet.alice.com:7777
# → neonet.alice.com:7777 → 1.2.3.4:7777 (pubkey: ed25519:DEF456...)
```

---

### `neonet rooms`

Gestion des rooms depuis la CLI.

```bash
neonet rooms list
neonet rooms create --name "Mon groupe" --members @pubkey1:d1.com,@pubkey2:d2.com
neonet rooms join <neonet://join/...>
neonet rooms info <room_id>
neonet rooms leave <room_id>
```

> La plupart de ces sous-commandes ne sont pas encore implémentées (stub).
> Utiliser l'API REST directement pour l'instant.

---

### `neonet rendezvous`

Gestion des listes de rendezvous.

```bash
neonet rendezvous list
neonet rendezvous add <url>
neonet rendezvous remove <url>
neonet rendezvous verify <url>
neonet rendezvous serve --port 8080
```

> Non implémenté (stub).

---

### `neonet logs`

Affiche les logs du daemon. Les logs sont écrits dans `~/.neonet/daemon.log`
lorsque le daemon est démarré avec `--daemon`.

```bash
neonet logs                             # suit les logs en temps réel (défaut)
neonet logs --level warn                # filtre par niveau (best-effort)
neonet logs --level error
neonet logs --level debug
neonet logs --no-follow                 # affiche le contenu actuel et quitte
```

**Équivalent shell :**
```bash
tail -f ~/.neonet/daemon.log
```

> `--since` est accepté mais non implémenté.

---

### `neonet config`

Gestion de la configuration.

```bash
neonet config show                      # affiche la config actuelle (TOML)
neonet config set api.port 7781         # non implémenté
neonet config get network.listen_port   # non implémenté
```

---

## 3. Options globales

Disponibles sur toutes les commandes :

| Option | Description |
|---|---|
| `--config <path>` | Chemin vers config.toml (défaut: `~/.neonet/config.toml`) |
| `--api-url <url>` | URL de l'API locale (défaut: `http://127.0.0.1:7780`) |
| `--json` | Sortie JSON sur la plupart des commandes |
| `--help` | Aide |
| `--version` | Version |

---

## 4. Codes de sortie

| Code | Signification |
|---|---|
| 0 | Succès |
| 1 | Erreur générale |
| 2 | Daemon non joignable ou token périmé |
| 3 | Keystore non initialisé |
| 4 | Passphrase incorrecte |
| 5 | Erreur réseau |

---

## 5. Variables d'environnement

| Variable | Description |
|---|---|
| `NEONET_PASSPHRASE` | Passphrase du keystore — évite le prompt interactif. Également utilisée pour l'auto-init si aucun keystore n'existe. |
| `NEONET_DOMAIN` | Domaine public du nœud — utilisé lors de l'auto-init. |
| `NEONET_RENDEZVOUS` | Adresse(s) rendezvous (séparées par `,`) |
| `NEONET_LISTEN_PORT` | Port QUIC d'écoute (défaut: `7777`) |
| `NEONET_LOG` | Niveau de log (`error`, `warn`, `info`, `debug`, `trace`) |
| `NEONET_API_URL` | URL API locale (défaut: `http://127.0.0.1:7780`) |

Priorité : CLI > variable d'environnement > `config.toml`

---

## 6. Fichiers générés

| Fichier | Description |
|---|---|
| `~/.neonet/keystore/identity.key` | Clé privée Ed25519 chiffrée (Argon2id + ChaCha20) |
| `~/.neonet/config.toml` | Configuration du daemon |
| `~/.neonet/neonet.db` | Base de données SQLite (TOFU peers, rooms, messages) |
| `~/.neonet/session.token` | Token Bearer de la session courante (chmod 600) |
| `~/.neonet/daemon.pid` | PID du daemon en arrière-plan (créé par `--daemon`) |
| `~/.neonet/daemon.log` | Logs du daemon en arrière-plan (créé par `--daemon`) |

> **Docker** : avec `ENV HOME=/data`, tous ces fichiers sont dans `/data/.neonet/`.
> Monter `./neonetconf:/data` dans docker-compose suffit à tout persister.

---

## 7. Exemples d'usage complets

### Premier démarrage (laptop)

```bash
# 1. Initialiser l'identité
neonet init
# Domaine : neonet.alice.com  (ou laisser vide)
# Passphrase : xxxxxxxx

# 2. Démarrer le daemon en background
NEONET_PASSPHRASE="ta_passphrase" \
neonet start --mode client --relay neonet.alice.com:7777 --daemon
# → Daemon démarré (PID 84521)
# → Logs : ~/.neonet/daemon.log

# 3. Vérifier
neonet status
neonet logs

# 4. Connecter un pair
neonet peers connect neonet.bob.com:7777

# 5. Arrêter
neonet stop
```

### Démarrage Docker automatique (relay / rendezvous)

```bash
# Le .env suffit — plus besoin de docker exec init
cp .env.example .env
nano .env          # NEONET_PASSPHRASE + NEONET_DOMAIN + NEONET_RENDEZVOUS

docker compose up -d
docker compose logs -f
# → Auto-init : keystore créé
# → Adresse   : @BASE64URL:neonet.alice.com
# → Daemon prêt.
```

### Debug réseau

```bash
# Logs en temps réel avec filtre
neonet logs --level debug

# Tester la connexion à un pair
neonet peers connect neonet.alice.com:7777

# Appel API direct (debug)
TOKEN=$(cat ~/.neonet/session.token)
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:7780/v1/identity | jq
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:7780/v1/peers | jq
```

---

## 8. Crates Rust

| Crate | Usage |
|---|---|
| `clap` | Parsing des arguments CLI (derive feature) |
| `rpassword` | Saisie passphrase sans écho |
| `indicatif` | Progress bars |
| `colored` | Couleurs terminal |
| `comfy-table` | Tableaux formatés |
| `reqwest` | Appels à l'API locale depuis la CLI |
| `tokio` | Runtime async |
| `libc` | SIGTERM Unix pour `neonet stop` |

---

## Récapitulatif des commandes

| Commande | Statut | Description |
|---|---|---|
| `neonet init` | ✅ | Créer l'identité et la config |
| `neonet start` | ✅ | Démarrer le daemon (foreground ou `--daemon`) |
| `neonet stop` | ✅ | Arrêter le daemon (SIGTERM via PID file) |
| `neonet status` | ✅ | État du daemon |
| `neonet identity show/export` | ✅ | Gestion de l'identité |
| `neonet peers connect` | ✅ | Connexion QUIC + handshake |
| `neonet peers list` | ✅ | Pairs connectés |
| `neonet logs` | ✅ | Logs du daemon (tail `daemon.log`) |
| `neonet config show` | ✅ | Afficher la config |
| `neonet rooms *` | 🚧 | Stub — via API REST pour l'instant |
| `neonet peers ping/find` | 🚧 | Stub |
| `neonet rendezvous *` | 🚧 | Stub |
| `neonet config set/get` | 🚧 | Stub |
