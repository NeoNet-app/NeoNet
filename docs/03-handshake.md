# NeoNet — Handshake & Session

---

## 1. Objectifs

- Authentifier mutuellement deux nœuds via leur identité Ed25519
- Établir un canal chiffré forward-secret via X25519 + ChaCha20-Poly1305
- Minimiser les métadonnées visibles pendant le handshake
- Supporter le mode sealed sender (expéditeur masqué)
- Fonctionner au-dessus de QUIC (transport par défaut)

---

## 2. Vue d'ensemble

```
Initiateur (A)                          Répondeur (B)
      │                                       │
      │──── HelloInit ────────────────────────►│
      │     eph_pubkey_a                       │
      │     padding                            │
      │                                        │
      │◄─── HelloResp ────────────────────────│
      │     eph_pubkey_b                       │
      │     identity_b (chiffré)               │
      │     padding                            │
      │                                        │
      │  [dérivation clés session]             │
      │  shared = X25519(eph_priv_a, eph_pub_b)│
      │  session_key = HKDF(shared, ...)       │
      │                                        │
      │──── Finish ───────────────────────────►│
      │     identity_a (chiffré)               │
      │     session_id                         │
      │     padding                            │
      │                                        │
      │◄─── Ack ──────────────────────────────│
      │     session_id confirmé                │
      │                                        │
      │  [session établie — frames chiffrées]  │
```

Le handshake est en **4 messages**. Après le `Ack`, tout le trafic est chiffré avec la clé de session.

---

## 3. Dérivation des clés

### Échange X25519

```
eph_shared = X25519(eph_priv_a, eph_pub_b)
           = X25519(eph_priv_b, eph_pub_a)   // commutatif
```

Chaque nœud génère une paire éphémère **par session** — jamais réutilisée. Cela garantit la **forward secrecy** : compromettre une clé de long terme ne déchiffre pas les sessions passées.

### HKDF

```
ikm     = eph_shared
salt    = "neonet-v1-session"          // domaine fixe
info    = session_id || pubkey_a || pubkey_b
output  = 64 bytes

session_key_ab = output[0..32]         // A → B
session_key_ba = output[32..64]        // B → A
```

Les deux sens ont des clés distinctes — pas de risque de réutilisation de nonce.

### Chiffrement des identités dans le handshake

Les champs `identity_a` et `identity_b` sont chiffrés avec `session_key_ab` **avant** que le Finish soit envoyé. Un observateur réseau ne voit jamais les clés publiques en clair pendant le handshake.

---

## 4. Format des messages de handshake

Tous les messages sont sérialisés avec **postcard** (format binaire compact, no_std compatible).

### HelloInit

```rust
struct HelloInit {
    version:     u8,           // version protocole NeoNet (actuellement 1)
    eph_pubkey:  [u8; 32],     // clé publique éphémère X25519 de A
    padding:     Vec<u8>,      // padding aléatoire (longueur variable, voir §7)
}
```

### HelloResp

```rust
struct HelloResp {
    eph_pubkey:   [u8; 32],    // clé publique éphémère X25519 de B
    identity_enc: Vec<u8>,     // IdentityPayload de B, chiffré ChaCha20-Poly1305
    padding:      Vec<u8>,
}
```

### IdentityPayload (chiffré dans HelloResp et Finish)

```rust
struct IdentityPayload {
    pubkey:     [u8; 32],      // clé publique Ed25519 long terme
    sig:        [u8; 64],      // signature Ed25519 sur (session_id || eph_pubkey)
    node_type:  NodeType,      // Full | Relay | Light
    features:   u32,           // bitfield de features supportées
}

enum NodeType { Full = 0, Relay = 1, Light = 2 }
```

### Finish

```rust
struct Finish {
    session_id:   [u8; 16],    // identifiant de session aléatoire (généré par A)
    identity_enc: Vec<u8>,     // IdentityPayload de A, chiffré
    padding:      Vec<u8>,
}
```

### Ack

```rust
struct Ack {
    session_id: [u8; 16],      // echo du session_id reçu dans Finish
    padding:    Vec<u8>,
}
```

