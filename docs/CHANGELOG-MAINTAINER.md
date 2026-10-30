# Journal des versions et rôle Mainteneur — SEPPYL v0.1

## Fonctionnement

Le menu discret **Nouveautés** du header donne accès à tout l'historique des versions. Une grande fenêtre affiche la liste des versions à gauche et les notes à droite. Pour la version courante `v0.1`, une ouverture automatique se produit lors de la première connexion d'un compte qui ne l'a pas encore vue. Fermer la fenêtre enregistre l'acquittement dans SQLite (`changelog_reads`, clé composée `user_id,version`). Les autres comptes restent indépendants.

Deux versions sont préremplies à la première migration :

- **v0** : périmètre vérifié par rapport à la branche `backup/prod-2026-10-29` (entraînements, présences, notes, activités, matériel, tableur, fiche imprimable, compte).
- **v0.1** : périmètre de la branche actuelle `feat/clubs-roles-editor` (clubs, équipes, droits, audit, tactiques, éditeurs, thèmes, logos).

Les versions sont créées une seule fois : corriger un changelog à travers l'interface ne sera **jamais écrasé** lors d'un futur redémarrage. Les modifications ne réouvrent pas le changelog déjà consulté ; seule une nouvelle version déployée pourra créer un nouvel acquittement obligatoire.

Le texte est conservé comme du **texte**, affiché sans interprétation HTML, et les requêtes d'écriture vérifient systématiquement le rôle courant dans SQLite. Les versions et titres sont bornés en taille.

## Mainteneur : promotion et gestion des rôles

Mainteneur possède tous les droits d'Administrateur, ainsi que le droit de créer et modifier les notes de version. Un **Mainteneur** peut promouvoir un compte en **Mainteneur**, ou modifier le rôle d'un autre Mainteneur depuis **Administration → Utilisateurs**. Un **Administrateur** classique ne peut ni attribuer ce rôle ni le retirer à un autre compte. L'API vérifie ces contraintes en consultant le rôle actuel enregistré dans SQLite à chaque requête ; une interface modifiée ne peut pas contourner les vérifications.

Pour désigner le premier Mainteneur (ou effectuer une reprise d'accès), sauvegarder d'abord la base SQLite, puis exécuter sur le serveur :

```bash
cargo run -p api --bin seppyl-maintainer -- promote NOM_DU_COMPTE --db data/seppyl.local.db
```

L'argument `--db` doit désigner **le fichier SQLite réellement utilisé par le serveur**. Pour un binaire compilé :

```bash
./target/release/seppyl-maintainer promote NOM_DU_COMPTE --db /chemin/vers/seppyl.local.db
```

La commande exige qu'une base et un utilisateur existent déjà. Elle ne crée aucun compte ni nouvelle base et n'accepte pas de nom de rôle transmis en argument.

Pour éditer : se reconnecter avec le compte promu (la lecture du rôle est immédiate côté API), puis **Nouveautés → choisir une version → Modifier → Enregistrer**. L'éditeur riche reprend celui des activités (gras, italique, titres et listes). Les contenus HTML sont nettoyés côté serveur avant stockage. Un Mainteneur peut aussi créer une nouvelle entrée historique. Une nouvelle entrée est **visible par défaut**. Cocher **Masquer cette version** permet de préparer un brouillon : la version est grisée pour les Mainteneurs, inaccessible aux autres comptes et n'est pas annoncée automatiquement. Un bandeau rouge signale cet état dans l'édition. Toute correction d'une version déjà consultée conserve son état de lecture par utilisateur. La version qui s'affiche automatiquement lors d'une mise à jour est déterminée par `CURRENT_VERSION` dans `api/src/routes/miscellaneous/changelog.rs`, à faire évoluer lors d'une nouvelle livraison.

## Tests manuels conseillés

1. Premier login d'un compte : popup v0.1 automatique. Fermer, rafraîchir et se reconnecter : pas de nouvelle popup.
2. Un deuxième compte doit avoir sa propre popup ; les deux comptes peuvent consulter la v0 via le header.
3. Tenter l'édition en Administrateur : elle doit être refusée (HTTP 403). Un Administrateur ne peut ni promouvoir un utilisateur en Mainteneur ni modifier le rôle d'un Mainteneur. Un Mainteneur peut effectuer ces opérations et voit l'option « Mainteneur » dans le sélecteur de rôle.
4. Promouvoir un compte existant par CLI, lui faire modifier une faute de frappe, vérifier que l'édition reste après redémarrage sans afficher à nouveau la popup aux comptes ayant déjà fermé la v0.1.
5. Vérifier le rendu de la fenêtre sur desktop et téléphone, les très longs changelogs et les comportements de fermeture.

6. Entraîneur et Utilisateur : ouvrir « Nouveautés » pour vérifier que v0 et v0.1 sont visibles sans privilège d'édition. Tester particulièrement une connexion après un chargement initial déconnecté : la récupération doit réagir au changement de compte.
7. Depuis un Mainteneur, masquer v0.1 et confirmer que les comptes ordinaires ne la voient plus ; le Mainteneur la voit grisée et dispose d'un avertissement rouge. La rendre de nouveau visible puis contrôler l'annonce aux utilisateurs ne l'ayant pas encore consultée.
8. Modifier le contenu avec titres, gras et listes ; contrôler son rendu après actualisation et vérifier que les balises dangereuses (script, gestionnaires d'événement) sont neutralisées côté API.
