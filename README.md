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
   dx serve --platform web --addr 0.0.0.0 --port 8080
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


## Sessions et future connexion SSO

Le client rétablit la session du navigateur après un rafraîchissement, grâce à un adaptateur local de stockage des jetons (client/src/session.rs). Le jeton local expire après 24 heures. La déconnexion efface le jeton sauvegardé.

Ce mécanisme est temporaire : lors de l'arrivée du SSO Donnarieix, on remplacera le fournisseur d'authentification et on privilégiera un flux OIDC avec des cookies HttpOnly Secure SameSite. Tant que le jeton est conservé dans localStorage, ce prototype ne convient pas à un déploiement public avec des données sensibles.

## Bibliothèque commune

Les onglets Joueuses et Activités permettent d'administrer les données depuis l'application. Les activités comportent une vignette (URL ou téléversement de PNG, JPEG, WebP ou GIF, jusqu'à 4 Mo), une description et du matériel. Une activité peut ensuite être attachée à un entraînement ou ajoutée comme colonne du tableur. Les évaluations restent distinctes par entraînement.

Au démarrage, les activités d'entraînement antérieures à cette modification sont importées de manière idempotente dans la bibliothèque commune ; les notes déjà saisies restent attachées à leur séance. Les anciennes colonnes de l'ancien tableur restent lisibles, sans conversion automatique vers la bibliothèque.

Les images téléversées possèdent des URLs opaques mais sont pour l'instant servies sans authentification. Ne téléversez pas de photos privées ou sensibles avant la mise en place du contrôle d'accès aux médias.

## Responsive et paramètres du compte

L'interface propose désormais une navigation en haut sur ordinateur et une **barre de navigation inférieure** sur téléphone/tablette. Les pages Entraînements, Présences, Notes, Préparation, Joueuses, Activités et Tableur disposent de règles spécifiques pour les petites largeurs. Le tableur conserve un défilement horizontal volontaire plutôt que d'écraser les colonnes. Les formulaires sont adaptés aux écrans tactiles.

La page **Mon compte** est accessible depuis la navigation (route `/settings`) et propose :

- Thème **système**, **clair** ou **sombre** (le mode système suit les préférences de l'appareil)
- Densité d'affichage **confortable** ou **compacte**
- Affichage des présences **grille** ou **liste**
- Modification de l'identifiant de connexion local
- Changement du mot de passe local (mot de passe actuel requis ; nouveau mot de passe d'au moins 12 caractères)

Les préférences sont enregistrées **par compte** dans SQLite (`user_preferences`) et restaurées lors de la connexion, indépendamment du navigateur ou de l'appareil. Mettre à jour un réglage n'écrase pas les autres. Le schéma est synchronisé automatiquement, mais sauvegardez au préalable `data/seppyl.local.db`.

**Migration SSO** : les réglages d'interface resteront associés au compte SEPPYL ; les fonctions de gestion de l'identifiant/mot de passe local devront être remplacées par celles du SSO Donnarieix. Pour cette version, changer le mot de passe ne révoque pas les autres JWT déjà émis (leur durée de vie est de 24 h). Ne pas exposer directement le serveur de développement sur Internet.

Pour la validation manuelle, tester des largeurs proches de **360 px**, **768 px**, **1024 px** et **1440 px**, ainsi que les modes clair, sombre et système, notamment les tableaux, la navigation, les modales et la fiche imprimable.

## Finitions UX (fenêtres, thèmes et tableur)

- Les formulaires s'ouvrent dans des fenêtres plus larges. L'éditeur d'activité peut exploiter deux colonnes sur ordinateur (consignes + image), puis revient à une seule colonne sur téléphone. La zone de rédaction est agrandie et la barre d'actions reste accessible.
- Les paramètres du compte sont répartis en trois onglets : **Préférences**, **Informations** et **Sécurité**.
- L'option **Affichage des présences** est explicitement présentée comme la **vue par défaut**. Un changement reste possible depuis chaque entraînement.
- Les cellules du tableur restent discrètes au repos. Leur fondure/bordure d'édition n'apparaît que lors du survol ou de la saisie. Sur tactile, les cellules vides gardent un léger repère pour signaler qu'elles sont éditables.
- Le changement clair/sombre/système utilise l'API navigateur View Transitions lorsqu'elle est disponible : cercle partant du point de clic et couvrant la fenêtre. En cas de navigateur non compatible ou si l'utilisateur préfère réduire les animations, le changement reste instantané.

Tester en particulier les fenêtres d'activités sur mobile, la saisie de texte long, le passage entre onglets et la transition thème sur Chromium ainsi que son repli sur un navigateur ne prenant pas en charge les transitions.