---

## 5. Format des frames de session

Après le handshake, tout le trafic est encapsulé dans des **frames chiffrées**.

```rust
struct Frame {
    len:      u32,             // longueur totale de la frame (header compris)
    nonce:    [u8; 12],        // nonce ChaCha20-Poly1305 (96 bits, incrémental)
    ciphertext: Vec<u8>,       // payload chiffré + tag Poly1305 (16 bytes)
    padding:  Vec<u8>,         // padding uniforme (voir §7)
}
```

### Payload déchiffré d'une frame

```rust
struct FramePayload {
    kind:    FrameKind,        // type de contenu
    data:    Vec<u8>,          // contenu sérialisé (postcard)
}

enum FrameKind {
    DagEvent    = 0x01,        // NeoNet DAG event (cf. 01-concept.md)
    CrdtOp      = 0x02,        // NeoNet CRDT operation
    DhtMessage  = 0x10,        // message DHT (cf. 04-dht.md)
    Ping        = 0x20,        // keepalive
    Pong        = 0x21,
    Bootstrap   = 0x30,        // demande de pairs au rendezvous
    BootstrapResp = 0x31,
}
```

### Gestion du nonce

Le nonce est un compteur 96 bits **incrémenté de 1 à chaque frame**. Il ne doit jamais être réutilisé avec la même clé. Si le compteur atteint `2^64`, la session doit être re-négociée.

---

## 6. Sealed sender (optionnel)

En mode sealed sender, l'identité de A n'est **jamais révélée à B** pendant le handshake. Seul B peut déchiffrer le message grâce à sa clé privée.

```
identity_enc dans Finish = chiffré avec pubkey_b (ECIES X25519)
                           au lieu de session_key_ab
```

B sait qu'il a reçu un message valide signé par une clé NeoNet, mais ne peut pas relier cette clé à une identité connue sans information supplémentaire. Utilisé pour les messages haute-confidentialité et les cas d'usage anonymes.

---

## 7. Padding uniforme

Pour résister à l'analyse de trafic, **tous les messages** (handshake et frames) sont paddés à une longueur multiple de 256 bytes.

```
padded_len = ceil(real_len / 256) * 256
padding    = random bytes de longueur (padded_len - real_len)
```

Cela empêche de déduire le type ou la taille du contenu depuis l'observation des paquets réseau.

---

## 8. Vérification des identités

À la réception de `HelloResp` et `Finish`, chaque nœud vérifie :

1. Déchiffrer `identity_enc` avec la clé de session
2. Vérifier la signature Ed25519 : `verify(sig, session_id || eph_pubkey, identity.pubkey)`
3. Si le nœud cible était connu (via DHT ou rendezvous), vérifier que `identity.pubkey` correspond
4. Vérifier que `session_id` dans `Ack` correspond au `session_id` envoyé dans `Finish`

Si une vérification échoue → fermer la connexion QUIC immédiatement, sans message d'erreur explicite (pas d'oracle).

---

## 9. Crates Rust

| Crate | Usage |
|---|---|
| `x25519-dalek` | Échange de clés éphémères |
| `ed25519-dalek` | Signatures d'identité |
| `chacha20poly1305` | Chiffrement des frames |
| `hkdf` + `sha2` | Dérivation des clés de session |
| `postcard` | Sérialisation binaire des messages |
| `quinn` | Transport QUIC sous-jacent |
| `rand` | Génération nonces, padding, clés éphémères |

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Échange de clés | X25519 éphémère par session |
| Dérivation | HKDF-SHA256 avec domaine `neonet-v1-session` |
| Chiffrement session | ChaCha20-Poly1305, clés séparées par sens |
| Sérialisation | postcard (binaire compact) |
| Identités dans handshake | Chiffrées, jamais en clair sur le réseau |
| Forward secrecy | Garanti — clés éphémères jetées après HKDF |
| Sealed sender | Optionnel, ECIES X25519 |
| Padding | Multiple de 256 bytes, aléatoire |
| Erreurs | Pas d'oracle — fermeture silencieuse |