# NeoNet — Sync & DAG

---

## 1. Objectifs

- Synchroniser le graphe de messages d'une Room entre pairs
- Garantir la convergence même après partition réseau
- Minimiser les données transférées (sync différentiel)
- Gérer les conflits de branches concurrentes sans coordinateur

---

## 2. Structure du DAG

Chaque Room possède un **Event Graph** — un DAG orienté acyclique de messages.

```
genesis (parents=[])
    │
    ▼
event_001 (parents=[genesis])
    │
    ▼
event_002 (parents=[event_001])
    │
    ├──────────────────┐
    ▼                  ▼
event_003a          event_003b       ← branche concurrente
(parents=[002])     (parents=[002])
    │                  │
    └────────┬─────────┘
             ▼
         event_004 (parents=[003a, 003b])  ← merge implicite
```

Un event peut avoir **plusieurs parents** — c'est ce qui représente la concurrence. Il n'y a pas d'opération de merge explicite : un event qui référence deux branches les fusionne implicitement.

---

## 3. Représentation locale

```rust
struct EventGraph {
    room_id:   RoomId,
    events:    HashMap<EventId, StoredEvent>,
    tips:      HashSet<EventId>,       // events sans successeurs connus (têtes du DAG)
    roots:     HashSet<EventId>,       // events sans parents (= genesis)
}

struct StoredEvent {
    event:     DagEvent,               // l'event tel que reçu (cf. 01-concept.md)
    received_at: Instant,              // quand on l'a reçu (local uniquement)
    children:  HashSet<EventId>,       // successeurs connus
    state:     EventState,
}

enum EventState {
    Pending,       // reçu mais parents manquants
    Valid,         // parents connus et signature vérifiée
    Invalid,       // signature invalide ou ACL violée — ignoré
}
```

### Tips

Les **tips** sont les événements les plus récents du DAG — ceux qui n'ont pas encore de successeurs connus. Quand un client envoie un nouveau message, il référence tous les tips actuels comme parents :

```rust
fn new_event_parents(graph: &EventGraph) -> Vec<EventId> {
    graph.tips.iter().cloned().collect()
}
```

Cela garantit que chaque nouveau message "connaît" tout ce qui précède, fusionnant implicitement les branches concurrentes.

---

## 4. Protocole de synchronisation

La sync entre deux pairs A et B pour une Room donnée se fait en **3 phases**.

### Phase 1 — Échange de résumés (State Summary)

Chaque pair envoie un résumé compact de son état du DAG.

```rust
struct SyncSummary {
    room_id:    RoomId,
    tip_ids:    Vec<EventId>,          // tips connus
    event_count: u64,                  // nombre total d'events connus
    have_set:   BloomFilter,           // filtre de Bloom sur tous les event_ids connus
}
```

Le **filtre de Bloom** permet à B de détecter rapidement quels events de A lui manquent, sans envoyer la liste complète des IDs (qui peut être très longue).

```
A ──── SyncSummary ────► B
A ◄─── SyncSummary ───── B
```

### Phase 2 — Négociation des manquants (Want/Have)

Chaque pair calcule ce qu'il doit demander à l'autre :

```rust
struct SyncWant {
    room_id:    RoomId,
    event_ids:  Vec<EventId>,          // events que je veux
}

struct SyncHave {
    room_id:    RoomId,
    event_ids:  Vec<EventId>,          // events que j'ai et que tu n'as pas
}
```

```
A ──── SyncWant (events B a, A n'a pas) ────► B
A ◄─── SyncWant (events A a, B n'a pas) ───── B
```

### Phase 3 — Transfert des events

```rust
struct SyncBatch {
    room_id:    RoomId,
    events:     Vec<DagEvent>,         // max MAX_BATCH_SIZE events par message
    has_more:   bool,                  // d'autres batches suivent
}
```

```
A ──── SyncBatch ────► B
A ◄─── SyncBatch ───── B
```

Les events sont envoyés **dans l'ordre topologique** — les parents avant les enfants. Cela permet au récepteur de valider chaque event dès sa réception.

---

## 5. Paramètres de sync

| Paramètre | Valeur | Description |
|---|---|---|
| MAX_BATCH_SIZE | 256 | Events max par SyncBatch |
| BLOOM_FP_RATE | 0.01 | Taux de faux positifs du filtre de Bloom |
| SYNC_TIMEOUT | 30s | Timeout global d'une session de sync |
| MAX_PENDING_EVENTS | 1000 | Events en état Pending avant de déclencher une sync forcée |
| GOSSIP_FANOUT | 3 | Nombre de pairs à qui propager un nouvel event |

---

## 6. Validation d'un event reçu

À la réception d'un `DagEvent`, le nœud effectue dans l'ordre :

```
1. Vérifier la signature Ed25519 (author, payload, sig)
   → Échec : marquer Invalid, ignorer

2. Vérifier que l'auteur est membre de la room (ACL)
   → Échec : marquer Invalid, ignorer

3. Vérifier que tous les parents sont connus
   → Parents manquants : marquer Pending, déclencher SyncWant pour les parents

4. Déchiffrer le payload avec room_key
   → Échec déchiffrement : marquer Invalid, ignorer

5. Vérifier le kind (valeur u16 connue)
   → Kind inconnu : accepter mais ne pas interpréter (forward compatible)

6. Marquer Valid, ajouter à l'EventGraph
7. Mettre à jour les tips
8. Propager à d'autres pairs (gossip)
```

