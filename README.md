# SEPPYL — suivi des entraînements

Cette branche de développement propose deux utilisations :

- **Sur le terrain** : ouvrir un entraînement, cocher les joueuses présentes, puis saisir les notes générales de **0 à 5**, ou les notes **par activité**.
- **À la maison** : créer un entraînement daté, décrire des activités, leur associer un lien d'image et des quantités de matériel, puis ouvrir une fiche imprimable (navigateur : **Ctrl+P → Enregistrer au format PDF**) utilisable hors connexion.

Le tableur historique est conservé dans le menu **Tableur**, mais la nouvelle organisation distingue les **entraînements** des **activités**. Les anciennes activités du tableur ne sont pas automatiquement converties en entraînements.

## Démarrer en local

Depuis la racine du dépôt :

\`\`\`powershell
cargo run -p api
\`\`\`

Dans un autre terminal :

\`\`\`powershell
cd client
dx serve --platform web
\`\`\`

L'API écoute sur \`3000\`, le client utilise par défaut \`http://127.0.0.1:3000\`.

**Important** : l'API synchronise le schéma de la base SQLite au démarrage. **Faire une copie de \`data/sappyl.db\` avant le premier démarrage de cette branche**. Les nouvelles entités doivent créer de nouvelles tables ; aucune conversion automatique des anciennes données n'est réalisée.

## Tester sur un téléphone, même réseau local (développement uniquement)

\`127.0.0.1\` sur le téléphone désigne le téléphone, pas l'ordinateur. Il faut utiliser l'IP locale de l'ordinateur et rendre les serveurs accessibles depuis le LAN.

Exemple avec une IP de PC **fictive** \`192.168.1.50\` et un client servi sur le port **8080** :

1. Dans un terminal PowerShell de l'API, renseigner la liste explicite des origines autorisées (adapter l'adresse et le port) :
   \`\`\`powershell
   $env:SEPPYL_ALLOWED_ORIGINS="http://192.168.1.50:8080"
   cargo run -p api
   \`\`\`
2. Dans un autre terminal, définir l'URL d'API **avant de compiler le client** :
   \`\`\`powershell
   $env:SEPPYL_API_URL="http://192.168.1.50:3000"
   cd client
   dx serve --platform web --host 0.0.0.0 --port 8080
   \`\`\`
3. Vérifier l'adresse effectivement indiquée par Dioxus et adapter les commandes à la version de votre CLI. Ouvrir \`http://192.168.1.50:8080\` sur le téléphone. L'accès dépend également du pare-feu Windows.

**Sécurité :** ce mode HTTP local transmet les mots de passe et les jetons d'authentification sans chiffrement. À réserver à un Wi-Fi de test de confiance, avec des comptes de test, jamais à un accès Internet public. Avant une utilisation réelle à distance, il faudra servir le frontend et l'API derrière HTTPS, sécuriser les secrets et revoir les autorisations.

## Comptes

L'écran de connexion permet de créer un compte. On peut créer par exemple un compte pour Valentin et un autre pour l'entraîneur, **sans choisir ici leurs mots de passe**.

**Limitation actuelle :** les joueuses et entraînements sont rattachés à un seul compte. Les deux entraîneurs ne partagent pas encore une équipe ; cela nécessitera un espace d'équipe et des droits d'accès explicites.

## Périmètre et évolutions

Réalisé pour cette première itération :

- Création et liste d'entraînements (nom, date)
- Liste des joueuses et validation des présences
- Notes générales et par activité, limitées à 0–5 côté API
- Création d'activités avec description et URL d'image
- Matériel par activité et total de la séance
- Fiche d'entraînement mise en page pour impression/PDF hors ligne
- Retour possible au tableur précédent

À prévoir :

- Téléversement d'images depuis le PC/téléphone (actuellement seulement URL)
- Duplication d'entraînements, bibliothèque d'activités et mots-clés
- Partage de l'équipe et des séances entre entraîneurs
- Édition avancée, droits et déploiement HTTPS hors réseau local
- Véritable mode hors ligne *dans* l'application (la fiche PDF est hors ligne, la saisie ne l'est pas)
