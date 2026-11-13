# Support Discord — architecture et limites

## Fonctionnel aujourd'hui
- SEPPYL est la source de vérité : tickets et messages sont stockés dans SQLite, avec contrôle des droits par l'API.
- Un webhook Discord sortant facultatif notifie la création, les nouveaux messages et les escalades Maintainer.
- Configuration serveur uniquement via `SEPPYL_DISCORD_TICKET_WEBHOOK` et, éventuellement, `SEPPYL_DISCORD_TICKET_ROLE_ID`.
- Ne jamais injecter l'URL Discord dans le navigateur, les logs ou l'historique Git.
- Le polling des fiches de ticket est de 15 s et ne nécessite pas de service WebSocket additionnel.

## Réponses Discord → SEPPYL
**Non implémenté** : un webhook Discord ne peut pas écouter les messages entrants. Pour un canal bidirectionnel, il faut un **bot Discord** ou une application Discord avec interactions. Proposition :

1. Un canal privé (ou un thread privé) par ticket. Table de liaison `ticket_id` ↔ `channel_id/thread_id`; ne pas utiliser les noms comme identifiants.
2. Le bot authentifie chaque réponse via l'identité Discord, vérifie l'autorisation d'un administrateur/Maintainer et crée le message via un point d'entrée interne.
3. Déduplication sur `discord_message_id`, protections anti-boucle et prise en charge des réponses sur SEPPYL publiées vers Discord.
4. Filtrer Markdown et mentions autorisées; bloquer `@everyone`, `@here` et les liens d'accès non validés. Échapper le contenu et conserver les messages originaux.
5. Utiliser un secret serveur (jamais une clé en front), permissions Discord minimales, journaux d'audit et stratégie de réessai pour les erreurs de réseau.
6. Prévoir la révocation des liens et permissions quand un ticket est fermé ou qu'un membre du staff perd ses droits.

Le bot ne doit jamais contourner les règles de confidentialité de l'API. Une réponse postée par Discord doit indiquer l'identité de l'administrateur et être visible dans la même conversation SEPPYL.
