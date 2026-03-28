# NeoNet — API Client

---

## 1. Vue d'ensemble

Le daemon `neonet-node` expose une API locale sur `127.0.0.1:7780` (port par défaut).

```
┌─────────────┐   REST + WS    ┌──────────────┐   QUIC   ┌──────────────┐
│  App        │◄──────────────►│  neonet-node │◄────────►│  Réseau      │
│  (cliente)  │   localhost    │  daemon      │          │  NeoNet      │
└─────────────┘                └──────────────┘          └──────────────┘
                                      │
                                      │ chiffré Argon2id
                                      ▼
                                 ~/.neonet/keystore
```

**Modèle hybride :**
- **REST** pour toutes les actions (envoyer, créer, modifier)
- **REST** pour le chargement initial de l'historique
- **WebSocket** pour les notifications temps réel

Le daemon déchiffre tous les payloads — l'app reçoit du JSON clair, sans jamais manipuler de clés cryptographiques.

---

## 2. Authentification

### Token de session

Au démarrage du daemon, un token aléatoire est généré et écrit dans :

```
~/.neonet/session.token
chmod 600
```

Toutes les requêtes doivent inclure ce token en header :

```
Authorization: Bearer <token>
```

Une requête sans token ou avec un token invalide reçoit :

```
HTTP 401 Unauthorized
{ "error": "unauthorized", "message": "Missing or invalid session token" }
```

### Rotation du token

Le token est régénéré à chaque démarrage du daemon. Les apps doivent relire le fichier au démarrage.

---

## 3. Format des réponses

### Succès

```json
HTTP 200 OK
Content-Type: application/json

{ ...données... }
```

### Erreurs

```json
HTTP <code>
Content-Type: application/json

{
  "error": "snake_case_code",
  "message": "Description lisible"
}
```

### Codes d'erreur standard

| HTTP | error | Description |
|---|---|---|
| 400 | `bad_request` | Paramètres manquants ou invalides |
| 401 | `unauthorized` | Token manquant ou invalide |
| 403 | `forbidden` | Action non autorisée (ACL) |
| 404 | `not_found` | Ressource introuvable |
| 409 | `conflict` | Conflit d'état (ex: membre déjà présent) |
| 500 | `internal_error` | Erreur interne du daemon |
| 503 | `not_ready` | Daemon pas encore bootstrappé |

---

## 4. Endpoints REST

### 4.1 Identité

#### `GET /v1/identity`

Retourne l'identité locale du daemon.

**Réponse :**
```json
{
  "pubkey": "ed25519:BASE64URL",
  "address": "@ed25519BASE64URL:mondomaine.com",
  "node_type": "full"
}
```

---

#### `POST /v1/identity/sign`

Signe un payload arbitraire avec la clé privée locale. La clé privée ne sort jamais.

**Body :**
```json
{
  "payload": "BASE64URL"
}
```

**Réponse :**
```json
{
  "sig": "ed25519:BASE64URL",
  "pubkey": "ed25519:BASE64URL"
}
```

---

### 4.2 Rooms

#### `GET /v1/rooms`

Liste toutes les rooms dont le nœud est membre.

**Réponse :**
```json
{
  "rooms": [
    {
      "room_id": "BASE64URL",
      "room_type": "group",
      "name": "Mon groupe",
      "description": "...",
      "member_count": 5,
      "unread_count": 3,
      "last_event_id": "BASE64URL",
      "last_event_ts": 1720000000
    }
  ]
}
```

---

#### `POST /v1/rooms`

Crée une nouvelle room avec ses membres initiaux.

**Body :**
```json
{
  "name": "Mon groupe",
  "description": "Optionnel",
  "room_type": "group",
  "members": [
    "@ed25519ABC:domain1.com",
    "@ed25519DEF:domain2.com"
  ]
}
```

`room_type` : `"direct"` | `"group"` | `"channel"` | `"thread"`

Pour `"direct"` : exactement 1 membre dans la liste (+ soi-même = 2).

**Réponse :**
```json
{
  "room_id": "BASE64URL",
  "room_type": "group",
  "name": "Mon groupe",
  "created_at": 1720000000
}
```

---

#### `GET /v1/rooms/{room_id}`

Retourne l'état complet d'une room.

**Réponse :**
```json
{
  "room_id": "BASE64URL",
  "room_type": "group",
  "name": "Mon groupe",
  "description": "...",
  "members": [
    {
      "address": "@ed25519ABC:domain.com",
      "role": "owner",
      "joined_at": 1720000000,
      "presence": "online"
    }
  ],
  "settings": {},
  "created_at": 1720000000
}
```

---

#### `PATCH /v1/rooms/{room_id}`

Modifie les métadonnées d'une room (Admin/Owner uniquement).

**Body :**
```json
{
  "name": "Nouveau nom",
  "description": "Nouvelle description"
}
```

**Réponse :** `HTTP 200 {}`

---

#### `DELETE /v1/rooms/{room_id}`

Quitte une room. Si Owner, supprime la room pour tous (Owner uniquement).

