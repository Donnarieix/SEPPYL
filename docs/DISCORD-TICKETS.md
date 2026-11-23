# Intégration Discord — V1 : webhook sortant uniquement

## Objectif

La V1 de Discord dans SEPPYL utilise **un simple webhook de salon Discord**. Il n'y a **aucune application Discord**, aucun bot, aucune commande slash ni aucune liaison de comptes Discord/SEPPYL.

Le serveur API de SEPPYL publie une notification dans un salon privé réservé au personnel :
- lors de la **création d'un ticket** ;
- à chaque **nouveau message**, y compris les messages composés uniquement d'une image ;
- lorsqu'un ticket est **escaladé vers Maintainer** (passage de non requis à requis).

Le message Discord contient un titre, le sujet du ticket, son identifiant UUID et un lien `https://seppyl.donnarieix.fr/tickets/<UUID>`. **Le contenu des messages et les pièces jointes ne sont pas envoyés sur Discord.** L'ouverture du lien reste soumise à l'authentification et aux permissions de SEPPYL.

**Limite importante :** les webhooks de salon sont unidirectionnels. On ne peut **pas** répondre à un ticket SEPPYL en écrivant sur Discord. Cette fonctionnalité nécessitera une intégration supplémentaire dans une version ultérieure.

## Création du webhook dans Discord

1. Créer ou choisir un salon textuel privé pour l'équipe support, par exemple `#seppyl-support`.
2. Dans Discord, ouvrir les paramètres du salon, puis **Intégrations → Webhooks → Nouveau webhook**. Selon l'interface, la gestion des webhooks est aussi accessible depuis **Paramètres du serveur → Intégrations → Webhooks**.
3. Le nommer `SEPPYL`, choisir le salon de destination et cliquer sur **Copier l'URL du webhook**.
4. Conserver cette URL **secrète** : quiconque la possède peut publier des messages dans le salon. Ne pas la publier dans un chat, ticket, capture d'écran ou commit.

L'URL doit commencer par `https://discord.com/api/webhooks/` (ancien domaine `discordapp.com` également accepté).

## Test du webhook depuis PowerShell (sans toucher à SEPPYL)

Exécuter dans une console PowerShell locale :

```powershell
$webhook = Read-Host "URL du webhook Discord"
$payload = @{
    content = "SEPPYL - test de notification"
    allowed_mentions = @{ parse = @() }
} | ConvertTo-Json -Depth 4
Invoke-RestMethod -Uri $webhook -Method Post -ContentType "application/json" -Body ([Text.Encoding]::UTF8.GetBytes($payload))
Remove-Variable webhook
```

Un message « SEPPYL - test de notification » doit apparaître dans le salon Discord. Le succès HTTP renvoie généralement une réponse vide ; **une console sans erreur est normale**.

## Configuration du backend Rust

Deux variables d'environnement **côté serveur**, dans le processus de l'API SEPPYL :

| Variable | Obligatoire | Description |
| --- | --- | --- |
| `SEPPYL_DISCORD_TICKET_WEBHOOK` | Oui pour activer Discord | URL du webhook copiée depuis Discord |
| `SEPPYL_DISCORD_TICKET_ROLE_ID` | Non | Identifiant du rôle à mentionner uniquement à la création d'un ticket et lors d'une escalade Maintainer |

Si `SEPPYL_DISCORD_TICKET_WEBHOOK` n'est pas renseigné, l'intégration est désactivée silencieusement, et les tickets continuent de fonctionner normalement. Les échecs d'envoi Discord n'interrompent pas les opérations SEPPYL.

Le secret doit être injecté de façon privée dans l'environnement de l'API (pas dans le code client, pas dans Git). Si le déploiement actuel ne permet pas encore d'ajouter cette variable sans accès terminal, cette étape d'intégration au déploiement devra être préparée avant la mise en production. **Ne pas forcer la fusion de cette PR pour tester.**

## Vérification de bout en bout après configuration de l'API

1. Créer un ticket de test depuis un compte SEPPYL autorisé. Vérifier la notification Discord « Nouveau ticket ».
2. Répondre à ce ticket. Vérifier la notification Discord « Nouvelle réponse ».
3. Ajouter une image seule. Vérifier également une notification « Nouvelle réponse ».
4. Cocher « Maintainer », puis appliquer. Vérifier la notification « Intervention Maintainer demandée » et la mention du rôle, si configuré.
5. Changer simplement le statut, sans réactiver Maintainer. Vérifier qu'aucune notification d'escalade supplémentaire ne part.
6. Cliquer sur le lien Discord avec une session SEPPYL non autorisée. L'accès doit être refusé.

**Confidentialité :** le salon doit être réservé aux personnes qui peuvent consulter les tickets. Les identifiants et sujets de tickets sont visibles par les membres du salon ; les contenus privés ne sont pas copiés dans Discord.

La V1 ne permet pas de répondre depuis Discord ni de créer un thread Discord par ticket.
