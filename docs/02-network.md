# NeoNet — Réseau & Découverte de pairs

---

## 1. Modèle de découverte — Hybride Bootstrap + DHT

NeoNet utilise une découverte en deux phases :

**Phase 1 — Bootstrap via rendezvous**
Le client démarre avec une liste de nœuds rendezvous connus. Il s'y connecte pour obtenir ses premiers pairs DHT. Les rendezvous sont des annuaires éphémères — ils ne stockent rien de permanent et ne voient que l'IP de bootstrap.

**Phase 2 — DHT Kademlia**
Une fois les premiers pairs connus, le client rejoint la DHT et découvre le réseau organiquement. Le rendezvous n'est plus nécessaire. Le client lui est invisible.

```
Client
  │
  ├─── 1. Bootstrap ──► Rendezvous  (voit ton IP, te donne des pairs)
  │                          │
  │         pairs DHT ◄──────┘
  │
  └─── 2. DHT ─────────► Pair A ──► Pair B ──► ...
                         (rendezvous ne voit plus rien)
```

**Propriété privacy importante :** le rendezvous voit l'IP au moment du bootstrap uniquement. Pour les cas haute-sécurité, le bootstrap peut passer par Tor ou un relais NeoNet.

---

## 2. Convention `.neonet/` — Publication web

Tout domaine peut publier des métadonnées NeoNet sous un chemin conventionnel :

```
https://exemple.com/.neonet/
```

C'est le point d'entrée unique pour toute la configuration publique d'un domaine NeoNet. Le principe est similaire à `/.well-known/` (RFC 5785) mais dédié à NeoNet.

### Fichiers définis

| Chemin | Type | Description |
|---|---|---|
| `/.neonet/rendezvous.toml` | TOML | Liste des nœuds rendezvous du domaine |
| `/.neonet/identity.toml` | TOML | Clé publique et métadonnées du domaine *(à définir)* |
| `/.neonet/policy.toml` | TOML | Politique de fédération du domaine *(à définir)* |

> Les fichiers marqués *à définir* sont réservés pour des specs futures. Seul `rendezvous.toml` est spécifié ici.

### Règles de serving

- Servi en **HTTPS uniquement** (pas de HTTP)
- Content-Type : `application/toml`
- CORS : `Access-Control-Allow-Origin: *` recommandé (lecture publique)
- Cache : `Cache-Control: max-age=3600` recommandé

---

## 3. Format — `rendezvous.toml`

Chemin canonique : `https://exemple.com/.neonet/rendezvous.toml`

### Spec complète

```toml
# ─────────────────────────────────────────────
# NeoNet Rendezvous List
# Spec version : 1
# ─────────────────────────────────────────────

[meta]
spec        = 1                          # version de cette spec (integer)
name        = "Nom lisible de la liste"  # nom affiché dans les clients
maintainer  = "@pubkey:exemple.com"      # identité NeoNet du mainteneur
created_at  = 2025-01-01T00:00:00Z      # date de création (RFC 3339)
updated_at  = 2025-06-01T12:00:00Z      # date de dernière mise à jour (RFC 3339)
expires_at  = 2026-01-01T00:00:00Z      # optionnel — après cette date, ignorer
sig         = "ed25519:BASE64URL"        # signature Ed25519 du fichier entier
                                         # (champ sig mis à "" avant de signer)

[[nodes]]
addr   = "rendezvous1.exemple.com:7777" # adresse du nœud (host:port)
pubkey = "ed25519:BASE64URL"            # clé publique du nœud (authentification)
region = "eu-west"                      # optionnel — hint géographique
tor    = false                          # optionnel — accessible via Tor (défaut: false)

[[nodes]]
addr   = "rendezvous2.exemple.com:7777"
pubkey = "ed25519:BASE64URL"
region = "us-east"
tor    = false

[[nodes]]
addr   = "abcdef1234567890.onion:7777"  # nœud Tor (addr .onion)
pubkey = "ed25519:BASE64URL"
region = ""
tor    = true
```

### Champs obligatoires

| Champ | Type | Description |
|---|---|---|
| `meta.spec` | integer | Version de la spec. Actuellement `1`. |
| `meta.name` | string | Nom humain de la liste. |
| `meta.maintainer` | string | Identité NeoNet du mainteneur (`@pubkey:domain`). |
| `meta.sig` | string | Signature Ed25519 encodée en BASE64URL, préfixée `ed25519:`. |
| `nodes[].addr` | string | Adresse `host:port` du nœud rendezvous. |
| `nodes[].pubkey` | string | Clé publique Ed25519 du nœud, préfixée `ed25519:`. |

