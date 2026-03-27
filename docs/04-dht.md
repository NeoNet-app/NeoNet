# NeoNet — DHT Kademlia

---

## 1. Objectifs

- Découverte de pairs sans serveur central après le bootstrap
- Routage efficace O(log n) dans un réseau de n nœuds
- Résistance aux attaques Sybil via ancrage sur les identités Ed25519
- Compatibilité avec les nœuds derrière NAT (via relais)

---

## 2. Identifiant de nœud

Dans NeoNet, l'identifiant DHT d'un nœud est dérivé de sa clé publique Ed25519 :

```
node_id = blake3(pubkey_ed25519)[0..20]   // 160 bits
```

Ce n'est **pas** un ID aléatoire comme dans Kademlia classique. L'ancrer sur la clé publique empêche un attaquant de choisir librement son ID pour cibler une zone de la DHT (résistance Sybil de base).

---

## 3. Distance XOR

La distance entre deux nœuds est définie par le XOR de leurs IDs :

```
distance(a, b) = a XOR b
```

Le routage consiste toujours à se rapprocher de la cible en XOR. C'est la métrique Kademlia standard.

---

## 4. Routing table — K-buckets

Chaque nœud maintient une routing table composée de **160 buckets** (un par bit de distance).

```rust
struct RoutingTable {
    local_id: NodeId,                  // [u8; 20]
    buckets:  [KBucket; 160],
}

struct KBucket {
    nodes:      VecDeque<NodeInfo>,    // max K entrées (K = 20 dans NeoNet)
    last_seen:  Instant,
}

struct NodeInfo {
    id:         NodeId,                // [u8; 20]
    addr:       SocketAddr,
    pubkey:     [u8; 32],              // Ed25519, pour vérifier les messages DHT
    last_seen:  Instant,
    rtt:        Duration,              // latence mesurée
    node_type:  NodeType,              // Full | Relay | Light
}
```

### Paramètres

| Paramètre | Valeur | Description |
|---|---|---|
| K | 20 | Taille max d'un bucket |
| Alpha | 3 | Parallélisme des lookups |
| T_refresh | 1h | Intervalle de refresh des buckets inactifs |
| T_republish | 24h | Intervalle de republication des valeurs stockées |
| T_expire | 48h | Durée de vie max d'une valeur en DHT |

### Règle de remplacement

Quand un bucket est plein et qu'un nouveau nœud arrive :

1. Pinguer le nœud le plus ancien du bucket
2. S'il répond → le garder, ignorer le nouveau (les vieux nœuds sont plus fiables)
3. S'il ne répond pas → le remplacer par le nouveau

---

## 5. Les 4 messages DHT

Tous les messages DHT sont transportés dans des frames `FrameKind::DhtMessage` (cf. `03-handshake.md`), donc chiffrés et authentifiés.

### PING

Vérifie qu'un nœud est vivant.

```rust
struct Ping {
    request_id: [u8; 8],
    sender_id:  NodeId,
}

struct Pong {
    request_id: [u8; 8],
    sender_id:  NodeId,
}
```

### FIND_NODE

Demande les K nœuds les plus proches d'un target ID.

```rust
struct FindNode {
    request_id: [u8; 8],
    sender_id:  NodeId,
    target:     NodeId,
}

struct FindNodeResp {
    request_id: [u8; 8],
    sender_id:  NodeId,
    closest:    Vec<NodeInfo>,         // max K entrées
}
```

### FIND_VALUE

Demande une valeur stockée en DHT. Si le nœud ne l'a pas, retourne les K nœuds les plus proches (comme FIND_NODE).

```rust
struct FindValue {
    request_id: [u8; 8],
    sender_id:  NodeId,
    key:        [u8; 32],              // blake3 de la clé applicative
}

enum FindValueResp {
    Found {
        request_id: [u8; 8],
        value:      Vec<u8>,           // toujours chiffré E2EE
    },
    Closest {
        request_id: [u8; 8],
        nodes:      Vec<NodeInfo>,
    },
}
```

### STORE

Demande à un nœud de stocker une valeur.

```rust
struct Store {
    request_id: [u8; 8],
    sender_id:  NodeId,
    key:        [u8; 32],
    value:      Vec<u8>,               // chiffré E2EE
    sig:        [u8; 64],              // signature Ed25519 sur (key || value || ts)
    ts:         u64,                   // timestamp hint (non-autoritatif)
    ttl:        u32,                   // secondes avant expiration (max T_expire)
}

struct StoreResp {
    request_id: [u8; 8],
    accepted:   bool,
}
```

La signature dans STORE empêche un nœud malveillant de publier des valeurs au nom d'un autre.

---

## 6. Algorithme de lookup

