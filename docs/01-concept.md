# NeoNet — Concept & Architecture

> Protocole décentralisé hybride, privacy-first, écrit en Rust.
> Objectif : remplacer Matrix.org avec une stack plus légère, plus sûre, et plus respectueuse des métadonnées.

---

## 1. Problèmes résolus par rapport à Matrix

| Problème Matrix | Solution NeoNet |
|---|---|
| Métadonnées trop exposées | Payload toujours chiffré, padding uniforme des paquets, sealed sender |
| Dépendance aux homeservers | Identité basée sur clé cryptographique, pas sur un serveur |
| Synchro lente / peu fiable | CRDT pour l'état, DAG léger pour les messages |
| Trop complexe / lourd | Protocol minimaliste, pas de JSON-LD, types binaires |

---

## 2. Modèle de décentralisation

NeoNet est **hybride fédération + P2P** :

- Deux clients peuvent se parler en **P2P direct** sans aucun serveur (si le réseau le permet)
- Des **nœuds complets** fédèrent entre eux (comme des homeservers, mais optionnels)
- Des **nœuds relais** servent de pont pour les clients derrière NAT ou mobile
- Les relais ne voient que des **blobs chiffrés opaques**

### Types de nœuds

| Type | Rôle |
|---|---|
| Nœud complet | Fédère + route + stocke l'état |
| Nœud relais | Pont fédération ↔ P2P, cache temporaire chiffré |
| Client léger | Mobile / browser, P2P direct si disponible |

---

## 3. Identité

Format : `@pubkey:domaine`

- La clé publique est une clé **Ed25519**
- L'identité est **portable** : elle ne dépend pas d'un serveur
- Si le domaine disparaît, l'identité reste valide (la clé privée suffit)
- Le domaine sert uniquement de hint de découverte initiale
- Mode anonyme possible : `author` optionnel dans les events

---

## 4. Modèle de données — Hybride DAG + CRDT

NeoNet utilise deux types de structures selon la nature de la donnée.

### 4.1 DAG Event (messages, réactions, fichiers)

Chaque message est un événement immuable avec un hash, qui pointe vers ses parents causaux.

```
DAG Event {
  id:       blake3(payload)       // dérivé du contenu, pas assigné par serveur
  author:   @pubkey:domain
  parents:  [id, ...]             // références causales
  kind:     u16                   // type d'event
  payload:  bytes                 // toujours chiffré E2EE
  sig:      Ed25519               // signature de l'auteur
  ts:       u64                   // hint non-autoritatif (ne pas faire confiance)
}
```

**Points clés :**
- `id = blake3(payload)` — deux nœuds qui reçoivent le même event arrivent au même ID, sans coordination
- `ts` est un hint déclaré par le client, **non-autoritatif** — l'ordre réel est déterminé par le graphe des `parents`
- `payload` est toujours chiffré — les relais ne voient que des blobs opaques

### 4.2 CRDT State (présence, membres, settings)

Les données qui changent souvent et n'ont pas besoin d'historique causal utilisent des CRDTs. Le merge est mathématiquement garanti, sans coordinateur.

```
CRDT State {
  doc_id:   blake3(namespace)
  author:   @pubkey:domain
  vclock:   {pubkey: u64}         // horloge vectorielle
  op:       LWW | Set | Counter   // type d'opération CRDT
  payload:  bytes                 // toujours chiffré E2EE
  sig:      Ed25519
  ttl:      Option<u64>           // expiration optionnelle (ex: présence)
}
```

### 4.3 Pourquoi pas full DAG ou full CRDT ?

| | DAG | CRDT |
|---|---|---|
| Ordre causal | ✅ Explicite via parents | ❌ Ambigu |
| Historique | ✅ Complet et auditable | ❌ Pas natif |
| Merge automatique | ❌ Complexe à implémenter | ✅ Garanti mathématiquement |
| Offline-first / P2P | ⚠️ Possible mais lourd | ✅ Natif |
| Taille des données | ❌ Croît indéfiniment | ✅ État compact |

**Décision :** DAG pour ce qui a besoin de causalité (messages), CRDT pour ce qui converge naturellement (présence, membres, settings).

---

## 5. Types d'events (kind)

Les `kind` sont encodés sur 16 bits pour laisser de l'espace aux extensions tierces.

### DAG events (immuables)

| Kind | Type | Description |
|---|---|---|
| `0x0001` | message | Message texte/media dans une room |
| `0x0002` | thread reply | Réponse dans un thread |
| `0x0003` | reaction | Réaction emoji à un event |
| `0x0010` | edit | Édition d'un message existant |
| `0x0011` | redact | Suppression d'un message |
| `0x0200` | file ref | Référence vers un fichier chiffré |

### CRDT state events

| Kind | Type | Description |
|---|---|---|
| `0x0100` | room state | Nom, topic, paramètres de la room |
| `0x0101` | presence | Statut en ligne / dernière activité |
| `0x0102` | membership | Membres et rôles d'une room |

### Extensions

| Kind | Type |
|---|---|
| `0xF000+` | Extensions tierces (protocole ouvert) |

---

## 6. Privacy by design

- **Payload toujours chiffré** — les nœuds relais ne voient que des blobs opaques
- **`ts` non-autoritatif** — pas de corrélation temporale fiable depuis le réseau
- **`author` optionnel** — mode anonyme possible
- **Padding uniforme des paquets** — résiste à l'analyse de trafic
- **Sealed sender** — le destinataire ne sait pas qui envoie (optionnel, comme Signal)
- **Forward secrecy** — rotation de clés de session

---

## 7. Stack technique (Rust)

### Transport
- **QUIC / WebTransport** — multiplexing natif, 0-RTT, résistant aux coupures réseau
- **TLS 1.3** minimum
- **Tor** — support optionnel pour les cas haute-sécurité

### Cryptographie
- **Ed25519** — signatures (identité)
- **X25519** — échange de clés (sessions E2EE)
- **ChaCha20-Poly1305** — chiffrement symétrique
- **Blake3** — hachage (IDs des events)

### Crates Rust cibles
- `quinn` — QUIC
- `ed25519-dalek` — signatures
- `x25519-dalek` — échange de clés
- `chacha20poly1305` — chiffrement
- `blake3` — hachage
- `crdts` — types CRDT

---

## 8. Ce qu'on ne fait PAS (scope v1)

- Pas de voix/vidéo (peut venir via extensions `0xF000+`)
- Pas de recherche full-text côté serveur (privacy)
- Pas de modération centralisée
- Pas de compatibilité Matrix (protocol distinct)

---

## Récapitulatif des décisions d'architecture

| Décision | Choix | Raison |
|---|---|---|
| Identité | `@pubkey:domain` (Ed25519) | Portable, sans serveur autoritatif |
| Modèle messages | DAG avec hash Blake3 | Causalité explicite, sans serveur de séquençage |
| Modèle état | CRDT (LWW, Set, Counter) | Convergence automatique, offline-first |
| Transport | QUIC | Multiplexing, 0-RTT, mobile-friendly |
| Chiffrement | ChaCha20-Poly1305 + X25519 | Rapide, sûr, bien supporté en Rust |
| Hachage | Blake3 | Rapide, parallélisable |
| Langage | Rust | Performance, sécurité mémoire, écosystème crypto solide |
| Nom | NeoNet | — |