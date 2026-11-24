# SEPPYL — 6ᵉ passe, conception et exploitation

Cette branche est **exclusivement une feature**, non déployable avant revue de la PR et réussite de la CI complète. La version publiée reste v0.1.1 jusqu'à la préparation d'une nouvelle release.

## Confidentialité de la partie publique

Trois niveaux opt-in indépendants sont gérés par club :
1. publication du club (nom, logo, description) ;
2. publication de ses équipes (nom, logo, description) ;
3. publication explicite des effectifs (identité et numéro des joueuses).

Le second niveau exige le premier, le troisième exige les deux premiers. Ils sont tous désactivés par défaut sur les anciens clubs (NULL). Seuls les managers du club ou les rôles administrateur/maintainer peuvent les activer. Les notes, entraînements, chronologies de matchs, tickets et statistiques individuelles ne sont jamais exposés par des routes publiques. Les activités restent privées ; leur publication pourrait être envisagée plus tard via un filtre/système de consentement, jamais via l'intégralité du catalogue.

## Matchs

Un match appartient à une **équipe**, donc indirectement à un club. La lecture suit les autorisations d'équipe ; création, mise à jour, composition, événements et création de types sont autorisées aux entraîneurs de l'équipe, gestionnaires du club, administrateurs et maintainer.

Un match possède adversaire, date, minutes par mi-temps (de 1 à 120), notes, participantes, postes, et statut de titulaire. Les événements sont horodatés en **minute absolue du match** sur 2 mi-temps : une entrée à la minute 50 pour une mi-temps de 45 signifie 5 minutes après la reprise. Types intégrés : but, changement, carton jaune, carton rouge, blessure et autre. Les types personnalisés sont rattachés au club, et un événement ne peut référencer un type appartenant à un autre club.

Le calcul des minutes part des titulaires sur le terrain et suit les remplacements dans l'ordre chronologique. Les changements invalides (joueuse sortante absente du terrain, entrante déjà sur le terrain, minute hors période, joueuse hors composition) sont rejetés. Le calcul **ne modifie pas** directement une colonne « minutes cumulées » ; les statistiques sont recalculées à partir de la source des événements et de la composition.

## Durées, remarques et statistiques

Chaque entraînement dispose d'une durée optionnelle en minutes (1–1440). Les anciennes séances restent à durée non renseignée et ne sont pas artificiellement comptées. Un total de temps d'entraînement est la somme des durées de séances où la joueuse est présente, pour les équipes auxquelles le compte a accès. Le total de temps de match est la somme des minutes calculées sur ces matchs. Les notes générales et par activité peuvent porter des « informations complémentaires » optionnelles (4 000 caractères maximum).

## Tickets

Tous les comptes connectés (y compris rôle User) peuvent créer un ticket, lire **uniquement leurs tickets** et répondre. Les administrateurs et maintainer ont accès à l'ensemble des tickets, peuvent changer le statut (ouvert, en cours, résolu, fermé) et ajouter/retirer la mention « Maintainer ». Un ticket fermé n'accepte plus de réponse du demandeur ; le personnel peut intervenir. Les messages sont stockés dans une table séparée.

La messagerie fonctionne avec **polling toutes les 15 secondes** sur une fiche de ticket ouverte ; pas de WebSocket ni d'état temps réel permanent à maintenir sur l'infrastructure actuelle.

### Discord

La V1 webhook sortant reste disponible sans configuration supplémentaire. La nouvelle
intégration interactive ajoute une **application Discord** (commande `/seppyl`,
boutons « Voir / Répondre / Statut / Maintainer » sur les messages du bot) avec
un endpoint d'interactions signé Ed25519. La consultation et les réponses
interactives sont privées au membre connecté dans Discord.

L'identité est liée volontairement à un compte staff SEPPYL au moyen d'un code
temporaire à usage unique et contrôlée à chaque interaction : comptes désactivés,
rétrogradés ou déliés n'ont plus accès à la gestion. Les accusés de lecture et
l'escalade Maintainer restent individuels. Les réponses créées depuis Discord
ne bouclent pas vers le canal Discord, et les opérations sont dédupliquées
et consignées dans l'audit.

Les variables d'application/bot sont uniquement présentes sur le serveur.
Les notifications webhook sont conservées en secours et ne contiennent jamais
les messages privés. Voir `docs/DISCORD-TICKETS.md`.

### HTTP 404

La page 404 Dioxus est opérationnelle. Un HTML statique `404.html` et une
proposition Nginx sont disponibles pour obtenir **un vrai HTTP 404**, mais le
proxy de production ne doit pas être modifié automatiquement sans ses règles
réelles (API, certificats, SPA). Sans cette modification contrôlée, SEPPYL
conserve volontairement HTTP 200 et sa page 404 personnalisée.

## Annonces

Les annonces ne sont pas des changelogs. Elles comportent publication, début et fin de validité en timestamps Unix et un drapeau urgent. Seuls les administrateurs et maintainer peuvent les publier ou supprimer. Le bandeau global affiche exclusivement les annonces urgentes, publiées et actives. Le bouton « Nouveautés » ouvre l'historique des versions, jamais une liste d'annonces. Les annonces publiées et actives sont uniquement diffusées par le bandeau global. Leur gestion est réservée aux Maintainers depuis l'onglet Administration.

## CI et livraison

`.github/workflows/deploy.yml` s'exécute sur les pushes `main`, `feat/**`, `release/**` et sur les PR vers `main`. Les jobs compilations API/Web, vérifications Rust/JS/migrations et E2E Chromium sont présents dans **tous ces cas**. Les jobs SSH production et GitHub Release portent un garde explicite `github.ref == 'refs/heads/main'`, et ne sont donc jamais exécutés sur les features, même si une CI y est verte.

Les migrations ajoutent uniquement des colonnes optionnelles et de nouvelles tables. Aucune donnée historique n'est supprimée. Vérifier l'intégrité, le traitement des comptes existants, les droits croisés club/équipe et les flux E2E avant de transformer la PR en release.
