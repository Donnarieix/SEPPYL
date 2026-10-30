# SEPPYL — Préparation légale et SEO avant mise en ligne

Les pages publiques sont disponibles sans connexion sur `/mentions-legales`, `/confidentialite` et `/cgu`. Le footer commun présente également l'email de support, la version calculée à partir de `client/Cargo.toml` (0.1.0 → v0.1), et le lien « Développé par Donnarieix » pointant vers `https://github.com/Donnarieix`.

## Configuration de publication : obligatoire avant diffusion publique

Les données d'identification légale réelles ne peuvent pas être déduites du code ou du domaine. Les valeurs `À renseigner` du client sont des **indicateurs de configuration manquante**, pas des mentions légales suffisantes. Définir ces variables **dans l'environnement qui compile le client Web** (les `option_env!` sont évalués lors de la compilation Rust/WASM, pas au démarrage du serveur) :

```bash
export SEPPYL_LEGAL_PUBLISHER="Identité ou dénomination juridique de l'éditeur"
export SEPPYL_LEGAL_ADDRESS="Adresse postale de l'éditeur"
export SEPPYL_LEGAL_PHONE="Numéro de téléphone légal de l'éditeur"
export SEPPYL_LEGAL_DIRECTOR="Directeur ou directrice de la publication"
export SEPPYL_LEGAL_HOST="Nom de l'hébergeur réel"
export SEPPYL_LEGAL_HOST_ADDRESS="Adresse postale de l'hébergeur"
export SEPPYL_LEGAL_HOST_PHONE="Téléphone de l'hébergeur"
export SEPPYL_LEGAL_DATA_CONTROLLER="Responsable du traitement et répartition des responsabilités avec les clubs"
export SEPPYL_LEGAL_RETENTION="Durées ou critères déterminés pour les comptes, joueuses, notes, images, journaux et sauvegardes"
```

Vérifier les informations supplémentaires si l'éditeur est une entreprise (statut juridique, immatriculation, TVA selon situation). Pour un hébergement auto-géré, déterminer qui est juridiquement l'hébergeur et quelles coordonnées sont applicables : « auto-hébergé » ne suffit pas à identifier l'interlocuteur légal. Le responsable du traitement des informations sur les joueuses peut différer selon les clubs : le vérifier et prévoir la documentation adéquate. Ne pas publier les données personnelles des joueuses dans l'annuaire public.

Valider la configuration avant un build de production :

```bash
bash scripts/check-production-legal.sh
```

Ce contrôle vérifie uniquement que les champs nécessaires ont été renseignés : **il n'atteste pas de la véracité ni de la conformité juridique** des contenus. Faire relire les mentions par le responsable du service.

## Politique de protection des données

- Inventorier les traitements réellement utilisés (comptes, droits, entraînements, présence, notes, journaux d'audit, fichiers et changelogs). Définir les finalités, les bases légales, les destinataires et les durées de conservation par finalité.
- Vérifier la séparation entre responsable de traitement, clubs et éventuels sous-traitants ; identifier les éventuels transferts hors EEE (services et images hébergées sur des domaines externes, si autorisés).
- Permettre l'exercice effectif des droits (accès, rectification, suppression selon conditions) via `seppyl@donnarieix.fr`, et établir un processus de réponse et d'effacement.
- Informer les joueuses, **y compris mineures et représentants légaux**, lors de la collecte par le club, et limiter les personnes pouvant accéder aux données.
- Arrêter une politique de sauvegarde, journalisation, sécurité, suppression/archivage et gestion des incidents ; faire vérifier le HTTPS de bout en bout et les restrictions d'accès serveur.
- L'application actuelle conserve un jeton JWT dans le **localStorage** du navigateur (voir `client/src/session.rs`) : ce n'est pas un cookie HttpOnly. Une revue de sécurité sur le stockage des sessions et les attaques XSS est requise avant de décrire l'authentification comme sécurisée par cookie.
- Aucun traceur marketing n'est ajouté par cette livraison. Si de futurs traceurs facultatifs apparaissent, revoir l'information et mettre en place un bandeau de consentement uniquement si requis.

## SEO et indexation

`client/public/robots.txt`, `client/public/sitemap.xml` et `client/public/site.webmanifest` sont automatiquement copiés dans le dossier `public/` du build par **Dioxus 0.7**. Après déploiement du build Web :

```bash
curl -I https://seppyl.donnarieix.fr/robots.txt
curl -I https://seppyl.donnarieix.fr/sitemap.xml
curl -I https://seppyl.donnarieix.fr/mentions-legales
curl -I https://seppyl.donnarieix.fr/confidentialite
curl -I https://seppyl.donnarieix.fr/cgu
```

Les pages privées émettent une métadonnée `noindex` côté client et le robots.txt décourage leur exploration. Pour une protection effective de l'indexation des URL privées, ajouter également l'entête HTTP `X-Robots-Tag: noindex, nofollow` côté reverse proxy sur les chemins privés. **L'authentification API reste la seule vraie protection de données privées** ; robots.txt n'en est pas une.

SEPPYL est actuellement une **SPA Dioxus**. Le contenu de l'annuaire public est chargé côté client ; les robots de recherche qui n'exécutent pas JavaScript risquent de ne voir que la coquille HTML. Pour un référencement approfondi de chaque club, prévoir du SSR/SSG ou des pages publiques rendues côté serveur. Le sitemap initial ne répertorie que les pages publiques fixes (pas de liens directs vers les fiches dynamiques).

## Contact

Pour la première mise en ligne, le footer utilise `mailto:seppyl@donnarieix.fr` sans ticketing. Cette approche évite une nouvelle collecte et un stockage applicatif de correspondances personnelles. Vérifier que la boîte reçoit correctement les messages et qu'une personne est chargée de répondre aux demandes de droit d'accès ou d'effacement.

## Points à valider manuellement

1. Footer sur `/login`, `/public`, les pages légales et toutes les pages authentifiées ; comportement mobile, mode contrasté et impression (footer masqué à l'impression).
2. Vérifier la page Mentions légales avec **les véritables informations d'éditeur/hébergeur** : aucun texte « À renseigner » ne doit être encore publié.
3. Contrôler les liens externes GitHub et contact email, puis vérifier la version `v0.1`.
4. Vérifier que les URL privées ne sont pas indexables et que les pages légales sont accessibles sans compte, y compris après actualisation directe de l'URL dans Nginx (`try_files` SPA).
5. Envoyer un email test à `seppyl@donnarieix.fr`.

Sources officielles : Ministère de l'Économie (mentions légales) https://www.economie.gouv.fr/entreprises/developper-son-entreprise/innover-et-numeriser-son-entreprise/mentions-sur-votre-site-internet-les-obligations-respecter ; CNIL (transparence) https://www.cnil.fr/fr/conformite-rgpd-information-des-personnes-et-transparence ; CNIL (sport amateur) https://www.cnil.fr/fr/sport-amateur-hors-contrat/questions-reponses .
