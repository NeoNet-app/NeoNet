# NeoNet — Rooms

---

## 1. Définition

Une **Room** est l'unité de base de communication dans NeoNet. C'est un espace partagé dans lequel des membres échangent des DAG events et maintiennent un état CRDT commun.

Une Room peut représenter :
- Une conversation privée (2 personnes)
- Un groupe (N personnes)
- Un canal public (communauté)
- Un fil thématique (thread)

---

## 2. Identifiant de Room

```
room_id = blake3(creator_pubkey || nonce || ts_hint)
```

- Généré par le créateur au moment de la création
- Globalement unique sans coordination centrale
- Opaque — ne révèle pas l'identité du créateur sur le réseau

---

## 3. Structure d'une Room

Une Room est composée de deux couches :

```
Room
 ├── RoomState       (CRDT — converge automatiquement)
 │    ├── meta       nom, description, avatar_hash
 │    ├── members    { pubkey → MemberInfo }
 │    ├── settings   { clé → valeur }
 │    └── acl        { pubkey → Role }
 │
 └── EventGraph      (DAG — ordre causal)
      ├── root       event genesis de la room
      └── events     { event_id → DagEvent }
```

---

## 4. RoomState — CRDT

### Structure

```rust
struct RoomState {
    room_id:     RoomId,               // [u8; 32]
    meta:        LwwMap<String, Bytes>, // Last-Write-Wins
    members:     OrSet<MemberInfo>,    // Observed-Remove Set
    settings:    LwwMap<String, Bytes>,
    acl:         LwwMap<PublicKey, Role>,
}

struct MemberInfo {
    pubkey:      [u8; 32],             // Ed25519
    role:        Role,
    joined_at:   u64,                  // ts hint
    display_name: Option<String>,      // hint local, non-autoritatif
}

enum Role {
    Owner  = 0,    // peut tout faire, inclus supprimer la room
    Admin  = 1,    // peut gérer les membres et settings
    Member = 2,    // peut lire et écrire
    Guest  = 3,    // lecture seule
}
```

### Types CRDT utilisés

| Champ | Type CRDT | Raison |
|---|---|---|
| `meta` | LWW Map | Dernière écriture gagne, simple et suffisant |
| `members` | OR-Set | Gère les add/remove concurrents correctement |
| `settings` | LWW Map | Idem meta |
| `acl` | LWW Map | Un seul rôle par clé à la fois |

### Opérations sur RoomState

Toutes les opérations sont des CRDT ops signées (kind `0x0100`–`0x0102`) :

```rust
enum RoomOp {
    SetMeta     { key: String, value: Bytes, vclock: VClock },
    AddMember   { info: MemberInfo, vclock: VClock },
    RemoveMember{ pubkey: PublicKey, vclock: VClock },
    SetRole     { pubkey: PublicKey, role: Role, vclock: VClock },
    SetSetting  { key: String, value: Bytes, vclock: VClock },
}
```

---

## 5. Chiffrement des Rooms

### Room privée (défaut)

La Room a une **clé symétrique de room** (`room_key`) générée à la création :

```
room_key = ChaCha20-Poly1305 key (256 bits, aléatoire)
```

Cette clé est distribuée aux membres chiffrée avec leur clé publique X25519 :

```
for each member:
    encrypted_room_key[member] = ECIES(member.x25519_pubkey, room_key)
```

Tout le `payload` des DAG events et CRDT ops est chiffré avec `room_key`. Les nœuds relais ne voient que des blobs opaques.

### Room publique

`room_key` est publique et distribuée librement (publiée en DHT). N'importe qui peut rejoindre et lire.

### Rotation de clé

Quand un membre est exclu, la `room_key` doit être rotée pour garantir la **forward secrecy post-exclusion** :

```
1. Admin génère new_room_key
2. Distribue new_room_key chiffré aux membres restants
3. Les nouveaux events utilisent new_room_key
4. Les anciens events restent chiffrés avec l'ancienne clé
```

---

## 6. Event genesis

Chaque Room commence par un event genesis — le seul event sans parents :

