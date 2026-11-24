# SEPPYL / Discord — assistance interactive (V2)

## Développement local : bot Python sans tunnel

Pendant la validation sur ton PC, Discord peut transmettre les commandes et les
boutons au **bot Python connecté au Gateway** (WebSocket sortant). L'API Rust
reste sur \`127.0.0.1:3000\` et n'est pas publiée sur Internet.

**Important : dans Discord Developer Portal → General Information, laisser
« Interactions Endpoint URL » VIDE.** Une application reçoit ses interactions
**soit** via le Gateway, **soit** via un endpoint HTTP. Si un endpoint est
configuré, Discord ne transmettra pas les interactions au bot local.

### 1. Préparer l'application et les variables locales

- Créer une application Discord, activer son bot et l'installer sur un **serveur
  privé de test** avec les scopes \`bot\` et \`applications.commands\`. Le bot doit
  pouvoir voir le canal de test et envoyer des messages, mais aucun intent
  privilégié (« Message Content », « Server Members ») n'est nécessaire.
- Copier l'Application ID, le Guild ID, le Channel ID et le Bot Token. **Ne
  partager ni le token ni l'URL complète du webhook.**
- Depuis PowerShell, générer une clé locale à usage interne :
  \`python -c "import secrets; print(secrets.token_urlsafe(48))"\`
- Insérer ces valeurs dans le \`.env\` **privé à la racine du dépôt** :

\`\`\`dotenv
SEPPYL_BIND=127.0.0.1:3000
SEPPYL_DISCORD_APPLICATION_ID=APPLICATION_ID
SEPPYL_DISCORD_GUILD_ID=GUILD_ID
SEPPYL_DISCORD_BOT_TOKEN=BOT_TOKEN
SEPPYL_DISCORD_CHANNEL_ID=CHANNEL_ID
SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN=CLE_ALEATOIRE_D_AU_MOINS_32_CARACTERES
# Facultatif (port local par défaut : 3000)
SEPPYL_DISCORD_LOCAL_API=http://127.0.0.1:3000
\`\`\`

Pour le mode Gateway, la clé publique Discord (\`SEPPYL_DISCORD_PUBLIC_KEY\`)
**n'est pas nécessaire** : elle sert à vérifier les callbacks HTTP en
production. Le secret local est différent du JWT_SECRET et ne doit jamais
être committé.

### 2. Lancer les trois composants

Depuis la racine du projet, dans des terminaux séparés :

1. **API Rust :** \`cargo run -p api\`
2. **Client Dioxus :** commande habituelle de développement \`dx serve\` dans
   le répertoire client (l'API doit être joignable sur le port indiqué dans
   la configuration du client).
3. **Bot local Python :**

\`\`\`powershell
python -m pip install -r scripts/discord-requirements.txt
python scripts/discord_local_bot.py
\`\`\`

Le bot lit automatiquement le \`.env\` racine. Il vérifie l'API au démarrage,
ouvre une connexion sortante vers Discord et **synchronise lui-même**
\`/seppyl\` dans le serveur configuré : ne pas lancer
\`scripts/register-discord-commands.py\` pour ce mode.

Si le bot indique que la passerelle est inaccessible, vérifier que l'API
est démarrée avec exactement \`SEPPYL_BIND=127.0.0.1:3000\` et le même secret.
Une API liée à \`0.0.0.0\` désactive volontairement la passerelle locale.

### 3. Tester

- Connecter un administrateur/maintainer au client Dioxus local.
- Depuis **Mon compte → Discord**, générer un code de liaison à usage unique.
- Dans Discord : \`/seppyl lier\` (code), \`/seppyl liste\`,
  \`/seppyl voir\`, \`/seppyl repondre\`, \`/seppyl statut\` et
  \`/seppyl maintainer\`.
- Créer un ticket depuis le client. Le bot envoie une notification dans le
  canal configuré avec **Voir, Répondre, Statut, Maintainer**. Essayer aussi
  les boutons et le formulaire de réponse.
- Vérifier les changements dans l'interface locale. Relancer le bot et
  tester un **ancien** message : son bouton doit continuer de fonctionner.
- Délier le compte et vérifier qu'aucune action de modification ne passe.

Les réponses Discord sont liées à l'utilisateur SEPPYL, avec contrôle du rôle
à chaque opération. Aucune requête publique ne peut appeler l'adaptateur
local sans la clé secrète et une connexion en boucle locale.

### 4. Passer en production

Arrêter le bot Python local. **Ne pas** déployer
\`SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN\` ou ce mode Gateway sur le serveur.
Utiliser à la place le callback HTTPS signé
\`/api/discord/interactions\` avec la Public Key Discord et une URL publique.
Il faut alors configurer « Interactions Endpoint URL » sur Discord. Ne pas
utiliser simultanément les deux modes pour **la même application**.

---

## Architecture

L'API Rust SEPPYL sert elle-même les interactions Discord, sans processus Gateway séparé. La connexion exige une application Discord (contrairement au webhook V1).

**Actions disponibles dans Discord :**
- `/seppyl lier` et `/seppyl delier` : liaison volontaire et révocable d'un compte staff SEPPYL.
- `/seppyl liste` : dix derniers tickets ouverts ou en cours, avec lien direct.
- `/seppyl voir` : statut, escalade, cinq derniers messages (réponse éphémère privée).
- `/seppyl repondre` : répondre au ticket depuis Discord.
- `/seppyl statut` : ouvert, en cours, résolu, fermé.
- `/seppyl maintainer` : activer/retirer l'intervention Maintainer.

Lorsque `SEPPYL_DISCORD_BOT_TOKEN` **et** `SEPPYL_DISCORD_CHANNEL_ID` sont configurés, les notifications sont publiées par le bot avec quatre boutons : Voir, Répondre (ouvre un modal), Statut (sélection) et Maintainer (bascule). Les anciens webhooks restent un fallback si le bot n'est pas configuré ou en cas d'erreur d'envoi. **Les contenus privés ne figurent pas dans les annonces de salon**, mais les sujets des tickets restent visibles dans le salon staff.

Les réponses et mutations Discord modifient directement la SQLite SEPPYL, en conservant l'identité du compte SEPPYL lié et les événements temps réel de l'interface web. Les réponses Discord ne déclenchent pas de notification sortante vers Discord (pas de boucle). Les lectures /seppyl voir enregistrent les accusés de lecture habituels.

## Permissions et sûreté

Les appels de Discord doivent porter une signature Ed25519 valide, avec horodatage récent, identifiant d'application et de serveur concordants. Toute opération est attribuée à un compte staff SEPPYL lié. La permission est **recontrôlée dans SQLite à chaque interaction**, même si le compte est rétrogradé ou désactivé depuis sa liaison.

La liaison utilise un code UUID aléatoire valable **10 minutes**, à usage unique, généré côté SEPPYL. Les données de liaison sont privées. Les mutations Discord ont des identifiants d'interactions uniques enregistrés en BDD afin d'empêcher les doublons. Les commandes renvoient des réponses *éphémères* (visibles par leur auteur uniquement) et désactivent les mentions automatiques.

L'API n'expose jamais le jeton du bot, les secrets de webhook ni les JWT à Discord ou au navigateur. Les tables sont ajoutées sans destruction ni modification des tickets historiques. La suppression d'un compte SEPPYL révoque sa liaison.

## Préparer Discord

1. Depuis https://discord.com/developers/applications créer une application, par exemple « SEPPYL ».
2. Dans « General Information », copier **Application ID** et **Public Key**.
3. Ouvrir « Bot » et générer/copier le **Token** (secret). Ne pas le coller dans Git.
4. Installer l'application sur le serveur Discord cible avec scopes `bot` et `applications.commands` et permissions minimales : voir le salon staff et y envoyer des messages. Garder le salon privé.
5. Relever le serveur Discord (`Guild ID`) et l'identifiant du salon staff (`Channel ID`) depuis le mode développeur.

## Configuration serveur API SEPPYL

À ajouter au `.env` privé utilisé par l'API (et **non** au .env.example avec des valeurs réelles) :

```dotenv
SEPPYL_DISCORD_APPLICATION_ID=APPLICATION_ID
SEPPYL_DISCORD_PUBLIC_KEY=PUBLIC_KEY_HEX
SEPPYL_DISCORD_GUILD_ID=GUILD_ID
SEPPYL_DISCORD_BOT_TOKEN=BOT_TOKEN
SEPPYL_DISCORD_CHANNEL_ID=CHANNEL_ID
# Fallback optionnel, conserver le webhook V1 existant.
SEPPYL_DISCORD_TICKET_WEBHOOK=https://discord.com/api/webhooks/...
# Rôle à notifier facultatif
# SEPPYL_DISCORD_TICKET_ROLE_ID=ROLE_ID
```

Le bot token et le webhook sont des **secrets** ; ne pas les partager dans une capture d'écran ni dans le chat. Après mise à jour de l'API et du `.env`, redémarrer l'API. Les variables APP/PUBLIC/GUILD activent le flux de liaison et la vérification des interactions. Le bot token et channel ID activent les notifications interactives.

Une fois l'API disponible en HTTPS, renseigner dans « General Information » → « Interactions Endpoint URL » :

`https://seppyl.donnarieix.fr/api/discord/interactions`

