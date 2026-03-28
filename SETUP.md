# SETUP — Guide utilisateur NeoNet (laptop)

Ce guide t'installe NeoNet sur ton laptop en 5 étapes.

```
Prérequis avant de commencer ce guide :
  ✅ Ton relay tourne sur ton VPS    (neonet.alice.com:7777)
  ✅ Un rendezvous tourne quelque part (<IP_RDV>:7777)
  ✅ Ton pote a aussi son relay up   (neonet.bob.com:7777)

Ce guide installe 2 choses sur ton laptop :
  1. neonet            → le daemon (cerveau, tourne en arrière-plan)
  2. neonet-demo-chat  → l'app de chat (tourne dans un terminal)

⚠️  Le daemon DOIT tourner pour que neonet-demo-chat fonctionne.
    C'est lui le pont entre l'app et le réseau.
```

---

## Étape 1 — Installer le daemon `neonet`

Télécharge le binaire correspondant à ta plateforme.

### macOS ARM (Apple Silicon)

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-aarch64-apple-darwin \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

### Linux x86_64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-x86_64-unknown-linux-musl \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

### Linux ARM64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-aarch64-unknown-linux-musl \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

---

## Étape 2 — Initialiser ton identité

Cette commande génère ta clé Ed25519 et chiffre le keystore sur disque.
**À faire une seule fois.** Ta clé privée ne quitte jamais ton laptop.

```bash
neonet init
```

Tu verras :

```
NeoNet — Initialisation
───────────────────────
Domaine (optionnel, ex: mondomaine.com) : ⏎  (laisser vide si pas de domaine)

Génération de la paire de clés Ed25519...
Keystore créé : /Users/alice/.neonet/keystore/identity.key
Clé publique  : ed25519:ABC123...
Adresse NeoNet: @ABC123...:localhost

Configuration créée : /Users/alice/.neonet/config.toml

Démarrez le daemon avec : neonet start
```

Affiche et partage ton adresse à tes contacts :

```bash
neonet identity show
# → Clé pub : ed25519:ABC123...
# → Adresse : @ABC123...:localhost
```

> Si tu as un relay VPS (`neonet.alice.com`), entre ce domaine lors du `init` :
>
> ```
> Domaine (optionnel, ex: mondomaine.com) : neonet.alice.com
> ```
>
> Ton adresse sera alors `@ABC123...:neonet.alice.com`.

---

## Étape 3 — Démarrer le daemon

Le daemon tourne en arrière-plan. Il gère ta connexion au relay et au réseau.

```bash
NEONET_PASSPHRASE="ta_passphrase" \
neonet start \
  --mode client \
  --relay neonet.alice.com:7777 \
  --daemon
```

Sortie attendue :

```
NeoNet daemon v0.1.0
────────────────────
Mode        : client
Identité    : @ABC123...:neonet.alice.com
API locale  : http://127.0.0.1:7780
Réseau      : 0.0.0.0:7777 (QUIC)
Token API   : écrit dans /Users/alice/.neonet/session.token
Relay configuré : neonet.alice.com:7777

Daemon démarré (PID 84521)
```

Vérifie que tout fonctionne :

```bash
neonet status
# → Daemon en cours
# → Identité : @ABC123...:neonet.alice.com
# → API      : http://127.0.0.1:7780
```

> **Le daemon doit toujours être démarré** avant de lancer `neonet-demo-chat`.
> Si tu redémarres ton laptop, relance cette commande.

---

## Étape 4 — Installer neonet-demo-chat

### macOS ARM

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-demo-chat-aarch64-apple-darwin \
  -o neonet-demo-chat
chmod +x neonet-demo-chat
sudo mv neonet-demo-chat /usr/local/bin/
```

### Linux x86_64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-demo-chat-x86_64-unknown-linux-musl \
  -o neonet-demo-chat
chmod +x neonet-demo-chat
sudo mv neonet-demo-chat /usr/local/bin/
```

### Linux ARM64

```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-demo-chat-aarch64-unknown-linux-musl \
  -o neonet-demo-chat
chmod +x neonet-demo-chat
sudo mv neonet-demo-chat /usr/local/bin/
```

---

## Étape 5 — Chatter avec quelqu'un

### Récupérer l'adresse de ton pote

Ton pote lance sur sa machine :

