# Tickets SEPPYL — intégration Discord

## Périmètre

SEPPYL reste la source de vérité : messages, autorisations et historiques sont enregistrés dans SQLite. Discord sert à notifier le personnel et à envoyer des réponses depuis un serveur Discord **privé**. Les utilisateurs finaux continuent d'accéder à leurs tickets depuis SEPPYL.

Cette première implémentation comprend :

- **SEPPYL → Discord** : notifications par webhook pour création de ticket, nouvelle réponse et escalade vers Maintainer. Les messages ne sont **pas** recopiés en entier sur Discord. Le webhook envoie le titre du ticket et un lien vers sa conversation authentifiée. Mention facultative du rôle Discord uniquement à la création et à l'escalade.
- **Discord → SEPPYL** : commande slash `/seppyl lier` pour associer une identité Discord à un compte Administrateur/Maintainer ; commande `/seppyl repondre` pour écrire un message Markdown dans la conversation SEPPYL.
- **Mon compte > Discord** : génération d'un code de liaison à usage unique valable 10 minutes, statut de liaison, déliaison.
- Les commandes renvoient des réponses **éphémères**, visibles seulement par leur auteur. Les messages envoyés depuis Discord ne rebouclent pas vers le webhook.
- Signatures Discord Ed25519 obligatoires, contrôle de l'identifiant du serveur Discord, vérification du rôle **actuel** dans SEPPYL, comptes désactivés refusés, déduplication par identifiant d'interaction.

Cette étape ne crée pas encore un canal/thread par ticket et ne synchronise pas automatiquement les conversations entières avec Discord. Les réponses depuis Discord se font par commande slash en donnant l'UUID du ticket. Pas besoin de bot Gateway ni de Message Content Intent pour ces commandes.

## Configuration côté Discord

1. Créer une application depuis le [Discord Developer Portal](https://discord.com/developers/applications). Copier sa **Public Key** depuis l'onglet General Information.
2. Activer le bot de l'application et l'installer sur le serveur voulu, avec le scope `applications.commands` (et si besoin `bot`). Conserver le jeton du bot dans les secrets de l'opérateur, jamais dans le dépôt ni dans le navigateur.
3. Configurer l'URL des **Interactions Endpoint** sur `https://seppyl.donnarieix.fr/api/discord/interactions` **après avoir déployé l'API**. Discord enverra une requête PING signée pour vérifier la réponse. Cette route n'utilise pas le JWT SEPPYL, mais refuse toute requête non signée.
4. Dans un canal Discord privé réservé au staff, créer un webhook (Paramètres du salon → Intégrations → Webhooks). Garder son URL secrète.
5. Relever l'ID du serveur Discord et, éventuellement, celui du rôle à mentionner (mode développeur Discord).

## Configuration côté serveur SEPPYL

Configurer les variables d'environnement du **service API Rust** :

| Variable | Description |
| --- | --- |
| `SEPPYL_DISCORD_PUBLIC_KEY` | Clé publique de l'application Discord, hexadécimale (obligatoire pour les commandes entrantes) |
| `SEPPYL_DISCORD_GUILD_ID` | ID du seul serveur Discord autorisé à utiliser les commandes (obligatoire pour les commandes entrantes) |
| `SEPPYL_DISCORD_TICKET_WEBHOOK` | URL complète du webhook Discord HTTPS, canal staff privé (facultatif pour les notifications) |
| `SEPPYL_DISCORD_TICKET_ROLE_ID` | ID du rôle Discord à mentionner à la création/escalade (facultatif) |

La **clé publique** n'est pas un secret. En revanche, le webhook et le jeton du bot sont secrets. Le jeton de bot n'est **pas** nécessaire à l'exécution de l'API SEPPYL : il sert uniquement à enregistrer la commande auprès de Discord.

Après mise en place de la configuration et déploiement, enregistrer une fois la commande depuis l'environnement d'administration avec :

```bash
export DISCORD_BOT_TOKEN='...'
export DISCORD_APPLICATION_ID='...'
export SEPPYL_DISCORD_GUILD_ID='...'
python3 scripts/register-discord-commands.py
```

Le script crée/met à jour la seule commande slash `seppyl` dans le serveur ciblé, sans écraser les autres commandes. Ne jamais publier le jeton ni le mettre dans un fichier versionné.

## Utilisation

1. Se connecter au site SEPPYL avec un compte **Administrateur** ou **Maintainer**.
2. Dans `Mon compte > Discord`, cliquer sur **Générer un code de liaison**.
3. Sur le serveur Discord configuré, saisir `/seppyl lier` et y coller le code. Le code expire après dix minutes et n'est utilisable qu'une fois. Une nouvelle liaison remplace l'ancienne.
4. Lorsqu'un ticket est signalé dans le canal staff, ouvrir son lien ou copier son UUID indiqué dans l'embed. Pour répondre depuis Discord, saisir `/seppyl repondre` puis les paramètres `ticket` (UUID) et `message` (texte Markdown).
5. La réponse apparaît dans SEPPYL avec **l'identité du compte SEPPYL lié**. Un lien désactivé, un compte désactivé, ou un utilisateur n'ayant plus les droits Admin/Maintainer ne peut pas poster.
6. Le bouton **Délier Discord** retire immédiatement la liaison. Il révoque l'accès aux futures réponses depuis Discord.

## Protection des données et limites

- Utiliser uniquement un serveur et un salon Discord **privés au personnel autorisé**. Le titre du ticket et son identifiant sont visibles dans le salon où pointe le webhook. Les pièces jointes et le contenu des messages ne sont pas inclus dans le webhook.
- Ne jamais afficher ni enregistrer l'URL du webhook dans des logs, payloads front ou historiques Git. L'accès au ticket via le lien Discord requiert toujours une session SEPPYL autorisée.
- Les messages entrants sont associés à la personne ayant lié son compte, et non à un compte bot partagé. L'auteur ne peut pas être choisi par une option de commande.
- Discord limite la longueur des options slash ; la commande `repondre` est configurée à 2 000 caractères. Les réponses plus longues doivent être saisies depuis SEPPYL.
- Les erreurs du webhook ne bloquent pas les opérations SEPPYL ; l'envoi reste best-effort. Un mécanisme de file persistante et des threads Discord par ticket pourront venir dans une passe suivante.
- Pour éviter toute publication accidentelle, l'intégration **n'est pas déployée automatiquement** : branche de développement et PR à valider avant fusion sur `main`.

## Vérifications

- Faire répondre Discord au PING de validation de l'Interactions Endpoint.
- Confirmer qu'un compte non Admin/Maintainer reçoit HTTP 403 sur les endpoints de liaison.
- Vérifier qu'une requête Discord sans signature valide reçoit HTTP 401 (si l'intégration est configurée).
- Tester la liaison d'un maintainer, une réponse depuis Discord, puis sa visibilité dans SEPPYL et dans les non-lus du demandeur.
- Vérifier qu'une répétition de la même interaction ne duplique pas le message.
- Vérifier qu'un compte désactivé ou rétrogradé ne peut plus répondre.
- Vérifier que le webhook de nouvelle réponse ne boucle pas pour une réponse venue de Discord.