**Réponse :** `HTTP 200 {}`

---

### 4.3 Messages

#### `GET /v1/rooms/{room_id}/messages`

Retourne l'historique des messages. Le daemon déchiffre tous les payloads.

**Query params :**

| Param | Type | Défaut | Description |
|---|---|---|---|
| `since` | event_id | — | Events après cet ID (exclusif) |
| `before` | event_id | — | Events avant cet ID (exclusif) |
| `limit` | integer | 50 | Nombre max de messages (max 200) |

**Réponse :**
```json
{
  "messages": [
    {
      "event_id": "BASE64URL",
      "author": "@ed25519ABC:domain.com",
      "kind": "message",
      "content": {
        "text": "Salut !"
      },
      "parents": ["BASE64URL"],
      "ts_hint": 1720000000,
      "edited": false,
      "redacted": false
    }
  ],
  "has_more": false
}
```

`kind` : `"message"` | `"reaction"` | `"thread_reply"` | `"edit"` | `"redact"` | `"file"`

---

#### `POST /v1/rooms/{room_id}/messages`

Envoie un message dans une room.

**Body :**
```json
{
  "text": "Salut tout le monde",
  "reply_to": "BASE64URL"
}
```

`reply_to` est optionnel — référence un event_id parent pour les réponses.

**Réponse :**
```json
{
  "event_id": "BASE64URL",
  "ts_hint": 1720000000
}
```

---

#### `POST /v1/rooms/{room_id}/messages/{event_id}/react`

Ajoute une réaction à un message.

**Body :**
```json
{
  "emoji": "👍"
}
```

**Réponse :** `HTTP 200 { "event_id": "BASE64URL" }`

---

#### `PATCH /v1/rooms/{room_id}/messages/{event_id}`

Édite un message (auteur uniquement).

**Body :**
```json
{
  "text": "Texte corrigé"
}
```

**Réponse :** `HTTP 200 { "event_id": "BASE64URL" }`

---

#### `DELETE /v1/rooms/{room_id}/messages/{event_id}`

Supprime un message (auteur ou Admin/Owner).

**Réponse :** `HTTP 200 {}`

---

### 4.4 Membres

#### `POST /v1/rooms/{room_id}/members`

Invite un membre (Admin/Owner uniquement).

**Body :**
```json
{
  "address": "@ed25519ABC:domain.com",
  "role": "member"
}
```

`role` : `"admin"` | `"member"` | `"guest"`

**Réponse :** `HTTP 200 {}`

---

#### `PATCH /v1/rooms/{room_id}/members/{address}`

Change le rôle d'un membre (Admin/Owner uniquement).

**Body :**
```json
{
  "role": "admin"
}
```

**Réponse :** `HTTP 200 {}`

---

#### `DELETE /v1/rooms/{room_id}/members/{address}`

Exclut un membre (Admin/Owner uniquement).

**Réponse :** `HTTP 200 {}`

---

### 4.5 Fichiers

#### `POST /v1/rooms/{room_id}/files`

Upload un fichier. Le daemon le chiffre avec la `room_key` et le distribue.

**Body :** `multipart/form-data`
- `file` : le fichier
- `filename` : nom affiché

**Réponse :**
```json
{
  "event_id": "BASE64URL",
  "file_id": "BASE64URL",
  "filename": "photo.jpg",
  "size": 204800,
  "mime_type": "image/jpeg"
}
```

---

#### `GET /v1/rooms/{room_id}/files/{file_id}`

Télécharge et déchiffre un fichier.

**Réponse :** binaire avec `Content-Type` approprié.

---

### 4.6 Peers & réseau

#### `GET /v1/peers`

Retourne les pairs actuellement connectés et l'état de la DHT.

**Réponse :**
```json
{
  "status": "online",
  "bootstrapped": true,
  "peer_count": 42,
  "routing_table": {
    "bucket_count": 160,
    "known_nodes": 312
  },
  "peers": [
    {
      "node_id": "HEX",
      "addr": "1.2.3.4:7777",
      "pubkey": "ed25519:BASE64URL",
      "node_type": "full",
      "rtt_ms": 24,
      "last_seen": 1720000000
    }
  ]
}
```

---

#### `GET /v1/peers/rendezvous`

Liste les rendezvous configurés et leur statut.

**Réponse :**
```json
{
  "rendezvous": [
    {
      "addr": "rendezvous1.exemple.com:7777",
      "pubkey": "ed25519:BASE64URL",
      "status": "reachable",
      "last_seen": 1720000000,
      "source": "builtin"
    }
  ]
}
```

`source` : `"builtin"` | `"config"` | `"cli"`

---

## 5. WebSocket

### Connexion

```
WS /v1/ws
Header: Authorization: Bearer <token>
```

Un seul WebSocket global par app. L'app filtre les events côté client selon les rooms qui l'intéressent.

### Message de subscribe

Envoyé par l'app après connexion pour indiquer les rooms à suivre :

```json
{
  "type": "subscribe",
  "room_ids": ["BASE64URL", "BASE64URL"]
}
```