Utilisé par FIND_NODE et FIND_VALUE. Parallélisme alpha = 3.

```
1. Initialiser shortlist avec les K nœuds locaux les plus proches de target
2. Marquer tous comme "non contactés"
3. Répéter :
     a. Sélectionner les Alpha nœuds non contactés les plus proches
     b. Envoyer FIND_NODE (ou FIND_VALUE) en parallèle
     c. Pour chaque réponse :
          - Ajouter les nouveaux nœuds à la shortlist
          - Mettre à jour la routing table locale
     d. Marquer les nœuds contactés
     e. Si aucun nœud plus proche trouvé depuis le dernier round → arrêter
4. Retourner les K nœuds les plus proches du target
```

Timeout par requête : 5 secondes. Un nœud qui ne répond pas est marqué suspect et retiré de la shortlist.

---

## 7. Bootstrap depuis les rendezvous

```
1. Se connecter à un nœud rendezvous (cf. 02-network.md)
2. Envoyer BootstrapReq — demande des pairs DHT
3. Recevoir BootstrapResp — liste de NodeInfo (max 20)
4. Pour chaque pair reçu :
     a. Envoyer PING pour vérifier qu'il est vivant
     b. S'il répond → l'ajouter à la routing table
5. Lancer un FIND_NODE sur notre propre node_id
   → peuple les buckets proches de nous
6. Si routing table >= 8 nœuds confirmés → bootstrap terminé
```

```rust
struct BootstrapReq {
    request_id: [u8; 8],
    sender_id:  NodeId,
    node_type:  NodeType,
}

struct BootstrapResp {
    request_id: [u8; 8],
    peers:      Vec<NodeInfo>,         // max 20
}
```

---

## 8. Résistance aux attaques Sybil

NeoNet ne peut pas éliminer les attaques Sybil complètement (aucune DHT publique ne le peut), mais applique plusieurs mesures :

**Ancrage des IDs sur Ed25519**
`node_id = blake3(pubkey)` — un attaquant ne peut pas choisir librement son ID sans générer une nouvelle clé. Cibler une zone précise de la DHT requiert de bruteforcer des paires de clés.

**Diversité des IPs**
Un bucket ne peut pas contenir plus de **2 nœuds avec le même /24 IPv4** ou **/48 IPv6**. Limite l'impact d'un attaquant qui contrôle un bloc d'adresses.

**Vérification des signatures sur STORE**
Toute valeur stockée en DHT doit être signée par une clé Ed25519 valide. Un nœud malveillant ne peut pas injecter de fausses valeurs au nom d'un autre.

**Pas de valeurs sensibles en DHT**
Les messages NeoNet ne transitent pas par la DHT — seulement les métadonnées de découverte (adresses de nœuds). Même une DHT partiellement compromise ne donne pas accès aux messages.

---

## 9. Refresh et maintenance

```
Toutes les heures :
  Pour chaque bucket non accédé depuis T_refresh :
    Choisir un ID aléatoire dans la range du bucket
    Lancer un FIND_NODE sur cet ID
    → force le refresh des nœuds dans ce bucket

Toutes les 24h :
  Republier les valeurs STORE dont ce nœud est responsable

À chaque démarrage :
  Relancer le bootstrap complet (FIND_NODE sur notre propre ID)
```

---

## 10. Nœuds derrière NAT

Les clients légers (mobile, browser) ne peuvent pas recevoir de connexions entrantes. Ils s'enregistrent auprès d'un **nœud relais** qui les représente dans la DHT.

```
Client léger ──► Relais NeoNet ──► DHT
                     │
                     └─ publie addr du relais comme point de contact
                        pour le client
```

Le relais ne voit que des frames chiffrées — il ne peut pas lire le contenu des messages.

---

## 11. Crates Rust

| Crate | Usage |
|---|---|
| `blake3` | Calcul des node_id |
| `ed25519-dalek` | Vérification des signatures STORE |
| `quinn` | Transport QUIC des messages DHT |
| `postcard` | Sérialisation des messages DHT |
| `tokio` | Runtime async pour le lookup parallèle |
| `rand` | Génération des request_id, IDs de refresh |

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Algorithme | Kademlia |
| Taille node_id | 160 bits |
| Dérivation node_id | `blake3(pubkey_ed25519)[0..20]` |
| K (bucket size) | 20 |
| Alpha (parallélisme) | 3 |
| Messages | PING, FIND_NODE, FIND_VALUE, STORE |
| Sérialisation | postcard |
| Transport | Frames chiffrées NeoNet (03-handshake.md) |
| Anti-Sybil | ID ancré sur Ed25519 + diversité IP par bucket |
| Valeurs DHT | Signées Ed25519, chiffrées E2EE |
| NAT traversal | Nœuds relais comme proxy DHT |