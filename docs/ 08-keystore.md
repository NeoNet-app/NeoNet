# NeoNet — Keystore

---

## 1. Objectifs

- Stocker la clé privée Ed25519 chiffrée sur disque
- Ne jamais exposer la clé privée en dehors du daemon
- Dériver la clé de chiffrement depuis une passphrase via Argon2id
- Supporter le déverrouillage au démarrage (interactif ou via env)

---

## 2. Structure des fichiers

```
~/.neonet/
├── keystore/
│   ├── identity.key       # clé privée chiffrée (chmod 600)
│   └── identity.pub       # clé publique en clair (chmod 644)
├── session.token          # token API (chmod 600, régénéré au démarrage)
├── config.toml            # configuration du daemon
└── neonet.db              # SQLite
```

---

## 3. Format de `identity.key`

Fichier TOML chiffré, structure avant chiffrement :

```toml
[meta]
version    = 1
created_at = 2025-01-01T00:00:00Z
pubkey     = "ed25519:BASE64URL"      # copie de la clé publique (vérification)

[kdf]
algorithm  = "argon2id"
salt       = "BASE64URL"              # 32 bytes aléatoires
m_cost     = 65536                    # 64 MB mémoire
t_cost     = 3                        # 3 itérations
p_cost     = 1                        # 1 thread

[cipher]
algorithm  = "chacha20poly1305"
nonce      = "BASE64URL"              # 12 bytes aléatoires
ciphertext = "BASE64URL"             # clé privée Ed25519 (32 bytes) chiffrée
tag        = "BASE64URL"              # tag Poly1305 (16 bytes)
```

---

## 4. Procédure de création du keystore

```
1. Demander la passphrase à l'utilisateur (double saisie pour confirmation)
2. Générer une paire de clés Ed25519
3. Générer un salt aléatoire (32 bytes)
4. Dériver une clé de chiffrement :
     enc_key = Argon2id(passphrase, salt, m=65536, t=3, p=1) → 32 bytes
5. Générer un nonce aléatoire (12 bytes)
6. Chiffrer la clé privée :
     ciphertext = ChaCha20-Poly1305(enc_key, nonce, privkey)
7. Écrire identity.key (chmod 600)
8. Écrire identity.pub (chmod 644) — clé publique en clair
```

---

## 5. Procédure de déverrouillage

```
1. Lire identity.key
2. Obtenir la passphrase (voir §6)
3. Recalculer enc_key = Argon2id(passphrase, salt, params du fichier)
4. Déchiffrer :
     privkey = ChaCha20-Poly1305-Decrypt(enc_key, nonce, ciphertext, tag)
5. Vérifier : Ed25519::from_privkey(privkey).pubkey == meta.pubkey
     → Échec : passphrase incorrecte, arrêter
6. Garder privkey en mémoire uniquement (jamais réécrire sur disque)
7. Zeroize enc_key après usage (crate `zeroize`)
```

---

## 6. Sources de la passphrase

Par ordre de priorité :

```
1. Variable d'environnement NEONET_PASSPHRASE   (CI, scripts)
2. Argument CLI --passphrase <value>             (déconseillé, visible dans ps)
3. Saisie interactive (stdin, pas d'écho)        (défaut)
```

En production, privilégier la variable d'environnement injectée par le système (systemd `EnvironmentFile`, Docker secrets, etc.).

---

## 7. Sécurité mémoire

```rust
use zeroize::Zeroizing;

// La clé privée est toujours dans un Zeroizing<Vec<u8>>
// → zéroïsée automatiquement quand le wrapper est droppé
let privkey: Zeroizing<[u8; 32]> = Zeroizing::new(decrypted_privkey);
```

- La clé privée ne doit jamais être copiée dans une structure sans `Zeroizing`
- Elle ne doit jamais apparaître dans les logs
- Elle ne doit jamais être sérialisée ou envoyée sur un channel

---

## 8. Opérations exposées par le keystore

Le keystore expose uniquement des opérations de haut niveau — jamais la clé :

```rust
trait Keystore {
    fn pubkey(&self) -> [u8; 32];
    fn sign(&self, payload: &[u8]) -> [u8; 64];
    fn address(&self) -> String;     // "@pubkey_base64url:domain"
}
```

Le daemon utilise ces méthodes pour signer les DAG events, les CRDT ops, et les messages de handshake.

---

## 9. Première utilisation — init

Si aucun keystore n'existe au démarrage :

```
$ neonet start
→ Aucun keystore trouvé dans ~/.neonet/keystore/
→ Lancement de neonet init pour créer votre identité...
  (ou passer --init pour créer automatiquement)
```

---

## 10. Crates Rust

| Crate | Usage |
|---|---|
| `argon2` | Dérivation de clé (Argon2id) |
| `chacha20poly1305` | Chiffrement de la clé privée |
| `ed25519-dalek` | Génération et gestion de la paire de clés |
| `zeroize` | Zéroïsation mémoire des secrets |
| `rand` | Génération salt, nonce |
| `rpassword` | Saisie passphrase sans écho terminal |
| `toml` + `serde` | Lecture/écriture identity.key |

---

## Récapitulatif des décisions

| Décision | Choix |
|---|---|
| Format stockage | TOML chiffré, chmod 600 |
| KDF | Argon2id (m=64MB, t=3, p=1) |
| Chiffrement | ChaCha20-Poly1305 |
| Source passphrase | Env > CLI > stdin interactif |
| Clé en mémoire | `Zeroizing<[u8; 32]>` |
| API exposée | sign(), pubkey() — jamais la clé brute |
| Init automatique | Proposé au premier démarrage |