Peut être renvoyé à tout moment pour mettre à jour les abonnements.

### Events poussés par le daemon

Tous les events ont la structure :

```json
{
  "type": "<event_type>",
  "room_id": "BASE64URL",
  ...données spécifiques
}
```

#### `new_message`
```json
{
  "type": "new_message",
  "room_id": "BASE64URL",
  "event_id": "BASE64URL",
  "author": "@ed25519ABC:domain.com",
  "kind": "message",
  "content": { "text": "Salut !" },
  "ts_hint": 1720000000
}
```

#### `message_edited`
```json
{
  "type": "message_edited",
  "room_id": "BASE64URL",
  "event_id": "BASE64URL",
  "original_event_id": "BASE64URL",
  "content": { "text": "Texte corrigé" }
}
```

#### `message_deleted`
```json
{
  "type": "message_deleted",
  "room_id": "BASE64URL",
  "event_id": "BASE64URL",
  "original_event_id": "BASE64URL"
}
```

#### `reaction`
```json
{
  "type": "reaction",
  "room_id": "BASE64URL",
  "event_id": "BASE64URL",
  "target_event_id": "BASE64URL",
  "author": "@ed25519ABC:domain.com",
  "emoji": "👍"
}
```

#### `presence_update`
```json
{
  "type": "presence_update",
  "room_id": "BASE64URL",
  "address": "@ed25519ABC:domain.com",
  "status": "online",
  "last_active": 1720000000
}
```

#### `member_joined`
```json
{
  "type": "member_joined",
  "room_id": "BASE64URL",
  "address": "@ed25519ABC:domain.com",
  "role": "member"
}
```

#### `member_left`
```json
{
  "type": "member_left",
  "room_id": "BASE64URL",
  "address": "@ed25519ABC:domain.com"
}
```

#### `room_updated`
```json
{
  "type": "room_updated",
  "room_id": "BASE64URL",
  "changes": {
    "name": "Nouveau nom"
  }
}
```

#### `sync_status`
```json
{
  "type": "sync_status",
  "room_id": "BASE64URL",
  "status": "syncing",
  "progress": 0.72
}
```

`status` : `"syncing"` | `"synced"` | `"error"`

---

## 6. Configuration du daemon

Fichier : `~/.neonet/config.toml`

```toml
[daemon]
api_port     = 7780                        # port de l'API locale
api_host     = "127.0.0.1"                # toujours localhost
log_level    = "info"

[identity]
keystore     = "~/.neonet/keystore"        # clé privée chiffrée Argon2id

[network]
mode         = "client"                    # client | relay | rendezvous | full
listen_port  = 7777                        # port QUIC entrant
relay        = ""                          # adresse du relay (mode client)
rendezvous   = []                          # adresses rendezvous (mode relay)
domain       = ""                          # domaine public (mode relay)

[storage]
db_path      = "~/.neonet/neonet.db"       # SQLite
max_db_size  = "10GB"
```

**Fichiers générés par le daemon :**

| Fichier | Description |
|---|---|
| `~/.neonet/session.token` | Token Bearer courant (chmod 600, renouvelé à chaque démarrage) |
| `~/.neonet/daemon.pid` | PID du daemon background (créé avec `--daemon`) |
| `~/.neonet/daemon.log` | Logs du daemon background (créé avec `--daemon`) |

> **Docker** : avec `ENV HOME=/data`, tous ces chemins deviennent `/data/.neonet/…`.
> Le bind mount `./neonetconf:/data` dans docker-compose suffit à tout persister.

**Rotation du token :** le token est régénéré à chaque démarrage du daemon.
Les apps clientes doivent relire `session.token` après un redémarrage.
Le token est trimé (sans `\n` final) — toujours utiliser la valeur exacte du fichier.

---

## 7. Crates Rust

| Crate | Usage |
|---|---|
| `actix-web` | Serveur HTTP REST |
| `actix-ws` | WebSocket |
| `serde` + `serde_json` | Sérialisation JSON |
| `tokio` | Runtime async |
| `uuid` | Génération du session token |
| `argon2` | Dérivation de clé pour le keystore |
| `reqwest` | Client HTTP pour `neonet status` |
| `libc` | SIGTERM Unix pour `neonet stop` |

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Transport | HTTP REST + WebSocket |
| Port défaut | `127.0.0.1:7780` |
| Auth | Bearer token, fichier `~/.neonet/session.token` chmod 600 |
| Rotation token | À chaque démarrage du daemon |
| Token format | UUID v4, trimé sans `\n`, valide tant que le daemon tourne |
| Scopes | Aucun pour l'instant — open bar localhost |
| Déchiffrement | Côté daemon — l'app reçoit du JSON clair |
| Clé privée | Jamais exposée — opérations via `/v1/identity/sign` |
| Erreurs | HTTP status codes + body JSON `{ error, message }` |
| WebSocket | Un seul par app, subscribe par room_ids |
| Création room | Membres initiaux inclus dans le POST |
| Peers | Exposés via `/v1/peers` pour debug et monitoring |