---

## 7. Gossip — propagation temps réel

Quand un nœud reçoit ou crée un nouvel event valide, il le propage à `GOSSIP_FANOUT` pairs aléatoires de la room :

```rust
struct GossipEvent {
    event: DagEvent,
}
```

Les pairs qui reçoivent un event via gossip le valident (§6) puis le propagent à leur tour si c'est la première fois qu'ils le voient.

Pour éviter les boucles, chaque nœud maintient un **cache des event_ids récemment vus** (LRU, 10 000 entrées, TTL 10 minutes). Un event déjà vu est ignoré sans être re-propagé.

---

## 8. Résolution d'ordre pour l'affichage

Le DAG ne définit qu'un **ordre partiel** — deux events concurrents (sans relation parent/enfant) n'ont pas d'ordre canonique. Pour l'affichage, NeoNet utilise un ordre déterministe :

```
1. Ordre causal d'abord (parent avant enfant)
2. À égalité causale → trier par ts (timestamp hint du client)
3. À égalité de ts → trier par event_id (lexicographique)
```

Cette règle est purement locale et cosmétique — elle n'affecte pas la validité des events. Deux clients peuvent afficher les events concurrents dans un ordre légèrement différent, c'est acceptable.

---

## 9. Gestion des events Pending

Un event est en état `Pending` quand ses parents ne sont pas encore connus. Cela arrive normalement lors d'une sync partielle ou d'une connexion tardive.

```
À la réception d'un event Pending :
  1. Stocker l'event localement (ne pas valider encore)
  2. Extraire les parent_ids manquants
  3. Envoyer SyncWant({ event_ids: missing_parents }) aux pairs

Quand un parent manquant arrive :
  1. Valider le parent
  2. Chercher tous les Pending qui l'attendaient
  3. Si tous leurs parents sont maintenant connus → valider

Si MAX_PENDING_EVENTS est atteint :
  → Déclencher une sync complète avec les pairs de la room
```

---

## 10. Sync au démarrage

Quand un nœud rejoint une room ou redémarre :

```
1. Charger l'EventGraph local depuis le stockage
2. Se connecter aux pairs de la room (via DHT)
3. Envoyer SyncSummary à chaque pair connecté
4. Traiter les SyncWant / SyncBatch reçus
5. Une fois sync terminée → passer en mode gossip
```

Pour les rooms avec un grand historique, la sync initiale peut être **partielle** — le nœud ne télécharge que les N derniers jours d'events et laisse le reste en lazy-fetch.

```rust
struct SyncSummaryPartial {
    room_id:     RoomId,
    tip_ids:     Vec<EventId>,
    since:       u64,              // ts hint — je veux seulement les events après cette date
    have_set:    BloomFilter,
}
```

---

## 11. Stockage local

```rust
trait EventStore {
    fn insert(&mut self, event: StoredEvent) -> Result<()>;
    fn get(&self, id: &EventId) -> Option<&StoredEvent>;
    fn tips(&self) -> Vec<EventId>;
    fn events_since(&self, ts: u64) -> Vec<&StoredEvent>;
    fn ancestors(&self, id: &EventId, depth: usize) -> Vec<&StoredEvent>;
}
```

Implémentation recommandée : **SQLite via `rusqlite`** avec un index sur `(room_id, ts_hint)` pour les requêtes partielles.

---

## 12. Messages de sync — récapitulatif des FrameKind

Ces types complètent l'enum `FrameKind` défini dans `03-handshake.md` :

```rust
// À ajouter à FrameKind
SyncSummary        = 0x40,
SyncSummaryPartial = 0x41,
SyncWant           = 0x42,
SyncHave           = 0x43,
SyncBatch          = 0x44,
GossipEvent        = 0x50,
```

---

## 13. Crates Rust

| Crate | Usage |
|---|---|
| `rusqlite` | Stockage local des events |
| `bloomfilter` | Filtre de Bloom pour SyncSummary |
| `postcard` | Sérialisation des messages de sync |
| `tokio` | Sync async entre pairs |
| `ed25519-dalek` | Validation des signatures |
| `lru` | Cache gossip (event_ids récents) |

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Structure | DAG orienté acyclique, parents multiples |
| Tips | Events sans successeurs — parents des nouveaux messages |
| Sync | 3 phases : Summary (Bloom) → Want/Have → Batch |
| Ordre topologique | Parents avant enfants dans les batches |
| Ordre d'affichage | Causal > ts hint > event_id lexicographique |
| Propagation | Gossip fanout=3, cache LRU anti-boucle |
| Events manquants | État Pending + SyncWant automatique |
| Sync partielle | `since` timestamp pour les grands historiques |
| Stockage | SQLite + rusqlite |
| Batch size | 256 events max par message |