```rust
struct GenesisPayload {
    room_id:          RoomId,
    creator:          PublicKey,
    room_type:        RoomType,
    initial_state:    RoomState,       // état initial (membres, settings)
    encrypted_keys:   HashMap<PublicKey, Bytes>, // room_key chiffré par membre
}

enum RoomType {
    DirectMessage = 0,   // exactement 2 membres, pas d'admin
    Group         = 1,   // N membres, avec rôles
    Channel       = 2,   // potentiellement public, lecture-seule pour guests
    Thread        = 3,   // rattaché à un event parent d'une autre room
}
```

L'event genesis a `kind = 0x0000` et `parents = []`.

---

## 7. Contrôle d'accès (ACL)

Les permissions sont vérifiées localement par chaque nœud à la réception d'un event :

| Action | Owner | Admin | Member | Guest |
|---|---|---|---|---|
| Envoyer un message | ✅ | ✅ | ✅ | ❌ |
| Modifier les meta | ✅ | ✅ | ❌ | ❌ |
| Ajouter un membre | ✅ | ✅ | ❌ | ❌ |
| Exclure un membre | ✅ | ✅* | ❌ | ❌ |
| Changer les rôles | ✅ | ✅* | ❌ | ❌ |
| Supprimer la room | ✅ | ❌ | ❌ | ❌ |
| Lire les messages | ✅ | ✅ | ✅ | ✅ |

*Un Admin ne peut pas modifier le rôle d'un Owner ni s'élever à Owner.

Un event qui viole l'ACL est **silencieusement ignoré** — pas d'erreur renvoyée (pas d'oracle).

---

## 8. Découverte d'une Room

### Room privée

Partagée hors-bande : lien NeoNet, QR code, invitation directe.

Format d'un lien d'invitation :

```
neonet://join/{room_id_base64url}?key={encrypted_room_key_base64url}&via={rendezvous_addr}
```

### Room publique

Publiée en DHT sous la clé :

```
dht_key = blake3("neonet-room-v1" || room_id)
dht_value = {
    room_id:    RoomId,
    name:       String,
    description: String,
    node_count: u32,      // hint, non-autoritatif
    entry_nodes: Vec<NodeInfo>,
}
```

---

## 9. Thread

Un Thread est une Room de type `Thread` rattachée à un event parent :

```rust
struct ThreadPayload {
    parent_room_id:  RoomId,
    parent_event_id: EventId,    // event auquel ce thread répond
}
```

Le thread hérite des membres et de la `room_key` de la room parente. Il a son propre DAG de messages.

---

## 10. Présence dans une Room

La présence est un CRDT op de kind `0x0101` avec un TTL court :

```rust
struct PresenceOp {
    room_id:    RoomId,
    pubkey:     PublicKey,
    status:     PresenceStatus,
    last_active: u64,            // ts hint
    ttl:        u32,             // secondes (recommandé: 60)
}

enum PresenceStatus {
    Online  = 0,
    Away    = 1,
    Offline = 2,
}
```

Les nœuds n'envoient pas de présence pour les rooms dont ils sont absents. La présence expire automatiquement après `ttl` secondes.

---

## 11. Crates Rust

| Crate | Usage |
|---|---|
| `crdts` | LWW Map, OR-Set |
| `blake3` | room_id, dht_key |
| `ed25519-dalek` | Vérification ACL (signatures des ops) |
| `x25519-dalek` | Chiffrement room_key par membre (ECIES) |
| `chacha20poly1305` | Chiffrement des payloads avec room_key |
| `postcard` | Sérialisation |
| `serde` | (De)sérialisation pour le stockage local |

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Identifiant | `blake3(creator_pubkey \|\| nonce \|\| ts)` |
| État | CRDT (LWW Map + OR-Set) |
| Messages | DAG (cf. 06-sync.md) |
| Chiffrement | ChaCha20 room_key, distribuée par ECIES |
| Rotation de clé | À chaque exclusion de membre |
| ACL | Vérifiée localement, violation ignorée silencieusement |
| Découverte privée | Lien hors-bande `neonet://join/...` |
| Découverte publique | DHT sous `blake3("neonet-room-v1" \|\| room_id)` |
| Présence | CRDT op avec TTL court (60s recommandé) |
| Threads | Room enfant rattachée à un event parent |