### Champs optionnels

| Champ | Type | Défaut | Description |
|---|---|---|---|
| `meta.created_at` | RFC 3339 | — | Date de création. |
| `meta.updated_at` | RFC 3339 | — | Date de dernière modification. |
| `meta.expires_at` | RFC 3339 | — | Date d'expiration. Passée cette date, le client ignore la liste. |
| `nodes[].region` | string | `""` | Hint géographique libre (`eu-west`, `us-east`, etc.). |
| `nodes[].tor` | bool | `false` | `true` si l'adresse est une `.onion`. |

### Procédure de signature

1. Construire le fichier TOML complet avec `meta.sig = ""`
2. Sérialiser en UTF-8 canonique (pas de trailing whitespace, LF uniquement)
3. Signer avec la clé privée Ed25519 du mainteneur
4. Encoder la signature en BASE64URL
5. Remplacer `meta.sig = ""` par `meta.sig = "ed25519:BASE64URL"`

Le client vérifie la signature en reconstituant le fichier avec `meta.sig = ""` avant de vérifier.

---

## 4. Découverte d'une liste de rendezvous

Un client NeoNet cherche une liste de rendezvous dans cet ordre de priorité :

```
1. Config locale       ~/.config/neonet/rendezvous.toml  (priorité max)
2. Argument CLI        --rendezvous https://exemple.com/.neonet/rendezvous.toml
3. Listes embarquées   dans le binaire (rendezvous NeoNet officiels)
```

Les listes sont **agrégées** — un client peut utiliser plusieurs listes simultanément. Il tente de se connecter à plusieurs nœuds en parallèle et garde les N premiers qui répondent.

---

## 5. Comportement du client au bootstrap

```
1. Charger toutes les listes rendezvous disponibles
2. Vérifier la signature de chaque liste (rejeter si invalide)
3. Vérifier expires_at (ignorer les listes expirées)
4. Mélanger aléatoirement les nœuds (éviter de toujours contacter le même)
5. Tenter des connexions QUIC en parallèle (max 5 simultanées)
6. Pour chaque rendezvous contacté :
     a. Vérifier son pubkey (TLS + handshake NeoNet)
     b. Demander une liste de pairs DHT (protocole NeoNet bootstrap)
     c. Ajouter ces pairs à la routing table DHT locale
7. Une fois >= 8 pairs DHT connus → passer en mode DHT pur
8. Déconnecter des rendezvous (optionnel — garder 1 connexion pour les nouveaux clients)
```

**Seuil minimal :** 8 pairs DHT avant de considérer le bootstrap terminé. En dessous, continuer à tenter d'autres rendezvous.

---

## 6. Rôle des communautés

N'importe quelle communauté peut héberger ses propres rendezvous et publier sa liste :

```
https://monserveur.org/.neonet/rendezvous.toml
```

Les utilisateurs ajoutent cette URL dans leur config ou la partagent (lien, QR code). Une fois dans la DHT, ils ne dépendent plus de ce rendezvous. Les rendezvous communautaires permettent de bootstrapper des **sous-réseaux distincts** tout en restant sur le même protocole NeoNet.

Exemples de cas d'usage :
- Une communauté journalistes avec ses propres nœuds Tor-only
- Un réseau d'entreprise avec rendezvous internes non-publics
- Un réseau gaming avec nœuds géolocalisés pour la latence

---

## 7. Ce que le rendezvous ne fait PAS

- Il ne stocke pas de messages
- Il ne connaît pas les identités des utilisateurs
- Il ne voit que l'IP de bootstrap (et seulement pendant la connexion initiale)
- Il ne peut pas bloquer un utilisateur de la DHT une fois celui-ci bootstrappé
- Il n'est pas un point de confiance — sa `pubkey` est vérifiée, pas sa bonne volonté

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Modèle de découverte | Hybride Bootstrap rendezvous + DHT Kademlia |
| Convention web | `https://domaine/.neonet/` |
| Fichier rendezvous | `/.neonet/rendezvous.toml` |
| Format | TOML signé Ed25519 |
| Authentification nœud | `pubkey` Ed25519 par nœud |
| Transport bootstrap | QUIC (Tor optionnel) |
| Seuil DHT | 8 pairs avant migration |
| Listes | Agrégées, priorité config locale > CLI > embarqué |