### Prérequis
- macOS ARM ou Linux x86_64 / ARM64
- Le relay de ton domaine est déjà up (neonet.alice.com:7777)
- Le rendezvous est déjà up (<IP_RDV>:7777)

### 1. Installer NeoNet

#### macOS ARM
```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-aarch64-apple-darwin \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

#### Linux x86_64
```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-x86_64-unknown-linux-musl \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

#### Linux ARM64
```bash
curl -L https://github.com/NeoNet-app/NeoNet/releases/latest/download/neonet-aarch64-unknown-linux-musl \
  -o neonet
chmod +x neonet
sudo mv neonet /usr/local/bin/
neonet --version
```

### 2. Initialiser son identité

```bash
neonet init
# → Domaine (optionnel) : laisser vide si pas de domaine perso
# → Passphrase : choisir une passphrase solide
# → Clé publique affichée → c'est ton adresse NeoNet

# Vérifier
neonet identity show
# → @ed25519ABC123:localhost  (ou ton domaine si renseigné)
```

⚠️ Note ta clé publique — c'est ce que tu partages à tes contacts.

### 3. Démarrer le daemon en mode client

```bash
neonet start \
  --mode client \
  --relay neonet.alice.com:7777 \
  --daemon

# Vérifier
neonet status
# → Daemon : en cours
# → Connecté au relay : neonet.alice.com:7777
```

### 4. Installer NeoNet Demo Chat

download: [github.com/NeoNet-app/NeoNet-demo-app/releases](https://github.com/NeoNet-app/NeoNet-demo-app/releases)

```bash
chmod +x neonet-demo-app
sudo mv neonet-demo-app /usr/local/bin/
```

### 5. Démarrer une conversation

#### Tu invites quelqu'un (tu crées la room)
```bash
neonet-demo-app --invite @PUBKEY_DE_TON_POTE:neonet.bob.com
# → Pseudo : Alice
# → Room créée : <room_id>
# → Partage ce room_id à ton pote
```

#### Tu rejoins une room existante
```bash
neonet-demo-app --room <room_id>
# → Pseudo : Alice
# → Connecté à la room
```

#### Test rapide sur la même machine (deux terminaux)
```bash
# Terminal 1
neonet-demo-app --tmpconf --invite @TA_PROPRE_PUBKEY:localhost
# → note le room_id affiché

# Terminal 2
neonet-demo-app --tmpconf --room <room_id>
```

### 6. Trouver l'adresse NeoNet de ton pote

Ton pote lance sur sa machine :
```bash
neonet identity show
# → @ed25519XYZ:neonet.bob.com
```

Il te partage cette adresse (Signal, mail, IRL, peu importe).
Tu l'utilises pour l'inviter :
```bash
neonet-demo-app --invite @ed25519XYZ:neonet.bob.com
```

### 7. Commandes utiles au quotidien

```bash
# Statut du daemon
neonet status

# Voir ses rooms
neonet rooms list

# Voir les pairs connectés
neonet peers list

# Arrêter le daemon
neonet stop

# Relancer le daemon
neonet start --mode client --relay neonet.alice.com:7777 --daemon

# Voir les logs
neonet logs
neonet logs --level debug
```

### 8. Autostart au login (optionnel)

#### macOS — launchd
```bash
cat > ~/Library/LaunchAgents/com.neonet.daemon.plist << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.neonet.daemon</string>
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
    <string>TA_PASSPHRASE</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/com.neonet.daemon.plist
```

#### Linux — systemd
```bash
cat > ~/.config/systemd/user/neonet.service << EOF
[Unit]
Description=NeoNet daemon
After=network.target

[Service]
ExecStart=/usr/local/bin/neonet start --mode client \
          --relay neonet.alice.com:7777 --daemon
Environment=NEONET_PASSPHRASE=TA_PASSPHRASE
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

systemctl --user enable neonet
systemctl --user start neonet
```

### Schéma récap

```
Ton laptop                VPS (ton relay)          VPS (rendezvous)
┌──────────────┐          ┌─────────────────┐      ┌──────────────┐
│ neonet-demo  │          │  neonet-relay   │      │  neonet-rdv  │
│    -chat     │──QUIC───►│  docker compose │◄────►│docker compose│
│              │          │  :7777          │      │  :7777       │
│ neonet start │          └─────────────────┘      └──────────────┘
│ --mode client│                  ▲
│ --relay ...  │                  │ QUIC chiffré
└──────────────┘                  │
                          ┌─────────────────┐
                          │  neonet-relay   │
                          │  (relay Bob)    │
                          │  :7777          │
                          └─────────────────┘
                                  ▲
                                  │
                          ┌──────────────┐
                          │ Laptop Bob   │
                          │ neonet client│
                          └──────────────┘
```

### Dépannage

#### Le daemon ne démarre pas
```bash
neonet logs --level debug
# Vérifier que le relay est joignable :
neonet peers ping neonet.alice.com:7777
```

#### Pas de connexion au relay
```bash
# Vérifier que le port 7777 UDP est ouvert sur le VPS
# Vérifier que le relay tourne :
ssh user@vps "docker compose -f docker/relay/docker-compose.yml logs"
```

#### Mauvaise passphrase
```bash
# Réinitialiser complètement (perd l'identité actuelle) :
rm -rf ~/.neonet
neonet init
```

#### Voir la version installée
```bash
neonet --version
neonet-demo-app --version
```