```bash
neonet identity show
# → Adresse : @XYZ456...:neonet.bob.com  ← il te partage ça
```

### Créer une room (côté Alice)

```bash
neonet-demo-chat --invite @XYZ456...:neonet.bob.com
```

```
Pseudo : Alice
Connexion au daemon sur http://127.0.0.1:7780...
Room créée : a1b2c3d4-...
Partage ce room_id à Bob : a1b2c3d4-...
En attente de Bob...
```

### Rejoindre une room (côté Bob)

Bob utilise le room_id que tu lui as partagé :

```bash
neonet-demo-chat --room a1b2c3d4-...
```

```
Pseudo : Bob
Connexion au daemon...
Connexion à la room a1b2c3d4-...
[Alice] Salut Bob !
```

---

## Étape 6 — Test local sans pote (optionnel)

Pour vérifier que tout fonctionne sans avoir besoin d'un second participant :

```bash
# Terminal 1 — créer la room avec soi-même
neonet-demo-chat --invite @$(neonet identity show --pubkey-only):localhost
# → Note le room_id affiché

# Terminal 2 — rejoindre
neonet-demo-chat --room <room_id>
```

Les deux terminaux communiquent via le daemon local.

---

## Autostart du daemon au login

### macOS (launchd)

```bash
cat > ~/Library/LaunchAgents/me.neonet.daemon.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>me.neonet.daemon</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/neonet</string>
    <string>start</string>
    <string>--mode</string>
    <string>client</string>
    <string>--relay</string>
    <string>neonet.alice.com:7777</string>
    <string>--daemon</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>NEONET_PASSPHRASE</key>
    <string>ta_passphrase</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/me.neonet.daemon.plist
```

### Linux (systemd user)

```bash
mkdir -p ~/.config/systemd/user

cat > ~/.config/systemd/user/neonet.service << 'EOF'
[Unit]
Description=NeoNet Daemon
After=network.target

[Service]
ExecStart=/usr/local/bin/neonet start --mode client --relay neonet.alice.com:7777
Environment=NEONET_PASSPHRASE=ta_passphrase
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

systemctl --user enable neonet
systemctl --user start neonet
systemctl --user status neonet
```

---

## Commandes utiles

```bash
neonet status                       # état du daemon (PID, relay, API)
neonet identity show                # ton adresse NeoNet
neonet rooms list                   # tes rooms actives
neonet peers list                   # pairs connectés
neonet peers connect <addr>:7777    # forcer une connexion à un pair
neonet logs                         # logs du daemon en temps réel
neonet logs --level debug           # logs verbeux
neonet stop                         # arrêter le daemon
```

---

## Dépannage

### `neonet-demo-chat` affiche "daemon non disponible"

Le daemon ne tourne pas. Vérifie et relance :

```bash
neonet status
# → Error: Daemon non joignable sur http://127.0.0.1:7780

NEONET_PASSPHRASE="ta_passphrase" \
neonet start --mode client --relay neonet.alice.com:7777 --daemon
```

### "Relay injoignable" au démarrage du daemon

Vérifie que ton relay VPS tourne :

```bash
# Forcer une connexion test
neonet peers connect neonet.alice.com:7777

# Sur le VPS relay, voir les logs
docker compose logs -f

# Vérifier que le port UDP est ouvert sur le VPS
ufw status | grep 7777
```

### "decryption failed — wrong passphrase?"

Mauvaise passphrase au démarrage. Relance avec la bonne :

```bash
neonet stop
NEONET_PASSPHRASE="bonne_passphrase" \
neonet start --mode client --relay neonet.alice.com:7777 --daemon
```

### "Passphrase required."

La variable `NEONET_PASSPHRASE` est vide ou absente.
Soit tu la passes explicitement :

```bash
NEONET_PASSPHRASE="ta_passphrase" neonet start --mode client --relay ...
```

Soit tu laisses le daemon la demander interactivement (sans `--daemon`) :

```bash
neonet start --mode client --relay neonet.alice.com:7777
# → Passphrase : [tu tapes]
```

### Réinitialiser complètement

⚠️ **Cette opération supprime ton identité de façon irréversible.**
Tu perdras toutes tes rooms et ton adresse NeoNet.

```bash
neonet stop
rm -rf ~/.neonet
neonet init
```

Tu auras une nouvelle clé Ed25519 et une nouvelle adresse NeoNet.
