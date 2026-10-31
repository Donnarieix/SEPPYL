# Tests E2E et protection de la production SEPPYL

## Contrat de déploiement

`.github/workflows/deploy.yml` déploie **uniquement sur un push ou lancement manuel de `main`**, et seulement si les jobs suivants réussissent :

1. `verify` (Rust API, Rust WASM, tests Rust, tests JS, migrations SQLite et préservation des données synthétiques);
2. `build-api` et `build-web` (artefacts de release);
3. `e2e` (vrai navigateur Chromium, client Dioxus release et API Rust release reliés par un proxy local).

Une PR vers `main` exécute ces vérifications sans livrer. Les suites utilisent une base **jetable et exclusivement synthétique** : aucune connexion à la production n'est effectuée pendant les E2E.

## Pourquoi les E2E sont nécessaires

Un `cargo check` et un `dx build` peuvent réussir alors que le WebAssembly plante à l'exécution (ex. : `Link` Dioxus dans le footer hors du `Router`). Les E2E capturent `pageerror`, `console.error` et les échecs de requêtes locales, et échouent si l'interface ne se charge pas ou si la connexion ne fonctionne pas.

Les parcours actuels couvrent :
- page de connexion, annuaire public et navigation entre les pages légales par le footer ;
- connexion réelle du compte synthétique, rechargement de session, accès aux pages protégées et Administration ;
- affichage mobile des pages publiques ;
- API authentifiée inaccessible sans jeton ; inscription bloquée dans le proxy comme dans le Nginx de production ; santé API.

Les traces, captures, vidéos et rapport HTML Playwright sont téléchargables dans les **Artifacts** de l'exécution GitHub Actions (14 jours).

## Exécution sur GitHub et localement

La CI installe la version fixée de `@playwright/test` et Chromium. Elle construit une version Web spéciale E2E avec `SEPPYL_API_URL=http://127.0.0.1:4173/api`, sans jamais toucher au `main` de production. Le serveur Python de test reproduit le SPA fallback et la règle de proxy Nginx ; le serveur API écoute seulement en boucle locale.

En local, avec les toolchains Rust/Dioxus et Chromium installés : préparer uniquement une base CI jetable avec `CI=true python3 scripts/create-legacy-fixture.py data/sappyl.db` puis `CI=true python3 scripts/prepare-e2e-fixture.py data/sappyl.db`. **Ces scripts refusent toute exécution hors CI et ne doivent jamais être lancés depuis le serveur de production.** Construire `api` en release et `client` en release avec la variable `SEPPYL_API_URL` locale, exporter `SEPPYL_E2E_API_BINARY` et `SEPPYL_E2E_WEB_ROOT`, puis exécuter `bash scripts/run-e2e.sh`.

## Gestion d'incidents et limites

Une CI verte réduit le risque, sans prouver que la base de production, le DNS, Nginx, TLS ou les périphériques des utilisateurs n'ont pas de problème. Les sauvegardes de SQLite et les procédures de retour arrière restent nécessaires. Le retour arrière automatique de l'API ne restaure **pas** le schéma SQLite.

Les comptes Maintainer ne sont jamais créés par les E2E en production : `fixture-coach` est un compte de test dans une copie SQLite synthétique. Le mécanisme d'amorçage temporaire du Maintainer a été supprimé du code et de la CI.

## Bootstrap retiré définitivement

Le compte Maintainer déjà créé est conservé comme un compte normal de la base SQLite. Son identifiant et son mot de passe sont modifiables par son titulaire. Le code ne recrée ni ne promeut aucun compte au démarrage. Le fichier marqueur historique a été traité par la première release de nettoyage ; il n'y a plus ni marqueur ni code d'entretien du marqueur dans le binaire final.

**Action administrative hors du dépôt :** supprimer l'ancien secret GitHub `SEPPYL_BOOTSTRAP_MAINTAINER_PASSWORD_HASH` depuis Settings → Secrets and variables → Actions. Le pipeline ne le consulte plus.

## Versions et GitHub Releases

Chaque livraison versionnée est décrite par une version SemVer commune dans `api/Cargo.toml`, `client/Cargo.toml` et `Cargo.lock`. L'API et le client utilisent cette version pour le changelog courant et le pied de page.

Avant une nouvelle version, préparer une PR avec :
- la version harmonisée dans les deux Cargo.toml et dans Cargo.lock ;
- une entrée `seed` dans `api/src/routes/miscellaneous/changelog.rs`, ajoutée sans écraser les personnalisations existantes ;
- les notes `docs/releases/vX.Y.Z.md`.

La vérification `scripts/release-version.py --check` bloque la CI si ces éléments sont incohérents.

Une PR exécute tous les tests mais ne publie **ni tag, ni release, ni déploiement**. Après fusion dans `main`, le workflow déploie d'abord l'API et le client, vérifie la disponibilité publique, puis crée le tag `vX.Y.Z` et la GitHub Release associée au commit effectivement livré. Les archives API Linux et client Web de cette même CI sont jointes à la release.

Une release existante reste immuable : toute nouvelle version doit incrémenter le numéro. Les déploiements ultérieurs sans changement de version ne créent pas de second tag ni de nouvelle release.