Discord effectue une validation PING. Une route non déployée ou un reverse proxy qui bloque POST /api/discord/interactions fera échouer cette étape.

## Enregistrer les commandes Discord

Depuis un poste disposant du dépôt et de Python 3, avec les trois secrets/IDs ci-dessous dans l'environnement de la console :

```powershell
$env:DISCORD_APPLICATION_ID = "APPLICATION_ID"
$env:SEPPYL_DISCORD_GUILD_ID = "GUILD_ID"
$env:DISCORD_BOT_TOKEN = "BOT_TOKEN"
python scripts/register-discord-commands.py
Remove-Item Env:DISCORD_BOT_TOKEN
```

Cette opération enregistre la commande `/seppyl` **uniquement sur le serveur ciblé**. Elle ne remplace pas les autres commandes du serveur.

## Liaison et premier essai

1. Connectez-vous à SEPPYL avec un compte Administrateur ou Maintainer.
2. Ouvrez **Mon compte → Discord** puis **Générer un code de liaison**.
3. Dans le salon privé Discord, tapez `/seppyl lier` avec ce code (10 minutes, une seule utilisation).
4. Exécutez `/seppyl liste`, puis `/seppyl voir` avec l'UUID d'un ticket de test.
5. Répondez depuis `/seppyl repondre` ou depuis le bouton « Répondre » dans une notification bot.
6. Vérifiez sur SEPPYL que le message porte votre compte et que le demandeur le voit.
7. Testez « Statut » et « Maintainer » depuis le message Discord, puis vérifiez la synchronisation dans SEPPYL.
8. Testez la déliaison, la rétrogradation de rôle et une requête non signée. Les mutations ne doivent alors plus être possibles.

## Limites connues

- Les commandes et les boutons permettent de **traiter l'ensemble du cycle de vie du ticket** (lire, répondre, ouvrir, commencer, résoudre, fermer, escalader), mais ne permettent pas de rédiger des réponses simplement en écrivant dans un salon/thread comme dans une messagerie. Cela nécessiterait un bot Gateway et une synchronisation de threads.
- Les uploads Discord et la gestion de l'historique des révisions de messages restent accessibles depuis SEPPYL Web.
- Sur une branche non déployée, on peut vérifier compilation/tests, mais **pas valider le PING Discord** sans URL HTTPS publique pointant vers cette branche.
- L'activation en production ne sera réalisée qu'après validation de la PR ; aucune configuration secrète ne doit être committée.
