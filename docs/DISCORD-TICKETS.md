# SEPPYL / Discord — assistance interactive (V2)

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
